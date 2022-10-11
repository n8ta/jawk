mod scopes;
// mod runtime;
// mod subroutines;

pub use value::{ValuePtrT, ValueT};

mod subroutines;
mod value;
mod helpers;

use crate::codgen::scopes::Scopes;
use crate::codgen::subroutines::Subroutines;
use crate::lexer::{BinOp, LogicalOp, MathOp};
use crate::parser::{ArgT, Program, ScalarType, Stmt, TypedExpr};
use crate::printable_error::PrintableError;
use crate::runtime::{LiveRuntime, Runtime, TestRuntime};
use crate::{AnalysisResults, Expr, Symbolizer};
use gnu_libjit::{Abi, Context, Label, Value};
use std::collections::{HashMap, HashSet};
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::rc::Rc;
use crate::parser::Stmt::Print;
use crate::symbolizer::Symbol;
use crate::typing::{TypedProgram, TypedFunc};

/// ValueT is the jit values that make up a struct. It's not a tagged union
/// just a struct with only one other field being valid to read at a time based on the tag field.
///
/// ValueT {
///     tag: u8
///     float: f64
///     string: *mut c_void
/// }


pub const FLOAT_TAG: i8 = 0;
pub const STRING_TAG: i8 = 1;

// Entry point to run a program
pub fn compile_and_run(prog: Program, files: &[String], symbolizer: &mut Symbolizer) -> Result<(), PrintableError> {
    let mut runtime = LiveRuntime::new(files.to_vec());
    let mut codegen = CodeGen::new(&mut runtime, symbolizer);
    codegen.compile(prog, false)?;
    codegen.run();
    Ok(())
}

// Entry point to run and debug/test a program. Use the test runtime.
pub fn compile_and_capture(prog: Program, files: &[String], symbolizer: &mut Symbolizer) -> Result<TestRuntime, PrintableError> {
    let mut test_runtime = TestRuntime::new(files.to_vec());
    {
        let mut codegen = CodeGen::new(&mut test_runtime, symbolizer);
        codegen.compile(prog, true)?;
        codegen.run();
    }
    assert_eq!(test_runtime.strings_in(), test_runtime.strings_out(), "LEFT strings in does not match RIGHT strings out");
    Ok(test_runtime)
}

struct CodeGen<'a, RuntimeT: Runtime> {
    // Core stuff
    pub(crate) function: gnu_libjit::Function,
    scopes: Scopes,
    // Stores the points to each global variable in the program
    pub(crate) context: Context,
    // The jit context
    runtime: &'a mut RuntimeT,
    // Runtime provides native functions and may be used for debugging.
    symbolizer: &'a mut Symbolizer,

    array_map: HashMap<String, i32>, // Map each array name to its unique identifier

    // Used for commonly reused snippets like string-truthyness etc.
    subroutines: Subroutines,

    // These are local variables that we use as scratch space.
    binop_scratch: ValuePtrT,
    // just an int (generally 0 or 1 used for insn_eq ne comparisons) stored as a local.
    binop_scratch_int: Value,

    // Stack space to use for passing multiple return values from the runtime.
    ptr_scratch: ValuePtrT,

    // Var arg scratch for passing a variable # of printf args (max 64) allocated on the heap
    var_arg_scratch: Value,

    // Used to init the pointer section of the value struct when it's undefined. Should never be dereferenced.
    zero_ptr: Value,
    // Used to init the float section of value. Safe to use but using it is a bug.
    zero_f: Value,

    // To avoid creating tons of constants just reuse the tags here
    float_tag: Value,
    string_tag: Value,

    // Where a 'break' keyword should jump
    break_lbl: Vec<Label>,
    // Where a return should jump after storing return value
    return_lbl: Option<Label>,

    function_map: HashMap<Symbol, gnu_libjit::Function>,

}

impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    fn new(runtime: &'a mut RuntimeT, symbolizer: &'a mut Symbolizer) -> Self {
        let mut context = Context::new();
        let mut function = context
            .function(Abi::Cdecl, &Context::float64_type(), vec![])
            .expect("to create function");

        let zero_ptr = Box::into_raw(Box::new("".to_string())) as *mut c_void;
        let zero_ptr = function.create_void_ptr_constant(zero_ptr);
        let zero_f = function.create_float64_constant(0.0);
        let float_tag = function.create_sbyte_constant(FLOAT_TAG as c_char);
        let string_tag = function.create_sbyte_constant(STRING_TAG as c_char);

        // Leak some memory to use as scratch space for passing values between the jit and the runtime.
        let tag_scratch = function.create_void_ptr_constant((Box::leak(Box::new(FLOAT_TAG)) as *mut i8) as *mut c_void);
        let float_scratch = function.create_void_ptr_constant(Box::leak(Box::new(0.0 as f64)) as *mut f64 as *mut c_void);
        let zero = Box::leak(Box::new(0)) as *mut i32;
        let ptr_scratch = function.create_void_ptr_constant(Box::leak(Box::new(zero)) as (*mut (*mut i32)) as *mut c_void);
        let ptr_scratch = ValuePtrT::new(tag_scratch, float_scratch, ptr_scratch);

        let binop_scratch_int = function.create_value_int();
        let binop_scratch = ValueT::new(
            function.create_value_int(),
            function.create_value_float64(),
            function.create_value_void_ptr(),
        );


        let var_arg_scratch = unsafe { libc::malloc(100 * 8) };
        let var_arg_scratch = function.create_void_ptr_constant(var_arg_scratch);

        let subroutines = Subroutines::new(&mut context, runtime);
        let mut function_map = HashMap::new();
        function_map.insert(symbolizer.get("main function"), function.clone());
        let codegen = CodeGen {
            var_arg_scratch,
            array_map: HashMap::new(),
            function,
            scopes: Scopes::new(),
            context,
            runtime,
            subroutines,
            binop_scratch,
            ptr_scratch,
            binop_scratch_int,
            zero_ptr,
            zero_f,
            float_tag,
            string_tag,
            break_lbl: vec![],
            function_map,
            return_lbl: None,
            symbolizer,
        };
        codegen
    }

    fn run(&mut self) {
        let function: extern "C" fn() = self.function.to_closure();
        function();
    }

    fn stub_all_functions(&mut self, prog: &Program) -> Result<(), PrintableError> {
        for (name, parser_func) in &prog.functions {
            if name.to_str() == "main function" {
                continue;
            }
            let mut params = vec![];
            for arg in &parser_func.args {
                let arg_typ = if let Some(arg_typ) = arg.typ { arg_typ } else { continue; };
                match arg_typ {
                    ArgT::Scalar => {
                        params.push(Context::sbyte_type());
                        params.push(Context::float64_type());
                        params.push(Context::void_ptr_type());
                    }
                    ArgT::Array => {
                        params.push(Context::int_type());
                    }
                }
            }

            let mut function = self.context.function(Abi::Cdecl, &Context::void_ptr_type(), params).unwrap();
            self.function_map.insert(name.clone(), function);
        }
        Ok(())
    }

    fn compile(&mut self, prog: Program, dump: bool) -> Result<(), PrintableError> {
        let zero = self.function.create_float64_constant(0.0);
        self.define_all_globals(&prog)?;

        // Since functions are can be recursive or mutually recursive we need to
        // declare them all before we build compile bodies.
        self.stub_all_functions(&prog)?;

        let main = self.symbolizer.get("main function");
        for (name, func) in prog.functions {
            let main = name == main;
            self.compile_function(&name, &func,
                                  main,
                                  &prog.global_analysis,
                                  dump)?;
        }

        let main = self.function_map.get(&self.symbolizer.get("main function")).unwrap();
        self.function.insn_call(main, vec![]);

        self.function.insn_return(&zero);
        self.context.build_end();
        self.function.compile();
        Ok(())
    }

    fn compile_function(&mut self, name: &Symbol,
                        parser_func: &crate::parser::Function,
                        main: bool,
                        global_analysis: &AnalysisResults,
                        dump: bool) -> Result<(), PrintableError> {
        self.scopes.open_scope();

        let mut function = self.function_map.get_mut(&name).unwrap().clone();

        self.return_lbl = Some(Label::new());
        let mut arg_idx = 0;
        for arg in &parser_func.args {
            let arg_typ = if let Some(arg_typ) = arg.typ { arg_typ } else { continue; };
            match arg_typ {
                ArgT::Scalar => {
                    let mut scalar = self.new_stack_value();
                    self.scopes.insert_scalar(arg.name.clone(), scalar.clone())?;
                    let tag = self.function.arg(arg_idx).unwrap();
                    let float = self.function.arg(arg_idx + 1).unwrap();
                    let ptr = self.function.arg(arg_idx + 2).unwrap();
                    let value_from_args = ValueT::new(tag, float, ptr);
                    // Load args into normal stack value
                    // TODO: Can we use the args as our storage instead of a separate stack allocation?
                    // And thus avoid this pointless store
                    self.store(&mut scalar, &value_from_args);
                    arg_idx += 3;
                }
                ArgT::Array => {
                    let array = self.function.arg(arg_idx).unwrap();
                    self.scopes.insert_array(arg.name.clone(), array)?;
                    arg_idx += 1;
                }
            }
        }

        std::mem::swap(&mut self.function, &mut function);

        self.compile_stmt(&parser_func.body)?;

        self.function.insn_label(&mut self.return_lbl.clone().unwrap());
        self.return_lbl = None;

        // Drop all strings we may have stored in function locals
        let mut scope = self.scopes.close_scope();
        for x in scope.scalars.iter_mut() {
            self.drop_if_string_ptr(x.1, ScalarType::Variable);
        }

        let zero = self.function.create_float64_constant(0.0);

        if main {
            for var in global_analysis.global_scalars.clone() {
                let mut var_ptrs = self.scopes.get_scalar(&var)?.clone();
                self.drop_if_string_ptr(&mut var_ptrs, ScalarType::Variable);
            }
        }
        self.function.insn_return(&zero);

        std::mem::swap(&mut self.function, &mut function);
        if dump {
            println!("{}", function.dump().unwrap());
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                let ret_val = if let Some(ret) = ret {
                    self.compile_expr(ret)?
                } else {
                    ValueT::new(self.float_tag.clone(), self.zero_f.clone(), self.zero_ptr.clone())
                };
                self.store(&mut self.binop_scratch.clone(), &ret_val);
                self.function.insn_branch(&mut self.return_lbl.clone().unwrap())
            }
            Stmt::Printf { args, fstring } => {
                let fstring_val = self.compile_expr(fstring)?;
                let fstring_ptr = self.val_to_string(&fstring_val, fstring.typ);
                // write all the values into scratch space. Runtime will read from that pointer
                for (idx, arg) in args.iter().enumerate() {
                    let compiled = self.compile_expr(arg)?;
                    self.function.insn_store_relative(&self.var_arg_scratch, (idx * 24) as c_long, &compiled.tag);
                    self.function.insn_store_relative(&self.var_arg_scratch, (idx * 24 + 8) as c_long, &compiled.float);
                    self.function.insn_store_relative(&self.var_arg_scratch, (idx * 24 + 16) as c_long, &compiled.pointer);
                }
                let nargs = self.function.create_int_constant(args.len() as c_int);
                self.runtime.printf(&mut self.function, fstring_ptr, nargs, self.var_arg_scratch.clone());
            }
            Stmt::Break => {
                if let Some(lbl) = self.break_lbl.last_mut() {
                    self.function.insn_branch(lbl)
                } else {
                    return Err(PrintableError::new("Found break keyword outside of a loop"));
                }
            }
            Stmt::Expr(expr) => {
                let res = self.compile_expr(expr)?;
                self.drop_if_str(&res, expr.typ);
            }
            Stmt::Print(expr) => {
                let val = self.compile_expr(expr)?;
                // Optimize print based on static knowledge of type
                match expr.typ {
                    ScalarType::String => {
                        self.runtime.print_string(&mut self.function, val.pointer.clone());
                        self.drop(&val.pointer);
                    }
                    ScalarType::Float => {
                        self.runtime.print_float(&mut self.function, val.float);
                    }
                    ScalarType::Variable => {
                        let str = self.val_to_string(&val, expr.typ);
                        self.runtime.print_string(&mut self.function, str.clone());
                        self.runtime.free_string(&mut self.function, str);
                    }
                }
            }
            Stmt::Group(group) => {
                for group in group {
                    self.compile_stmt(group)?
                }
            }
            Stmt::If(test, if_so, if_not) => {
                if let Some(if_not) = if_not {
                    let test_value = self.compile_expr(test)?;
                    let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                    self.drop_if_str(&test_value, test.typ);
                    let mut then_lbl = Label::new();
                    let mut done_lbl = Label::new();
                    self.function.insn_branch_if(&bool_value, &mut then_lbl);
                    self.compile_stmt(if_not);
                    self.function.insn_branch(&mut done_lbl);
                    self.function.insn_label(&mut then_lbl);
                    self.compile_stmt(if_so);
                    self.function.insn_label(&mut done_lbl);
                } else {
                    let test_value = self.compile_expr(test)?;
                    let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                    self.drop_if_str(&test_value, test.typ);
                    let mut done_lbl = Label::new();
                    self.function.insn_branch_if_not(&bool_value, &mut done_lbl);
                    self.compile_stmt(if_so);
                    self.function.insn_label(&mut done_lbl);
                }
            }
            Stmt::While(test, body) => {
                let mut test_label = Label::new();
                let mut done_label = Label::new();
                self.break_lbl.push(done_label.clone());
                self.function.insn_label(&mut test_label);
                let test_value = self.compile_expr(test)?;
                let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                self.drop_if_str(&test_value, test.typ);
                self.function.insn_branch_if_not(&bool_value, &mut done_label);
                self.compile_stmt(body)?;
                self.function.insn_branch(&mut test_label);
                self.function.insn_label(&mut done_label);
                self.break_lbl.pop().unwrap();
            }
        }
        Ok(())
    }

    // When compile_expr returns a string the caller is responsible for freeing it
    fn compile_expr(&mut self, expr: &TypedExpr) -> Result<ValueT, PrintableError> {
        Ok(match &expr.expr {
            Expr::Call { target, args } => {
                todo!()
            }
            Expr::ScalarAssign(var, value) => {
                // BEGIN: Optimization
                // Optimization to allow reusing the string being assigned to by a string concat operation
                // a = "init"
                // a = a "abc" (We don't want to make a copy of a when we concat "abc" with it)
                // We first calculfate a to be init and "abc" to "abc". This results in a copy being made
                // of "init" (increasing the reference count to 2). Then we drop a BEFORE actually doing the
                // concat.  Reference count returns to 1.
                // Now concat can re-use the original value since ref count is 1 it's safe to downgrade
                // from Rc -> Box

                if let Expr::Concatenation(vars) = &value.expr {
                    let mut var_ptrs = self.scopes.get_scalar(var)?.clone();
                    let strings_to_concat = self.compile_exprs_to_string(vars)?;
                    let old_value = self.load(&mut var_ptrs);
                    self.drop_if_str(&old_value, ScalarType::Variable);
                    let new_value = self.concat_values(&strings_to_concat);
                    self.store(&mut var_ptrs, &new_value);
                    return Ok(self.copy_if_string(new_value, ScalarType::Variable));
                }
                let new_value = self.compile_expr(value)?;
                let mut var_ptrs = self.scopes.get_scalar(var)?.clone();
                let old_value = self.load(&mut var_ptrs);
                self.drop_if_str(&old_value, ScalarType::Variable);
                self.store(&mut var_ptrs, &new_value);
                self.copy_if_string(new_value, value.typ)
            }
            Expr::NumberF64(num) => ValueT::new(
                self.float_tag(),
                self.function.create_float64_constant(*num),
                self.zero_ptr.clone(),
            ),
            Expr::String(str) => {
                // Every string constant is stored in a variable with the name " name"
                // the space ensures we don't collide with normal variable names;
                let string_tag = self.string_tag();
                let space_before = format!(" {}", str);
                let mut var_ptr = self.scopes.get_scalar(&self.symbolizer.get(space_before))?.clone();
                let var = self.load(&mut var_ptr);
                let zero = self.function.create_float64_constant(0.0);
                let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                ValueT::new(string_tag, zero, new_ptr)
            }
            Expr::Regex(str) => {
                // Every string constant is stored in a variable with the name " name"
                // the space ensures we don't collide with normal variable names;
                let string_tag = self.string_tag();
                let mut var_ptr = self.scopes.get_scalar(&self.symbolizer.get(format!(" {}", str)))?.clone();
                let var = self.load(&mut var_ptr);
                let zero = self.function.create_float64_constant(0.0);
                let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                ValueT::new(string_tag, zero, new_ptr)
            }
            Expr::MathOp(left_expr, op, right_expr) => {
                // Convert left and right to floats if needed and perform the MathOp
                let mut left = self.compile_expr(left_expr)?;
                let mut right = self.compile_expr(right_expr)?;
                let left_float = self.val_to_float(&left, left_expr.typ);
                let right_float = self.val_to_float(&right, right_expr.typ);
                let result = match op {
                    MathOp::Minus => self.function.insn_sub(&left_float, &right_float),
                    MathOp::Plus => self.function.insn_add(&left_float, &right_float),
                    MathOp::Slash => self.function.insn_div(&left_float, &right_float),
                    MathOp::Star => self.function.insn_mult(&left_float, &right_float),
                    MathOp::Modulus => self.function.insn_rem(&left_float, &right_float),
                    MathOp::Exponent => self.function.insn_pow(&left_float, &right_float),
                };
                self.drop_if_str(&left, left_expr.typ);
                self.drop_if_str(&right, right_expr.typ);

                let zero = self.float_tag();
                ValueT::new(zero, result, self.zero_ptr.clone())
            }
            Expr::BinOp(left_expr, op, right_expr) => {
                let left = self.compile_expr(left_expr)?;
                let right = self.compile_expr(right_expr)?;
                let tag = self.float_tag();

                // Optimize the case where we know both are floats
                if left_expr.typ == ScalarType::Float && right_expr.typ == ScalarType::Float {
                    return Ok(ValueT::new(tag, self.float_binop(&left.float, &right.float, *op), self.zero_ptr.clone()));
                }

                let left_is_float = self.function.insn_eq(&tag, &left.tag);
                let right_is_float = self.function.insn_eq(&tag, &right.tag);
                let mut both_float_lbl = Label::new();
                let mut done_lbl = Label::new();
                let both_float = self.function.insn_and(&left_is_float, &right_is_float);
                self.function
                    .insn_branch_if(&both_float, &mut both_float_lbl);

                // String/Float Float/String String/String case
                let left_as_string = self.val_to_string(&left, left_expr.typ);
                let right_as_string = self.val_to_string(&right, right_expr.typ);
                let res = self.runtime.binop(&mut self.function, left_as_string.clone(), right_as_string.clone(), *op);
                let result = ValueT::new(self.float_tag.clone(), res, self.zero_ptr.clone());
                self.store(&mut self.binop_scratch.clone(), &result);
                self.drop(&left_as_string);
                self.drop(&right_as_string);
                self.function.insn_branch(&mut done_lbl);

                // Float/Float case
                self.function.insn_label(&mut both_float_lbl);
                let float_val = self.float_binop(&left.float, &right.float, *op);
                let value = ValueT::new(tag, float_val, self.zero_ptr.clone());
                self.store(&mut self.binop_scratch.clone(), &value);

                // Done load the result from scratch
                self.function.insn_label(&mut done_lbl);
                self.load(&mut self.binop_scratch.clone())
            }
            Expr::LogicalOp(left, op, right) => {
                let float_1 = self.function.create_float64_constant(1.0);
                let float_0 = self.function.create_float64_constant(0.0);
                // Short circuiting and and or operators.
                // Gotta be careful to free values appropriately and only when they are actually created.
                let res = match op {
                    LogicalOp::And => {
                        let mut ret_false = Label::new();
                        let mut done = Label::new();
                        let left_val = self.compile_expr(left)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(&left_val, left.typ);
                        self.function.insn_branch_if_not(&l, &mut ret_false);
                        let right_val = self.compile_expr(right)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(&right_val, right.typ);
                        self.function.insn_branch_if_not(&r, &mut ret_false);
                        self.function.insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut ret_false);
                        self.function.insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.function.insn_load(&self.binop_scratch.float);
                        ValueT::new(tag, result_f, self.zero_ptr.clone())
                    }
                    LogicalOp::Or => {
                        let mut done = Label::new();
                        let mut return_true = Label::new();
                        let left_val = self.compile_expr(left)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(&left_val, left.typ);
                        self.function.insn_branch_if(&l, &mut return_true);
                        let right_val = self.compile_expr(right)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(&right_val, right.typ);
                        self.function.insn_branch_if(&r, &mut return_true);
                        self.function.insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut return_true);
                        self.function.insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.function.insn_load(&self.binop_scratch.float);
                        ValueT::new(tag, result_f, self.zero_ptr.clone())
                    }
                };
                res
            }
            Expr::Variable(var) => {
                // compile_expr returns a string/float that is 'owned' by the caller.
                // If it's a string we need to call copy_string to update the reference count.
                // If it's a float no-op.
                // If type is unknown we check tag then copy_string if needed.
                let mut var_ptr = self.scopes.get_scalar(var)?.clone();
                let string_tag = self.string_tag();
                match expr.typ {
                    ScalarType::String => {
                        let var = self.load(&mut var_ptr);
                        let zero = self.function.create_float64_constant(0.0);
                        let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                        ValueT::new(string_tag, zero, new_ptr)
                    }
                    ScalarType::Variable => {
                        // If it's a string variable copy it and store that pointer in self.binop_scratch.pointer
                        // otherwise store zero self.binop_scratch.pointer. After this load self.binop_scratch.pointer
                        // and make a new value with the old tag/float + new string pointer.
                        let var = self.load(&mut var_ptr);
                        let is_not_str = self.function.insn_eq(&string_tag, &var.tag);
                        let mut done_lbl = Label::new();
                        let mut is_not_str_lbl = Label::new();
                        self.function.insn_branch_if_not(&is_not_str, &mut is_not_str_lbl);
                        let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                        self.function.insn_store(&self.binop_scratch.pointer, &new_ptr);
                        self.function.insn_branch(&mut done_lbl);

                        self.function.insn_label(&mut is_not_str_lbl);
                        self.function.insn_store(&self.binop_scratch.pointer, &self.zero_ptr);

                        self.function.insn_label(&mut done_lbl);
                        let str_ptr = self.function.insn_load(&self.binop_scratch.pointer);
                        ValueT::new(var.tag, var.float, str_ptr)
                    }
                    ScalarType::Float => self.load(&mut var_ptr),
                }
            }
            Expr::Column(col) => {
                let column = self.compile_expr(col)?;
                let val = self.runtime.column(
                    &mut self.function,
                    column.tag.clone(),
                    column.float.clone(),
                    column.pointer.clone(),
                );
                let tag = self.string_tag();
                self.drop_if_str(&column, col.typ);
                ValueT::new(tag, self.function.create_float64_constant(0.0), val)
            }
            Expr::NextLine => {
                // Ask runtime if there is a next line. Returns a float 0 or 1
                let one = self.float_tag();
                let next_line_exists = self.runtime.call_next_line(&mut self.function);
                ValueT::new(one, next_line_exists, self.zero_ptr.clone())
            }
            Expr::Concatenation(vars) => {
                // Eg: a = "a" "b" "c"
                let compiled = self.compile_exprs_to_string(vars)?;
                self.concat_values(&compiled)
            }
            Expr::Ternary(cond, expr1, expr2) => {
                let mut done_lbl = Label::new();
                let mut truthy_lbl = Label::new();

                let result = self.compile_expr(cond)?;
                let bool_value = self.truthy_ret_integer(&result, cond.typ);

                self.function.insn_branch_if(&bool_value, &mut truthy_lbl);

                let falsy_result = self.compile_expr(expr2)?;
                self.store(&mut self.binop_scratch.clone(), &falsy_result);
                self.function.insn_branch(&mut done_lbl);

                self.function.insn_label(&mut truthy_lbl);

                let truthy_result = self.compile_expr(expr1)?;
                self.store(&mut self.binop_scratch.clone(), &truthy_result);

                self.function.insn_label(&mut done_lbl);

                self.load(&mut self.binop_scratch.clone())
            }
            Expr::ArrayIndex { name, indices } => {
                let values = self.compile_exprs_to_string(indices)?;
                let indices = self.concat_indices(&values);
                let array_id = self.scopes.get_array(name).unwrap().clone();
                // Runtime will set the out_tag out_float and out_ptr pointers to a new value. Just load em
                self.runtime.array_access(&mut self.function, array_id, indices, self.ptr_scratch.tag.clone(), self.ptr_scratch.float.clone(), self.ptr_scratch.pointer.clone());
                let tag = self.function.insn_load_relative(&self.ptr_scratch.tag, 0, &Context::sbyte_type());
                let float = self.function.insn_load_relative(&self.ptr_scratch.float, 0, &Context::float64_type());
                let pointer = self.function.insn_load_relative(&self.ptr_scratch.pointer, 0, &Context::void_ptr_type());
                ValueT::new(tag, float, pointer)
            }
            Expr::InArray { name, indices } => {
                let values = self.compile_exprs_to_string(indices)?;
                let value = self.concat_indices(&values);
                let array_id = self.scopes.get_array(name).unwrap().clone();
                let float_result = self.runtime.in_array(&mut self.function, array_id, value);
                ValueT::new(self.float_tag(), float_result, self.zero_ptr.clone())
            }
            Expr::ArrayAssign { name, indices, value } => {
                let rhs = self.compile_expr(value)?;
                let values = self.compile_exprs_to_string(indices)?;
                let indices = self.concat_indices(&values);
                let array_id = self.scopes.get_array(name).unwrap().clone();
                let result_copy = self.copy_if_string(rhs.clone(), value.typ);
                self.runtime.array_assign(&mut self.function, array_id, indices, rhs.tag, rhs.float, rhs.pointer);
                result_copy
            }
        })
    }
}

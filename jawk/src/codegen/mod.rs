pub use value::{ValuePtrT, ValueT};

mod value;
mod helpers;
mod globals;
mod codegen_consts;

use crate::parser::{Program, ScalarType, Stmt, TypedExpr};
use crate::lexer::{LogicalOp, MathOp};
use crate::printable_error::PrintableError;
use crate::runtime::{LiveRuntime, Runtime, TestRuntime};
use crate::{AnalysisResults, Expr, Symbolizer};
use gnu_libjit::{Abi, Context, Label, Value};
use std::os::raw::{c_char, c_int, c_long, c_void};
use crate::codegen::codegen_consts::CodegenConsts;
use crate::codegen::globals::Globals;

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
    codegen.compile(prog, false, false)?;
    codegen.run();
    Ok(())
}

// Entry point to run and debug/test a program. Use the test runtime.
pub fn compile_and_capture(prog: Program, files: &[String], symbolizer: &mut Symbolizer, dump: bool) -> Result<TestRuntime, PrintableError> {
    let mut test_runtime = TestRuntime::new(files.to_vec());
    {
        let mut codegen = CodeGen::new(&mut test_runtime, symbolizer);
        codegen.compile(prog, true, dump)?;
        codegen.run();
    }
    println!("Strings in: {}", test_runtime.strings_in());
    println!("Strings out: {}", test_runtime.strings_out());
    assert_eq!(test_runtime.strings_in(), test_runtime.strings_out(), "LEFT strings in does not match RIGHT strings out. This program caused a memory leak.");
    Ok(test_runtime)
}

struct CodeGen<'a, RuntimeT: Runtime> {
    // Core stuff
    pub(crate) function: gnu_libjit::Function,
    // Stores the points to each global variable in the program
    pub(crate) context: Context,
    // The jit context
    runtime: &'a mut RuntimeT,
    // Runtime provides native functions and may be used for debugging.
    symbolizer: &'a mut Symbolizer,

    // These are local variables that we use as scratch space.
    binop_scratch: ValuePtrT,

    // Stack space to use for passing multiple return values from the runtime.
    ptr_scratch: ValuePtrT,

    // Var arg scratch for passing a variable # of printf args (max 64) allocated on the heap
    var_arg_scratch: Value,

    c: crate::codegen::codegen_consts::CodegenConsts,

    // Where a 'break' keyword should jump
    break_lbl: Vec<Label>,
    // Where a return should jump after storing return value
    return_lbl: Option<Label>,

    globals: Globals,
}

impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    fn new(runtime: &'a mut RuntimeT, symbolizer: &'a mut Symbolizer) -> Self {
        let mut context = Context::new();
        let mut function = context
            .function(Abi::Cdecl, &Context::int_type(), vec![])
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
        let ptr_scratch = function.create_void_ptr_constant(Box::leak(Box::new(zero)) as *mut *mut i32 as *mut c_void);
        let ptr_scratch = ValuePtrT::var(tag_scratch, float_scratch, ptr_scratch);

        let binop_scratch = ValueT::var(
            function.create_value_int(),
            function.create_value_float64(),
            function.create_value_void_ptr(),
        );

        let var_arg_scratch = unsafe { libc::malloc(100 * 8) };
        let var_arg_scratch = function.create_void_ptr_constant(var_arg_scratch);

        let globals = Globals::new(AnalysisResults::new(), runtime, &mut function, symbolizer);
        let codegen = CodeGen {
            var_arg_scratch,
            function,
            context,
            runtime,
            binop_scratch,
            ptr_scratch,
            c: CodegenConsts::new(zero_ptr, zero_f, float_tag, string_tag),
            break_lbl: vec![],
            return_lbl: None,
            symbolizer,
            globals,
        };
        codegen
    }

    fn run(&mut self) {
        let function: extern "C" fn() -> i32 = self.function.to_closure();
        function();
    }

    fn compile(&mut self, mut prog: Program, debug_asserts: bool, dump: bool) -> Result<(), PrintableError> {
        let num_arrays = prog.global_analysis.global_arrays.len();
        let mut global_analysis = AnalysisResults::new();
        std::mem::swap(&mut global_analysis, &mut prog.global_analysis);
        self.globals = Globals::new(global_analysis, self.runtime, &mut self.function, self.symbolizer);

        self.return_lbl = Some(Label::new());
        let main = self.symbolizer.get("main function");
        let main = prog.functions.get(&main).unwrap();
        self.runtime.allocate_arrays(num_arrays);
        self.compile_stmt(&main.body)?;
        self.function.insn_label(&mut self.return_lbl.clone().unwrap());

        if debug_asserts {
            for value in self.globals.scalars(&mut self.function) {
                self.drop_if_str(&value);
            }
        }

        self.return_lbl = None;
        let zero = self.function.create_int_constant(0);
        self.function.insn_return(&zero);
        if dump {
            println!("{}", self.function.dump().unwrap());
        }
        self.function.compile();
        if dump {
            println!("{}", self.function.dump().unwrap());
        }

        self.context.build_end();
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                let ret_val = if let Some(ret) = ret {
                    self.compile_expr(ret, false)?
                } else {
                    self.no_op_value()
                };
                self.store(&mut self.binop_scratch.clone(), &ret_val);
                self.function.insn_branch(&mut self.return_lbl.clone().unwrap())
            }
            Stmt::Printf { args, fstring } => {
                let fstring_val = self.compile_expr(fstring, false)?;
                let fstring_ptr = self.val_to_string(&fstring_val, fstring.typ);
                // write all the values into scratch space. Runtime will read from that pointer
                for (idx, arg) in args.iter().enumerate() {
                    let compiled = self.compile_expr(arg, false)?;
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
                let res = self.compile_expr(expr, true)?;
                self.drop_if_str(&res);
            }
            Stmt::Print(expr) => {
                let val = self.compile_expr(expr, false)?;
                // Optimize print based on static knowledge of type
                match expr.typ {
                    ScalarType::String => {
                        self.runtime.print_string(&mut self.function, val.pointer.clone());
                    }
                    ScalarType::Float => {
                        self.runtime.print_float(&mut self.function, val.float);
                    }
                    ScalarType::Variable => {
                        let str = self.val_to_string(&val, expr.typ);
                        self.runtime.print_string(&mut self.function, str.clone());
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
                    let test_value = self.compile_expr(test, false)?;
                    let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                    self.drop_if_str(&test_value);
                    let mut then_lbl = Label::new();
                    let mut done_lbl = Label::new();
                    self.function.insn_branch_if(&bool_value, &mut then_lbl);
                    self.compile_stmt(if_not)?;
                    self.function.insn_branch(&mut done_lbl);
                    self.function.insn_label(&mut then_lbl);
                    self.compile_stmt(if_so)?;
                    self.function.insn_label(&mut done_lbl);
                } else {
                    let test_value = self.compile_expr(test, false)?;
                    let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                    self.drop_if_str(&test_value);
                    let mut done_lbl = Label::new();
                    self.function.insn_branch_if_not(&bool_value, &mut done_lbl);
                    self.compile_stmt(if_so)?;
                    self.function.insn_label(&mut done_lbl);
                }
            }
            Stmt::While(test, body) => {
                let mut test_label = Label::new();
                let mut done_label = Label::new();
                self.break_lbl.push(done_label.clone());
                self.function.insn_label(&mut test_label);
                let test_value = self.compile_expr(test, false)?;
                let bool_value = self.truthy_ret_integer(&test_value, test.typ);
                self.drop_if_str(&test_value);
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
    // side_effect_only:
    // print a; would be false for expr::variable(a)
    // a[0] = 5;  would be true for expr::array_assign(...) since while technically
    // a[0] = 5 is an expr and you could do y = a[0] = 5; In many cases it's just a Stmt::Expr
    // and result is unused.
    fn compile_expr(&mut self, expr: &TypedExpr, side_effect_only: bool) -> Result<ValueT, PrintableError> {
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
                    let old_value = self.globals.get(var, &mut self.function)?.clone();
                    let strings_to_concat = self.compile_exprs_to_string(vars)?;
                    self.drop_if_str(&old_value);
                    let new_value = self.concat_values(&strings_to_concat);
                    self.globals.set(&mut self.function, var, &new_value);
                    return Ok(self.copy_if_string(new_value, ScalarType::Variable));
                }
                let new_value = self.compile_expr(value, false)?;
                let old_value = self.globals.get(var, &mut self.function)?.clone();
                self.drop_if_str(&old_value);
                self.globals.set(&mut self.function, &var, &new_value);
                if side_effect_only {
                    self.no_op_value()
                } else {
                    self.copy_if_string(new_value, value.typ)
                }
            }
            Expr::NumberF64(num) => {
                ValueT::float(
                    self.float_tag(),
                    self.function.create_float64_constant(*num),
                    self.zero_ptr(),
                )
            }
            Expr::String(str) => {
                let ptr = self.function.create_void_ptr_constant(self.globals.get_const_str(&str)?);
                let new_ptr = self.runtime.copy_string(&mut self.function, ptr);
                ValueT::string(self.string_tag(), self.zero_f(), new_ptr)
            }
            Expr::Regex(str) => {
                let ptr = self.function.create_void_ptr_constant(self.globals.get_const_str(&str)?);
                let new_ptr = self.runtime.copy_string(&mut self.function, ptr);
                ValueT::string(self.string_tag(), self.zero_f(), new_ptr)
            }
            Expr::MathOp(left_expr, op, right_expr) => {
                // Convert left and right to floats if needed and perform the MathOp
                let left = self.compile_expr(left_expr, false)?;
                let right = self.compile_expr(right_expr, false)?;
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
                self.drop_if_str(&left);
                self.drop_if_str(&right);

                ValueT::float(self.float_tag(), result, self.zero_ptr())
            }
            Expr::BinOp(left_expr, op, right_expr) => {
                let left = self.compile_expr(left_expr, false)?;
                let right = self.compile_expr(right_expr, false)?;
                let tag = self.float_tag();

                // Optimize the case where we know both are floats
                if left_expr.typ == ScalarType::Float && right_expr.typ == ScalarType::Float {
                    return Ok(ValueT::float(tag,
                                            self.float_binop(&left.float, &right.float, *op),
                                            self.zero_ptr()));
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
                let result = ValueT::float(self.float_tag(), res, self.zero_ptr());
                self.store(&mut self.binop_scratch.clone(), &result);
                self.drop(&left_as_string);
                self.drop(&right_as_string);
                self.function.insn_branch(&mut done_lbl);

                // Float/Float case
                self.function.insn_label(&mut both_float_lbl);
                let float_val = self.float_binop(&left.float, &right.float, *op);
                let value = ValueT::float(tag, float_val, self.zero_ptr());
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
                        let left_val = self.compile_expr(left, false)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(&left_val);
                        self.function.insn_branch_if_not(&l, &mut ret_false);
                        let right_val = self.compile_expr(right, false)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(&right_val);
                        self.function.insn_branch_if_not(&r, &mut ret_false);
                        self.function.insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut ret_false);
                        self.function.insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.function.insn_load(&self.binop_scratch.float);
                        ValueT::float(tag, result_f, self.zero_ptr())
                    }
                    LogicalOp::Or => {
                        let mut done = Label::new();
                        let mut return_true = Label::new();
                        let left_val = self.compile_expr(left, false)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(&left_val);
                        self.function.insn_branch_if(&l, &mut return_true);
                        let right_val = self.compile_expr(right, false)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(&right_val);
                        self.function.insn_branch_if(&r, &mut return_true);
                        self.function.insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut return_true);
                        self.function.insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.function.insn_load(&self.binop_scratch.float);
                        ValueT::float(tag, result_f, self.zero_ptr())
                    }
                };
                res
            }
            Expr::Variable(var) => {
                // compile_expr returns a string/float that is 'owned' by the caller.
                // If it's a string we need to call copy_string to update the reference count.
                // If it's a float no-op.
                // If type is unknown we check tag then copy_string if needed.
                let mut var_ptr = self.globals.get(var, &mut self.function)?.clone();
                let string_tag = self.string_tag();
                match expr.typ {
                    ScalarType::String => {
                        let var = self.load(&mut var_ptr);
                        let zero = self.function.create_float64_constant(0.0);
                        let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                        ValueT::string(string_tag, zero, new_ptr)
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
                        self.function.insn_store(&self.binop_scratch.pointer, &self.zero_ptr());

                        self.function.insn_label(&mut done_lbl);
                        let str_ptr = self.function.insn_load(&self.binop_scratch.pointer);
                        ValueT::var(var.tag, var.float, str_ptr)
                    }
                    ScalarType::Float => {
                        let mut val = self.load(&mut var_ptr);
                        val.typ = ScalarType::Float;
                        val
                    }
                }
            }
            Expr::Column(col) => {
                let column = self.compile_expr(col, false)?;
                let val = self.runtime.column(
                    &mut self.function,
                    column.tag.clone(),
                    column.float.clone(),
                    column.pointer.clone(),
                );
                let tag = self.string_tag();
                self.drop_if_str(&column);
                ValueT::string(tag, self.function.create_float64_constant(0.0), val)
            }
            Expr::NextLine => {
                // Ask runtime if there is a next line. Returns a float 0 or 1
                let one = self.float_tag();
                let next_line_exists = self.runtime.call_next_line(&mut self.function);
                ValueT::float(one, next_line_exists, self.zero_ptr())
            }
            Expr::Concatenation(vars) => {
                // Eg: a = "a" "b" "c"
                let compiled = self.compile_exprs_to_string(vars)?;
                if side_effect_only {
                    for value in compiled {
                        self.drop(&value);
                    }
                    self.no_op_value()
                } else {
                    self.concat_values(&compiled)
                }
            }
            Expr::Ternary(cond, expr1, expr2) => {
                let mut done_lbl = Label::new();
                let mut truthy_lbl = Label::new();

                let result = self.compile_expr(cond, false)?;
                let bool_value = self.truthy_ret_integer(&result, cond.typ);

                self.function.insn_branch_if(&bool_value, &mut truthy_lbl);

                let falsy_result = self.compile_expr(expr2, false)?;
                self.store(&mut self.binop_scratch.clone(), &falsy_result);
                self.function.insn_branch(&mut done_lbl);

                self.function.insn_label(&mut truthy_lbl);

                let truthy_result = self.compile_expr(expr1, false)?;
                self.store(&mut self.binop_scratch.clone(), &truthy_result);

                self.function.insn_label(&mut done_lbl);

                self.load(&mut self.binop_scratch.clone())
            }
            Expr::ArrayIndex { name, indices } => {
                let array_id = self.globals.get_array(name, &mut self.function)?;

                if indices.len() == 1 {
                    let val = self.compile_expr(&indices[0], false)?;
                    self.runtime.array_access(&mut self.function, array_id,
                                              val.tag,
                                              val.float,
                                              val.pointer,
                                              self.ptr_scratch.tag.clone(),
                                              self.ptr_scratch.float.clone(),
                                              self.ptr_scratch.pointer.clone());
                } else {
                    let values = self.compile_exprs_to_string(indices)?;
                    let indices = self.concat_indices(&values);
                    // Runtime will set the out_tag out_float and out_ptr pointers to a new value. Just load em
                    let str_tag = self.string_tag();
                    let zero_f = self.zero_f();
                    self.runtime.array_access(&mut self.function, array_id,
                                              str_tag,
                                              zero_f,
                                              indices,
                                              self.ptr_scratch.tag.clone(),
                                              self.ptr_scratch.float.clone(),
                                              self.ptr_scratch.pointer.clone());
                }
                let tag = self.function.insn_load_relative(&self.ptr_scratch.tag, 0, &Context::sbyte_type());
                let float = self.function.insn_load_relative(&self.ptr_scratch.float, 0, &Context::float64_type());
                let pointer = self.function.insn_load_relative(&self.ptr_scratch.pointer, 0, &Context::void_ptr_type());
                ValueT::var(tag, float, pointer)
            }
            Expr::InArray { name, indices } => {
                let values = self.compile_exprs_to_string(indices)?;
                let value = self.concat_indices(&values);
                if side_effect_only {
                    for value in &values {
                        self.drop(value)
                    }
                    self.no_op_value()
                } else {
                    let array_id = self.globals.get_array(name, &mut self.function)?;
                    let str_tag = self.string_tag();
                    let zero_f = self.zero_f();
                    let float_result = self.runtime.in_array(&mut self.function, array_id, str_tag, zero_f, value);
                    ValueT::var(self.float_tag(), float_result, self.zero_ptr())
                }
            }
            Expr::ArrayAssign { name, indices: indices_arr, value } => {
                let array_id = self.globals.get_array(name, &mut self.function)?;
                let rhs = self.compile_expr(value, false)?;
                if indices_arr.len() == 1 {
                    let indices = self.compile_expr(&indices_arr[0], false)?;

                    let result_copy = if side_effect_only {
                        self.no_op_value()
                    } else {
                        self.copy_if_string(rhs.clone(), value.typ)
                    };
                    self.runtime.array_assign(&mut self.function, array_id,
                                              indices.tag,
                                              indices.float,
                                              indices.pointer,
                                              rhs.tag, rhs.float, rhs.pointer);
                    result_copy
                } else {
                    let values = self.compile_exprs_to_string(indices_arr)?;
                    let indices = self.concat_indices(&values);
                    // Skip copying assigned value if this side_effect_only
                    let result_copy = if side_effect_only {
                        self.no_op_value()
                    } else {
                        self.copy_if_string(rhs.clone(), value.typ)
                    };
                    let str_tag = self.string_tag();
                    let zero_f = self.zero_f();

                    self.runtime.array_assign(&mut self.function, array_id,
                                              str_tag,
                                              zero_f,
                                              indices,
                                              rhs.tag, rhs.float, rhs.pointer);
                    result_copy
                }
            }
        })
    }
}

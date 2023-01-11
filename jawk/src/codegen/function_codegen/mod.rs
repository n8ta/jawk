use crate::codegen::callable_function::CallableFunction;
use crate::codegen::codegen_consts::CodegenConsts;
use crate::codegen::function_scope::FunctionScope;
use crate::codegen::globals::Globals;
use crate::codegen::{ValuePtrT, ValueT, Tag};
use crate::lexer::{LogicalOp, MathOp};
use crate::parser::{ArgT, LValue, ScalarType, Stmt, TypedExpr};
use crate::runtime::Runtime;
use crate::symbolizer::Symbol;
use crate::typing::{BuiltinFunc, ITypedFunction, TypedUserFunction};
use crate::{Expr, PrintableError, Symbolizer};
use gnu_libjit::{Context, Function, Label, Value};
use hashbrown::HashMap;
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::rc::Rc;
use crate::codegen::function_codegen::helpers::fill_in;

mod builtin_codegen;
mod helpers;

#[allow(dead_code)]
pub struct FunctionCodegen<'a> {
    /// Core Stuff
    function: Function,
    context: &'a Context,
    function_scope: FunctionScope<'a>,
    symbolizer: &'a mut Symbolizer,
    runtime: &'a mut dyn Runtime,
    function_map: &'a HashMap<Symbol, CallableFunction>,

    /// Function Specific Items
    // These are local variables that we use as scratch space.
    binop_scratch: ValuePtrT,
    // Stack space to use for passing multiple return values from the runtime.
    ptr_scratch: ValuePtrT,
    c: CodegenConsts,
    // Where a 'break' keyword should jump
    break_lbl: Vec<Label>,
    // Where a return should jump after storing return value
    return_lbl: Label,
    // Var arg scratch for passing a variable # of printf args (max 64) allocated on the heap
    var_arg_scratch: &'a Value,
}


impl<'a> FunctionCodegen<'a> {
    pub fn build_function(
        mut function: Function,
        ast_function: &TypedUserFunction,
        runtime: &'a mut dyn Runtime,
        function_map: &'a HashMap<Symbol, CallableFunction>,
        context: &'a Context,
        globals: &'a Globals,
        symbolizer: &'a mut Symbolizer,
        var_arg_scratch: &'a Value,
        is_main: bool,
        debug_asserts: bool,
        dump: bool,
    ) -> Result<Function, PrintableError> {
        let binop_scratch = ValueT::var(
            function.create_value_int(),
            function.create_value_float64(),
            function.create_value_void_ptr(),
        );
        // TODO: This leaks 4 bytes per function compiled. We can avoid that.
        let zero = Box::leak(Box::new(0)) as *mut i32;

        let tag_scratch = function
            .create_void_ptr_constant((Box::leak(Box::new(Tag::FloatTag as i8)) as *mut i8) as *mut c_void);
        let float_scratch = function
            .create_void_ptr_constant(Box::leak(Box::new(0.0 as f64)) as *mut f64 as *mut c_void);
        let ptr_scratch_ptr = function
            .create_void_ptr_constant(Box::leak(Box::new(zero)) as *mut *mut i32 as *mut c_void);
        let ptr_scratch = ValuePtrT::var(tag_scratch, float_scratch, ptr_scratch_ptr.clone());

        // Consts
        let zero_ptr = ptr_scratch_ptr;
        let zero_f = function.create_float64_constant(0.0);
        let sentinel_float = function.create_float64_constant(123.123); // Used to init float portion of a string value
        let float_tag = function.create_sbyte_constant(Tag::FloatTag as c_char);
        let string_tag = function.create_sbyte_constant(Tag::StringTag as c_char);
        let strnum_tag = function.create_sbyte_constant(Tag::StrnumTag as c_char);
        let c = CodegenConsts::new(zero_ptr, zero_f, float_tag, string_tag, strnum_tag, sentinel_float);

        let function_scope = FunctionScope::new(globals, &mut function, ast_function.args());
        let mut func_gen = Self {
            function,
            context,
            function_scope,
            binop_scratch,
            ptr_scratch,
            var_arg_scratch,
            symbolizer,
            c,
            function_map,
            runtime,
            break_lbl: vec![],
            return_lbl: Label::new(),
        };
        func_gen.compile_function(ast_function, dump, debug_asserts, is_main)?;
        Ok(func_gen.done())
    }

    fn done(self) -> Function {
        self.function
    }

    fn compile_function(
        &mut self,
        func: &TypedUserFunction,
        dump: bool,
        debug_asserts: bool,
        is_main: bool,
    ) -> Result<(), PrintableError> {
        let zero = self.function.create_int_constant(0);

        for global in func.globals_used().iter() {
            // Pull all needed globals into function locals
            self.function_scope.get_scalar(&mut self.function, global)?;
        }

        let parser_func = func.function();
        self.compile_stmt(&parser_func.body)?;

        if !is_main {
            // Only hit if function doesn't have a return it.
            let str = self.runtime.empty_string(&mut self.function);
            let empty_str_return = ValueT::new(self.string_tag(), self.zero_f(), str);
            self.function_scope
                .return_value(&mut self.function, &empty_str_return);
        }

        self.function.insn_label(&mut self.return_lbl.clone());

        if is_main && debug_asserts {
            // Main function drops all globals when it completes
            for (global, _) in self.function_scope.global_scalars().mapping().clone() {
                let value = self
                    .function_scope
                    .get_scalar(&mut self.function, &global)?;
                self.drop_if_str(value, ScalarType::Variable)
            }
        }

        for (_name, value) in self.function_scope.args() {
            self.runtime
                .free_if_string(&mut self.function, value.clone(), ScalarType::Variable);
        }

        // All global scalars that this function used need to flushed from function locals back to the heap
        if !is_main {
            self.function_scope.flush(&mut self.function);
        }

        self.function.insn_return(&zero);
        if dump {
            println!(
                "Dumping function '{}'",
                fill_in(
                    self.function.dump().unwrap(),
                    self.runtime,
                    &self.function_scope,
                )
            );
        }
        self.function.compile();
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                let ret_val = if let Some(ret) = ret {
                    self.compile_expr(ret, false)?
                } else {
                    let str = self.runtime.empty_string(&mut self.function);
                    ValueT::new(self.string_tag(), self.zero_f(), str)
                };
                self.function_scope
                    .return_value(&mut self.function, &ret_val);
                self.function.insn_branch(&mut self.return_lbl.clone())
            }
            Stmt::Printf { args, fstring } => {
                let fstring_val = self.compile_expr(fstring, false)?;
                let fstring_ptr = self.val_to_string(&fstring_val, fstring.typ);
                // write all the values into scratch space. Runtime will read from that pointer
                for (idx, arg) in args.iter().enumerate() {
                    let compiled = self.compile_expr(arg, false)?;
                    self.function.insn_store_relative(
                        &self.var_arg_scratch,
                        (idx * 24) as c_long,
                        &compiled.tag,
                    );
                    self.function.insn_store_relative(
                        &self.var_arg_scratch,
                        (idx * 24 + 8) as c_long,
                        &compiled.float,
                    );
                    self.function.insn_store_relative(
                        &self.var_arg_scratch,
                        (idx * 24 + 16) as c_long,
                        &compiled.pointer,
                    );
                }
                let nargs = self.function.create_int_constant(args.len() as c_int);
                self.runtime.printf(
                    &mut self.function,
                    fstring_ptr,
                    nargs,
                    self.var_arg_scratch.clone(),
                );
            }
            Stmt::Break => {
                if let Some(lbl) = self.break_lbl.last_mut() {
                    self.function.insn_branch(lbl)
                } else {
                    return Err(PrintableError::new("Found break keyword outside of a loop"));
                }
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr, true)?;
                // No drop needed. Side effect_only means compile_expr will return a no_op_value which is just a 0.0 float
            }
            Stmt::Print(expr) => {
                let val = self.compile_expr(expr, false)?;
                // Optimize print based on static knowledge of type
                match expr.typ {
                    ScalarType::String => {
                        self.runtime
                            .print_string(&mut self.function, val.pointer.clone());
                    }
                    ScalarType::Float => {
                        self.runtime.print_float(&mut self.function, val.float);
                    }
                    ScalarType::Variable => {
                        let float_tag = self.float_tag();
                        let mut float_lbl = Label::new();
                        let mut done_lbl = Label::new();
                        let is_float = self.function.insn_eq(&val.tag, &float_tag);
                        self.function.insn_branch_if(&is_float, &mut float_lbl);
                        self.runtime.print_string(&mut self.function, val.pointer);
                        self.function.insn_branch(&mut done_lbl);
                        self.function.insn_label(&mut float_lbl);
                        self.runtime.print_float(&mut self.function, val.float);
                        self.function.insn_label(&mut done_lbl);
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
                    self.drop_if_str(test_value.clone(), test.typ);
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
                    self.drop_if_str(test_value.clone(), test.typ);
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
                self.drop_if_str(test_value.clone(), test.typ);
                self.function
                    .insn_branch_if_not(&bool_value, &mut done_label);
                self.compile_stmt(body)?;
                self.function.insn_branch(&mut test_label);
                self.function.insn_label(&mut done_label);
                self.break_lbl.pop().unwrap();
            }
        }
        Ok(())
    }

    fn compile_expr(
        &mut self,
        expr: &TypedExpr,
        side_effect_only: bool,
    ) -> Result<ValueT, PrintableError> {
        Ok(match &expr.expr {
            Expr::Call {
                target: target_name,
                args,
            } => {
                if let Some(builtin) = BuiltinFunc::get(target_name.to_str()) {
                    return self.compile_builtin(&builtin, args);
                }
                let target = self.function_map.get(target_name).expect("expected function to exist");
                let mut call_args = Vec::with_capacity(args.len());
                let target_args = target.args();
                for (idx, (ast_arg, target_arg)) in args.iter().zip(target_args.iter()).enumerate()
                {
                    match target_arg.typ {
                        ArgT::Scalar => {
                            let compiled = self.compile_expr(ast_arg, false)?;
                            call_args.push(compiled.tag);
                            call_args.push(compiled.float);
                            call_args.push(compiled.pointer);
                        }
                        ArgT::Array => {
                            if let Expr::Variable(sym) = &ast_arg.expr {
                                let array = self.function_scope.get_array(&mut self.function, &sym)?;
                                call_args.push(array)
                            } else {
                                return Err(PrintableError::new(format!("Tried to use scalar as arg #{} to function {} which accepts an array", idx + 1, &target_name)));
                            }
                        }
                        ArgT::Unknown => {
                            // If arg is untyped it is unused in the target function
                            // just compile for side-effects and drop the result.
                            let compiled = self.compile_expr(ast_arg, true)?;
                            self.drop_if_str(compiled, ast_arg.typ);
                        }
                    }
                }
                self.function_scope.flush(&mut self.function);
                self.function.insn_call(&target.jit_function(), call_args);
                let ret_value = self.function_scope.get_returned_value(&mut self.function);
                if side_effect_only {
                    self.drop_if_str(ret_value.clone(), ScalarType::Variable);
                    self.no_op_value()
                } else {
                    ret_value
                }
            }
            Expr::ScalarAssign(var, value) => {
                // BEGIN: Optimization
                // Optimization to allow reusing the string being assigned to by a string concat operation
                // a = "init"
                // a = a "abc" (We don't want to make a copy of a when we concat "abc" with it)
                // We first calculate a to be init and "abc" to "abc". This results in a copy being made
                // of "init" (increasing the reference count to 2). Then we drop a BEFORE actually doing the
                // concat.  Reference count returns to 1.
                // Now concat can re-use the original value since ref count is 1 it's safe to downgrade
                // from Rc -> Box

                if let Expr::Concatenation(vars) = &value.expr {
                    let old_value = self
                        .function_scope
                        .get_scalar(&mut self.function, var)?
                        .clone();
                    let strings_to_concat = self.compile_expressions_to_str(vars)?;
                    self.drop_if_str(old_value, ScalarType::Variable);
                    let new_value = self.concat_values(&strings_to_concat);
                    self.function_scope
                        .set_scalar(&mut self.function, var, &new_value);
                    return Ok(if side_effect_only {
                        self.no_op_value()
                    } else {
                        self.copy_if_string(new_value, ScalarType::Variable)
                    });
                }
                let new_value = self.compile_expr(value, false)?;
                self.assign_to_variable(var, new_value, value.typ,  side_effect_only)?
            }
            Expr::NumberF64(num) => ValueT::float(
                self.float_tag(),
                self.function.create_float64_constant(*num),
                self.zero_ptr(),
            ),
            Expr::String(str) => {
                let ptr = Rc::into_raw(str.clone()) as *mut c_void;
                let ptr = self.function.create_void_ptr_constant(ptr);
                let val = ValueT::new(self.string_tag(), self.zero_f(), ptr);
                self.runtime.copy_if_string(&mut self.function, val, ScalarType::String)
            }
            Expr::Regex(reg) => {
                let ptr = Rc::into_raw(reg.clone()) as *mut c_void;
                let ptr = self.function.create_void_ptr_constant(ptr);
                let val = ValueT::new(self.string_tag(), self.zero_f(), ptr);
                self.runtime.copy_if_string(&mut self.function, val, ScalarType::String)
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
                self.drop_if_str(left, left_expr.typ);
                self.drop_if_str(right, right_expr.typ);

                ValueT::float(self.float_tag(), result, self.zero_ptr())
            }
            Expr::BinOp(left_expr, op, right_expr) => {
                let left = self.compile_expr(left_expr, false)?;
                let right = self.compile_expr(right_expr, false)?;
                let tag = self.float_tag();

                // Optimize the case where we know both are floats
                if left_expr.typ == ScalarType::Float && right_expr.typ == ScalarType::Float {
                    return Ok(ValueT::float(
                        tag,
                        self.float_binop(&left, &right, *op),
                        self.zero_ptr(),
                    ));
                }

                let left_is_float = self.function.insn_eq(&tag, &left.tag);
                let right_is_float = self.function.insn_eq(&tag, &right.tag);
                let mut both_float_lbl = Label::new();
                let mut done_lbl = Label::new();
                let both_float = self.function.insn_and(&left_is_float, &right_is_float);
                self.function
                    .insn_branch_if(&both_float, &mut both_float_lbl);

                // String/Float Float/String String/String case
                let res = self.runtime.binop(
                    &mut self.function,
                    left.clone(),
                    right.clone(),
                    *op,
                );
                let result = ValueT::float(self.float_tag(), res, self.zero_ptr());
                self.store(&mut self.binop_scratch.clone(), &result);
                self.function.insn_branch(&mut done_lbl);

                // Float/Float case
                self.function.insn_label(&mut both_float_lbl);
                let float_val = self.float_binop(&left, &right, *op);
                let value = ValueT::float(tag, float_val, self.zero_ptr());
                self.store(&mut self.binop_scratch.clone(), &value);

                // Done load the result from scratch
                self.function.insn_label(&mut done_lbl);
                self.binop_scratch.clone()
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
                        self.drop_if_str(left_val, left.typ);
                        self.function.insn_branch_if_not(&l, &mut ret_false);
                        let right_val = self.compile_expr(right, false)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(right_val, right.typ);
                        self.function.insn_branch_if_not(&r, &mut ret_false);
                        self.function
                            .insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut ret_false);
                        self.function
                            .insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.binop_scratch.float.clone();
                        ValueT::float(tag, result_f, self.zero_ptr())
                    }
                    LogicalOp::Or => {
                        let mut done = Label::new();
                        let mut return_true = Label::new();
                        let left_val = self.compile_expr(left, false)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(left_val, left.typ);
                        self.function.insn_branch_if(&l, &mut return_true);
                        let right_val = self.compile_expr(right, false)?;
                        let r = self.truthy_ret_integer(&right_val, right.typ);
                        self.drop_if_str(right_val, right.typ);
                        self.function.insn_branch_if(&r, &mut return_true);
                        self.function
                            .insn_store(&self.binop_scratch.float, &float_0);
                        self.function.insn_branch(&mut done);
                        self.function.insn_label(&mut return_true);
                        self.function
                            .insn_store(&self.binop_scratch.float, &float_1);
                        self.function.insn_label(&mut done);
                        let tag = self.float_tag();
                        let result_f = self.binop_scratch.float.clone();
                        ValueT::float(tag, result_f, self.zero_ptr())
                    }
                };
                res
            }
            Expr::Variable(var) => {
                let var = self
                    .function_scope
                    .get_scalar(&mut self.function, var)?
                    .clone();
                self.runtime
                    .copy_if_string(&mut self.function, var, expr.typ)
            }
            Expr::Column(col) => {
                let column = self.compile_expr(col, false)?;
                let val = self.runtime.column(
                    &mut self.function,
                    column.tag.clone(),
                    column.float.clone(),
                    column.pointer.clone(),
                );
                let tag = self.strnum_tag();
                self.drop_if_str(column, col.typ);
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
                let compiled = self.compile_expressions_to_str(vars)?;
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

                self.binop_scratch.clone()
            }
            Expr::ArrayIndex { name, indices } => {
                let array_id = self.function_scope.get_array(&mut self.function, name)?;

                let indices_value = if indices.len() == 1 {
                    self.compile_expr(&indices[0], false)?
                } else {
                    let values = self.compile_expressions_to_str(indices)?;
                    let indices = self.concat_indices(&values);
                    // Runtime will set the out_tag out_float and out_ptr pointers to a new value. Just load em
                    let str_tag = self.string_tag();
                    let zero_f = self.zero_f();
                    ValueT::var(str_tag, zero_f, indices)
                };
                self.runtime.array_access(
                    &mut self.function,
                    array_id,
                    indices_value.tag,
                    indices_value.float,
                    indices_value.pointer,
                    self.ptr_scratch.tag.clone(),
                    self.ptr_scratch.float.clone(),
                    self.ptr_scratch.pointer.clone(),
                );
                let tag = self.function.insn_load_relative(
                    &self.ptr_scratch.tag,
                    0,
                    &Context::sbyte_type(),
                );
                let float = self.function.insn_load_relative(
                    &self.ptr_scratch.float,
                    0,
                    &Context::float64_type(),
                );
                let pointer = self.function.insn_load_relative(
                    &self.ptr_scratch.pointer,
                    0,
                    &Context::void_ptr_type(),
                );
                ValueT::var(tag, float, pointer)
            }
            Expr::InArray { name, indices } => {
                let values = self.compile_expressions_to_str(indices)?;
                let value = self.concat_indices(&values);
                if side_effect_only {
                    for value in &values {
                        self.drop(value)
                    }
                    self.no_op_value()
                } else {
                    let array_id = self.function_scope.get_array(&mut self.function, name)?;
                    let str_tag = self.string_tag();
                    let zero_f = self.zero_f();
                    let float_result =
                        self.runtime
                            .in_array(&mut self.function, array_id, str_tag, zero_f, value);
                    ValueT::float(self.float_tag(), float_result, self.zero_ptr())
                }
            }
            Expr::ArrayAssign {
                name,
                indices: indices_arr,
                value,
            } => {
                let rhs = self.compile_expr(value, false)?;
                self.assign_to_array(name, indices_arr, rhs, value.typ, side_effect_only)?
            }
            Expr::CallSub { arg1, arg2, arg3, global } => {
                let ere_val = self.compile_expr(arg1, false)?;
                let repl_val = self.compile_expr(arg2, false)?;
                let ere_str = self.val_to_string(&ere_val, arg1.typ);
                let repr_str = self.val_to_string(&repl_val, arg2.typ);

                let texpr = TypedExpr::new(arg3.clone().into()); // TODO: remove clone
                let arg3_value = self.compile_expr(&texpr, false)?;
                let input_ptr = self.val_to_string(&arg3_value, ScalarType::Variable);

                let is_global = self.function.create_int_constant(if *global { 1 } else { 0 });
                let new_str_ptr = self.runtime.sub(
                    &mut self.function,
                    ere_str,
                    repr_str,
                    input_ptr,
                    is_global,
                    self.ptr_scratch.float.clone(),
                );
                let new_str_ptr = self.mk_string(new_str_ptr);
                let num_replacements = self.function.insn_load_relative(&self.ptr_scratch.float, 0, &Context::float64_type());

                match arg3 {
                    LValue::Variable(name) => {
                        self.assign_to_variable(name, new_str_ptr, ScalarType::String, true)?;
                    }
                    LValue::ArrayIndex { name, indices } => {
                        self.assign_to_array(name, indices, new_str_ptr, ScalarType::String, true)?;
                    }
                    LValue::Column(_col) => {
                        todo!("sub with col")
                    }
                }
                self.mk_float(num_replacements)
            }
        })
    }
}

fn float_to_string(func: &mut Function, runtime: &mut dyn Runtime, value: &ValueT) -> Value {
    runtime.number_to_string(func, value.float.clone())
}

fn string_to_string(_func: &mut Function, _runtime: &mut dyn Runtime, value: &ValueT) -> Value {
    value.pointer.clone()
}

fn truthy_float(function: &mut Function, _runtime: &mut dyn Runtime, value: &ValueT) -> Value {
    let zero_f = function.create_float64_constant(0.0);
    function.insn_ne(&value.float, &zero_f)
}

fn truthy_string(function: &mut Function, _runtime: &mut dyn Runtime, value: &ValueT) -> Value {
    let string_len_offset = std::mem::size_of::<usize>() + std::mem::size_of::<*const u8>();
    let string_len = function.insn_load_relative(
        &value.pointer,
        string_len_offset as c_long,
        &Context::long_type(),
    );
    let zero_ulong = function.create_ulong_constant(0);
    function.insn_ne(&zero_ulong, &string_len)
}

use std::os::raw::{c_char, c_int, c_long, c_void};
use gnu_libjit::{Context, Function, Label, Value};
use hashbrown::HashMap;
use crate::codegen::codegen_consts::CodegenConsts;
use crate::codegen::globals::Globals;
use crate::codegen::{FLOAT_TAG, STRING_TAG, ValuePtrT, ValueT};
use crate::parser::{ArgT, ScalarType, Stmt, TypedExpr};
use crate::{Expr, PrintableError, Symbolizer};
use crate::codegen::callable_function::CallableFunction;
use crate::codegen::function_scope::FunctionScope;
use crate::lexer::{BinOp, LogicalOp, MathOp};
use crate::runtime::Runtime;
use crate::symbolizer::Symbol;

#[allow(dead_code)]
pub struct FunctionCodegen<'a, RuntimeT: Runtime> {
    /// Core Stuff
    function: Function,
    context: &'a Context,
    function_scope: FunctionScope<'a>,
    symbolizer: &'a mut Symbolizer,
    runtime: &'a mut RuntimeT,
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

fn fill_in<RuntimeT: Runtime>(mut body: String, runtime: &RuntimeT, scope: &FunctionScope) -> String {
    let mut mapping = scope.debug_mapping();
    let free_ptr = runtime.free_string_ptr() as i64;
    let free_ptr_hex = format!("0x{:x}", free_ptr);
    mapping.insert(free_ptr_hex, format!("free_string {}", free_ptr));
    mapping.insert(format!("{} {:?}", runtime.runtime_data_ptr() as i64, runtime.runtime_data_ptr())
                   , "runtime_data_ptr".to_string());
    for (from, to) in mapping {
        body = body.replace(&from, &to)
    }

    body
}

impl<'a, RuntimeT: Runtime> FunctionCodegen<'a, RuntimeT> {
    pub fn build_function(mut function: Function,
                          parser_func: &crate::parser::Function,
                          runtime: &'a mut RuntimeT,
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

        let tag_scratch = function.create_void_ptr_constant((Box::leak(Box::new(FLOAT_TAG)) as *mut i8) as *mut c_void);
        let float_scratch = function.create_void_ptr_constant(Box::leak(Box::new(0.0 as f64)) as *mut f64 as *mut c_void);
        let ptr_scratch_ptr = function.create_void_ptr_constant(Box::leak(Box::new(zero)) as *mut *mut i32 as *mut c_void);
        let ptr_scratch = ValuePtrT::var(tag_scratch, float_scratch, ptr_scratch_ptr.clone());

        // Consts
        let zero_ptr = ptr_scratch_ptr;
        let zero_f = function.create_float64_constant(0.0);
        let sentinel_float = function.create_float64_constant(123.123); // Used to init float portion of a string value
        let float_tag = function.create_sbyte_constant(FLOAT_TAG as c_char);
        let string_tag = function.create_sbyte_constant(STRING_TAG as c_char);
        let c = CodegenConsts::new(zero_ptr, zero_f, float_tag, string_tag, sentinel_float);

        let function_scope = FunctionScope::new(globals, &mut function, &parser_func.args);
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


        func_gen.compile_function(parser_func, dump, debug_asserts, is_main)?;
        Ok(func_gen.done())
    }

    fn done(self) -> Function {
        self.function
    }

    fn compile_function(&mut self,
                        func: &crate::parser::Function,
                        dump: bool,
                        debug_asserts: bool,
                        is_main: bool) -> Result<(), PrintableError> {
        let zero = self.function.create_int_constant(0);

        for global in &func.globals_used {
            // Pull all needed globals into function locals
            self.function_scope.get_scalar(&mut self.function, global)?;
        }

        self.compile_stmt(&func.body)?;


        if !is_main {
            // Only hit if function doesn't have a return it.
            let str = self.runtime.empty_string(&mut self.function);
            let empty_str_return = ValueT::new(self.string_tag(), self.zero_f(), str);
            self.function_scope.return_value(&mut self.function, &empty_str_return);
        }

        self.function.insn_label(&mut self.return_lbl.clone());

        if is_main && debug_asserts {
            for global in self.function_scope.all_globals(&mut self.function) {
                self.drop_if_str(&global, ScalarType::Variable)
            }
        }

        for (_name, value) in self.function_scope.args() {
            FunctionCodegen::drop_if_str_no_borrow(self.runtime, &mut self.function, value, ScalarType::Variable);
        }

        // All global scalars that this function used need to flushed from function locals back to the heap
        self.function_scope.flush(&mut self.function);

        self.function.insn_return(&zero);
        if dump {
            println!("Dumping function '{}'", fill_in(self.function.dump().unwrap(), self.runtime, &self.function_scope));
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
                self.function_scope.return_value(&mut self.function, &ret_val);
                self.function.insn_branch(&mut self.return_lbl.clone())
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
                self.compile_expr(expr, true)?;
                // No drop needed. Side effect_only means compile_expr will return a no_op_value which is just a 0.0 float
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
                    self.drop_if_str(&test_value, test.typ);
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
                    self.drop_if_str(&test_value, test.typ);
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
    // side_effect_only:
    // print a; would be false for expr::variable(a)
    // a[0] = 5;  would be true for expr::array_assign(...) since while technically
    // a[0] = 5 is an expr and you could do y = a[0] = 5; In many cases it's just a Stmt::Expr
    // and result is unused.
    fn compile_expr(&mut self, expr: &TypedExpr, side_effect_only: bool) -> Result<ValueT, PrintableError> {
        Ok(match &expr.expr {
            Expr::Call { target: target_name, args } => {
                let target = self.function_map.get(target_name).expect("function to exist");
                let mut call_args = Vec::with_capacity(args.len());
                for (idx, (ast_arg, target_arg)) in args.iter().zip(&target.args).enumerate() {
                    match target_arg.typ {
                        None => {
                            // If arg is untyped it is unused in the target function
                            // just compile for side-effects and drop the result.
                            let compiled = self.compile_expr(ast_arg, true)?;
                            self.drop_if_str(&compiled, ast_arg.typ);
                        }
                        Some(arg_t) => {
                            match arg_t {
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
                            }
                        }
                    }
                }
                self.function_scope.flush(&mut self.function);
                self.function.insn_call(&target.function, call_args);
                let ret_value = self.function_scope.get_returned_value(&mut self.function);
                if side_effect_only {
                    self.drop_if_str(&ret_value, ScalarType::Variable);
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
                    let old_value = self.function_scope.get_scalar(&mut self.function, var)?.clone();
                    let strings_to_concat = self.compile_expressions_to_str(vars)?;
                    self.drop_if_str(&old_value, ScalarType::Variable);
                    let new_value = self.concat_values(&strings_to_concat);
                    self.function_scope.set_scalar(&mut self.function, var, &new_value);
                    return Ok(if side_effect_only {
                        self.no_op_value()
                    } else {
                        self.copy_if_string(new_value, ScalarType::Variable)
                    });
                }
                let new_value = self.compile_expr(value, false)?;
                let old_value = self.function_scope.get_scalar(&mut self.function, var)?.clone();
                self.drop_if_str(&old_value, ScalarType::Variable);
                self.function_scope.set_scalar(&mut self.function, &var, &new_value);
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
                let ptr = self.function.create_void_ptr_constant(self.function_scope.get_const_str(&str)?);
                let new_ptr = self.runtime.copy_string(&mut self.function, ptr);
                ValueT::string(self.string_tag(), self.sentinel_f(), new_ptr)
            }
            Expr::Regex(str) => {
                let ptr = self.function.create_void_ptr_constant(self.function_scope.get_const_str(&str)?);
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
                self.drop_if_str(&left, left_expr.typ);
                self.drop_if_str(&right, right_expr.typ);

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
                        self.drop_if_str(&left_val, left.typ);
                        self.function.insn_branch_if_not(&l, &mut ret_false);
                        let right_val = self.compile_expr(right, false)?;
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
                        ValueT::float(tag, result_f, self.zero_ptr())
                    }
                    LogicalOp::Or => {
                        let mut done = Label::new();
                        let mut return_true = Label::new();
                        let left_val = self.compile_expr(left, false)?;
                        let l = self.truthy_ret_integer(&left_val, left.typ);
                        self.drop_if_str(&left_val, left.typ);
                        self.function.insn_branch_if(&l, &mut return_true);
                        let right_val = self.compile_expr(right, false)?;
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
                let var = self.function_scope.get_scalar(&mut self.function, var)?.clone();
                let string_tag = self.string_tag();
                match expr.typ {
                    ScalarType::String => {
                        let zero = self.function.create_float64_constant(0.0);
                        let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                        ValueT::string(string_tag, zero, new_ptr)
                    }
                    ScalarType::Variable => {
                        // If it's a string variable copy it and store that pointer in self.binop_scratch.pointer
                        // otherwise store zero self.binop_scratch.pointer. After this load self.binop_scratch.pointer
                        // and make a new value with the old tag/float + new string pointer.
                        let is_string = self.function.insn_eq(&string_tag, &var.tag);
                        let mut done_lbl = Label::new();
                        let mut is_not_str_lbl = Label::new();
                        self.function.insn_branch_if_not(&is_string, &mut is_not_str_lbl);
                        let new_ptr = self.runtime.copy_string(&mut self.function, var.pointer);
                        self.function.insn_store(&self.binop_scratch.pointer, &new_ptr);
                        self.function.insn_branch(&mut done_lbl);

                        self.function.insn_label(&mut is_not_str_lbl);
                        self.function.insn_store(&self.binop_scratch.pointer, &self.zero_ptr());

                        self.function.insn_label(&mut done_lbl);
                        let str_ptr = self.function.insn_load(&self.binop_scratch.pointer);
                        ValueT::var(var.tag, var.float, str_ptr)
                    }
                    ScalarType::Float => var,
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
                self.drop_if_str(&column, col.typ);
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

                self.load(&mut self.binop_scratch.clone())
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
                self.runtime.array_access(&mut self.function, array_id,
                                          indices_value.tag,
                                          indices_value.float,
                                          indices_value.pointer,
                                          self.ptr_scratch.tag.clone(),
                                          self.ptr_scratch.float.clone(),
                                          self.ptr_scratch.pointer.clone());
                let tag = self.function.insn_load_relative(&self.ptr_scratch.tag, 0, &Context::sbyte_type());
                let float = self.function.insn_load_relative(&self.ptr_scratch.float, 0, &Context::float64_type());
                let pointer = self.function.insn_load_relative(&self.ptr_scratch.pointer, 0, &Context::void_ptr_type());
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
                    let float_result = self.runtime.in_array(&mut self.function, array_id, str_tag, zero_f, value);
                    ValueT::float(self.float_tag(), float_result, self.zero_ptr())
                }
            }
            Expr::ArrayAssign { name, indices: indices_arr, value } => {
                let array_id = self.function_scope.get_array(&mut self.function, name)?;
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
                    let values = self.compile_expressions_to_str(indices_arr)?;
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

    pub fn float_tag(&self) -> Value {
        self.c.float_tag.clone()
    }
    pub fn string_tag(&self) -> Value {
        self.c.string_tag.clone()
    }
    pub fn zero_f(&self) -> Value {
        self.c.zero_f.clone()
    }
    pub fn sentinel_f(&self) -> Value {
        self.c.sentinel_float.clone()
    }

    pub fn zero_ptr(&self) -> Value {
        self.c.zero_ptr.clone()
    }

    pub fn cases(
        &mut self,
        input: &ValueT,
        input_type: ScalarType,
        is_ptr: bool,
        emit_float_code: fn(&mut Function, &mut RuntimeT, &ValueT) -> Value,
        emit_string_code: fn(&mut Function, &mut RuntimeT, &ValueT) -> Value,
    ) -> Value {
        match input_type {
            ScalarType::String => return emit_string_code(&mut self.function, &mut self.runtime, input),
            ScalarType::Float => return emit_float_code(&mut self.function, &mut self.runtime, input),
            _ => {}
        }
        let mut temp_storage = if is_ptr { self.binop_scratch.pointer.clone() } else { self.binop_scratch.float.clone() };

        let string_tag = self.string_tag();
        let mut string_lbl = Label::new();
        let mut done_lbl = Label::new();
        let is_string = self.function.insn_eq(&input.tag, &string_tag);
        self.function.insn_branch_if(&is_string, &mut string_lbl);
        let res = emit_float_code(&mut self.function, &mut self.runtime, input);
        self.function.insn_store(&mut temp_storage, &res);
        self.function.insn_branch(&mut done_lbl);
        self.function.insn_label(&mut string_lbl);
        let res = emit_string_code(&mut self.function, &mut self.runtime, input);
        self.function.insn_store(&mut temp_storage, &res);
        self.function.insn_label(&mut done_lbl);
        self.function.insn_load(&temp_storage)
    }

    pub fn val_to_float(&mut self, value: &ValueT, typ: ScalarType) -> Value {
        if typ == ScalarType::Float {
            return value.float.clone();
        }

        let zero = self.function.create_sbyte_constant(0);
        let mut done_lbl = Label::new();
        self.function.insn_store(&&self.binop_scratch.float, &value.float);
        let is_float = self.function.insn_eq(&value.tag, &zero);
        self.function.insn_branch_if(&is_float, &mut done_lbl);

        let res = self.runtime.string_to_number(&mut self.function, value.pointer.clone());
        self.function.insn_store(&&self.binop_scratch.float, &res);

        self.function.insn_label(&mut done_lbl);
        self.function.insn_load(&self.binop_scratch.float)
    }

    pub fn val_to_string(&mut self, value: &ValueT, typ: ScalarType) -> Value {
        if typ == ScalarType::String {
            return value.pointer.clone();
        }
        self.cases(
            value,
            typ,
            true,
            float_to_string,
            string_to_string,
        )
    }

    // Free the value if it's a string
    pub fn drop_if_str(&mut self, value: &ValueT, typ: ScalarType) {
        FunctionCodegen::drop_if_str_no_borrow(self.runtime, &mut self.function, value, typ);
    }

    fn drop_if_str_no_borrow(runtime: &mut RuntimeT, function: &mut Function, value: &ValueT, typ: ScalarType) {
        match typ {
            ScalarType::String => {
                runtime.free_string(function, value.pointer.clone());
            }
            ScalarType::Variable => {
                let str_tag = function.create_sbyte_constant(STRING_TAG);
                let mut done_lbl = Label::new();
                let is_string = function.insn_eq(&str_tag, &value.tag);
                function.insn_branch_if_not(&is_string, &mut done_lbl);
                runtime.free_string(function, value.pointer.clone());
                function.insn_label(&mut done_lbl);
            }
            _ => {}
        };
    }

    pub fn drop(&mut self, value: &Value) {
        self.runtime.free_string(&mut self.function, value.clone());
    }

    // Take a value and return an int 0 or 1
    pub fn truthy_ret_integer(&mut self, value: &ValueT, typ: ScalarType) -> Value {
        self.cases(value, typ, false, truthy_float, truthy_string)
    }

    pub fn no_op_value(&self) -> ValueT {
        ValueT::new(self.float_tag(), self.zero_f(), self.zero_ptr())
    }

    pub fn copy_if_string(&mut self, value: ValueT, typ: ScalarType) -> ValueT {
        let zero = self.function.create_float64_constant(0.0);
        let str_tag = self.string_tag();
        match typ {
            ScalarType::String => {
                let ptr = self.runtime.copy_string(&mut self.function, value.pointer);
                ValueT::new(str_tag, zero, ptr)
            }
            ScalarType::Float => value, // Float copy is a no-op
            ScalarType::Variable => {
                // If type unknown, check tag and call runtime if it's a string
                let mut done = Label::new();
                let is_string = self.function.insn_eq(&str_tag, &value.tag);
                self.function
                    .insn_store(&self.binop_scratch.pointer, &self.c.zero_ptr);
                self.function.insn_branch_if_not(&is_string, &mut done);
                let ptr = self.runtime.copy_string(&mut self.function, value.pointer);
                self.function.insn_store(&self.binop_scratch.pointer, &ptr);
                self.function.insn_label(&mut done);
                let string = self.function.insn_load(&self.binop_scratch.pointer);
                ValueT::string(value.tag, value.float, string)
            }
        }
    }

    pub fn float_binop(&mut self, a: &Value, b: &Value, op: BinOp) -> Value {
        let bool = match op {
            BinOp::Greater => self.function.insn_gt(a, b),
            BinOp::GreaterEq => self.function.insn_ge(a, b),
            BinOp::Less => self.function.insn_lt(a, b),
            BinOp::LessEq => self.function.insn_le(a, b),
            BinOp::BangEq => self.function.insn_ne(a, b),
            BinOp::EqEq => self.function.insn_eq(a, b),
            BinOp::MatchedBy | BinOp::NotMatchedBy => {
                let a_str = self.runtime.number_to_string(&mut self.function, a.clone());
                let b_str = self.runtime.number_to_string(&mut self.function, b.clone());
                return self.runtime.binop(&mut self.function, a_str, b_str, op);
            }
        };
        let one = self.function.create_float64_constant(1.0);
        let zero = self.function.create_float64_constant(0.0);
        let mut true_lbl = Label::new();
        let mut done_lbl = Label::new();
        self.function.insn_branch_if(&bool, &mut true_lbl);
        self.function.insn_store(&self.binop_scratch.float, &zero);
        self.function.insn_branch(&mut done_lbl);
        self.function.insn_label(&mut true_lbl);
        self.function.insn_store(&self.binop_scratch.float, &one);

        self.function.insn_label(&mut done_lbl);
        self.function.insn_load(&self.binop_scratch.float)
    }

    pub fn compile_expressions_to_str(&mut self, expressions: &Vec<TypedExpr>) -> Result<Vec<Value>, PrintableError> {
        let mut strings = Vec::with_capacity(expressions.len());
        for expr in expressions {
            let val = self.compile_expr(expr, false)?;
            let string = self.val_to_string(&val, expr.typ);
            strings.push(string)
        }
        Ok(strings)
    }

    // Call runtime and combine values. All values MUST be strings.
    pub fn concat_values(&mut self, compiled: &Vec<Value>) -> ValueT {
        let mut result = self.runtime.concat(
            &mut self.function,
            compiled.get(0).unwrap().clone(),
            compiled.get(1).unwrap().clone(),
        );
        if compiled.len() >= 3 {
            for var in &compiled[2..] {
                result = self.runtime.concat(&mut self.function, result, var.clone());
            }
        }
        ValueT::string(self.string_tag(), self.zero_f(), result)
    }

    // Concat indices all values MUST be strings
    pub fn concat_indices(&mut self, compiled: &Vec<Value>) -> Value {
        if compiled.len() == 1 {
            return compiled[0].clone();
        }
        let mut result = self.runtime.concat_array_indices(
            &mut self.function,
            compiled.get(0).unwrap().clone(),
            compiled.get(1).unwrap().clone(),
        );
        if compiled.len() >= 3 {
            for var in &compiled[2..] {
                result = self.runtime.concat(&mut self.function, result, var.clone());
            }
        }
        result
    }

    pub fn load(&mut self, ptr: &mut ValuePtrT) -> ValueT {
        let ptr_tag = self.function.address_of(&mut ptr.tag);
        let ptr_float = self.function.address_of(&mut ptr.float);
        let ptr_ptr = self.function.address_of(&mut ptr.pointer);
        let tag = self.function.insn_load_relative(&ptr_tag, 0, &Context::sbyte_type());
        let val = self.function.insn_load_relative(&ptr_float, 0, &Context::float64_type());
        let ptr = self.function.insn_load_relative(&ptr_ptr, 0, &Context::void_ptr_type());
        ValueT::var(tag, val, ptr)
    }

    pub fn store(&mut self, ptr: &mut ValuePtrT, value: &ValueT) {
        let ptr_tag = self.function.address_of(&mut ptr.tag);
        let ptr_float = self.function.address_of(&mut ptr.float);
        let ptr_ptr = self.function.address_of(&mut ptr.pointer);
        self.function.insn_store_relative(&ptr_tag, 0, &value.tag);
        self.function.insn_store_relative(&ptr_float, 0, &value.float);
        self.function.insn_store_relative(&ptr_ptr, 0, &value.pointer);
    }
}

fn float_to_string<RuntimeT: Runtime>(func: &mut Function, runtime: &mut RuntimeT, value: &ValueT) -> Value {
    runtime.number_to_string(func, value.float.clone())
}

fn string_to_string<RuntimeT: Runtime>(_func: &mut Function, _runtime: &mut RuntimeT, value: &ValueT) -> Value {
    value.pointer.clone()
}

fn truthy_float<RuntimeT: Runtime>(function: &mut Function, _runtime: &mut RuntimeT, value: &ValueT) -> Value {
    let zero_f = function.create_float64_constant(0.0);
    function.insn_ne(&value.float, &zero_f)
}

fn truthy_string<RuntimeT: Runtime>(function: &mut Function, _runtime: &mut RuntimeT, value: &ValueT) -> Value {
    let string_len_offset =
        std::mem::size_of::<usize>() + std::mem::size_of::<*const u8>();
    let string_len = function.insn_load_relative(
        &value.pointer,
        string_len_offset as c_long,
        &Context::long_type(),
    );
    let zero_ulong = function.create_ulong_constant(0);
    function.insn_ne(&zero_ulong, &string_len)
}

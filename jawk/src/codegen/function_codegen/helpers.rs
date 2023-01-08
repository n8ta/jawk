use gnu_libjit::{Function, Label, Value};
use crate::codegen::function_codegen::{float_to_string, FunctionCodegen, string_to_string, truthy_float, truthy_string};
use crate::codegen::function_scope::FunctionScope;
use crate::codegen::{ValuePtrT, ValueT};
use crate::lexer::BinOp;
use crate::parser::{ScalarType, TypedExpr};
use crate::printable_error::PrintableError;
use crate::runtime::Runtime;
use crate::symbolizer::Symbol;

impl<'a> FunctionCodegen<'a> {
    pub fn float_tag(&self) -> Value {
        self.c.float_tag.clone()
    }

    pub fn string_tag(&self) -> Value {
        self.c.string_tag.clone()
    }

    pub fn strnum_tag(&self) -> Value {
        self.c.strnum_tag.clone()
    }

    pub fn zero_f(&self) -> Value {
        self.c.zero_f.clone()
    }

    pub fn zero_ptr(&self) -> Value {
        self.c.zero_ptr.clone()
    }

    fn cases(
        &mut self,
        input: &ValueT,
        input_type: ScalarType,
        is_ptr: bool,
        emit_float_code: fn(&mut Function, &mut dyn Runtime, &ValueT) -> Value,
        emit_string_code: fn(&mut Function, &mut dyn Runtime, &ValueT) -> Value,
    ) -> Value {
        match input_type {
            ScalarType::String => return emit_string_code(&mut self.function, self.runtime, input),
            ScalarType::Float => return emit_float_code(&mut self.function, self.runtime, input),
            _ => {}
        }
        let mut temp_storage = if is_ptr {
            self.binop_scratch.pointer.clone()
        } else {
            self.binop_scratch.float.clone()
        };

        let string_tag = self.string_tag();
        let mut string_lbl = Label::new();
        let mut done_lbl = Label::new();
        let is_string = self.function.insn_eq(&input.tag, &string_tag);
        self.function.insn_branch_if(&is_string, &mut string_lbl);
        let res = emit_float_code(&mut self.function, self.runtime, input);
        self.function.insn_store(&mut temp_storage, &res);
        self.function.insn_branch(&mut done_lbl);
        self.function.insn_label(&mut string_lbl);
        let res = emit_string_code(&mut self.function, self.runtime, input);
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
        self.cases(value, typ, true, float_to_string, string_to_string)
    }

    // Free the value if it's a string
    pub fn drop_if_str(&mut self, value: ValueT, typ: ScalarType) {
        self.runtime.free_if_string(&mut self.function, value, typ);
    }

    pub fn drop(&mut self, value: &Value) {
        self.drop_if_str(
            ValueT::new(self.string_tag(), self.zero_f(), value.clone()),
            ScalarType::String,
        );
    }

    // Take a value and return an int 0 or 1
    pub fn truthy_ret_integer(&mut self, value: &ValueT, typ: ScalarType) -> Value {
        self.cases(value, typ, false, truthy_float, truthy_string)
    }

    pub fn no_op_value(&self) -> ValueT {
        ValueT::new(self.float_tag(), self.zero_f(), self.zero_ptr())
    }

    pub fn copy_if_string(&mut self, value: ValueT, typ: ScalarType) -> ValueT {
        self.runtime.copy_if_string(&mut self.function, value, typ)
    }

    pub fn float_binop(&mut self, a: &ValueT, b: &ValueT, op: BinOp) -> Value {
        let bool = match op {
            BinOp::Greater => self.function.insn_gt(&a.float, &b.float),
            BinOp::GreaterEq => self.function.insn_ge(&a.float, &b.float),
            BinOp::Less => self.function.insn_lt(&a.float, &b.float),
            BinOp::LessEq => self.function.insn_le(&a.float, &b.float),
            BinOp::BangEq => self.function.insn_ne(&a.float, &b.float),
            BinOp::EqEq => self.function.insn_eq(&a.float, &b.float),
            BinOp::MatchedBy | BinOp::NotMatchedBy => {
                return self.runtime.binop(&mut self.function, a.clone(), b.clone(), op);
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

    pub fn compile_expressions_to_str(
        &mut self,
        expressions: &Vec<TypedExpr>,
    ) -> Result<Vec<Value>, PrintableError> {
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

    pub fn store(&mut self, ptr: &mut ValuePtrT, value: &ValueT) {
        self.function.insn_store(&ptr.tag, &value.tag);
        self.function.insn_store(&ptr.float, &value.float);
        self.function.insn_store(&ptr.pointer, &value.pointer);
    }

    pub fn assign_to_variable(&mut self,
                              var: &Symbol,
                              new_value: ValueT,
                              new_value_typ: ScalarType,
                              side_effect_only: bool) -> Result<ValueT, PrintableError> {
        let old_value = self.function_scope.get_scalar(&mut self.function, var)?.clone();
        self.drop_if_str(old_value, ScalarType::Variable);
        self.function_scope.set_scalar(&mut self.function, &var, &new_value);
        if side_effect_only {
            Ok(self.no_op_value())
        } else {
            Ok(self.copy_if_string(new_value, new_value_typ))
        }
    }

    pub fn assign_to_array(&mut self, name: &Symbol, indices_arr: &Vec<TypedExpr>, rhs: ValueT, rhs_type: ScalarType, side_effect_only: bool) -> Result<ValueT, PrintableError> {
        let array_id = self.function_scope.get_array(&mut self.function, name)?;
        if indices_arr.len() == 1 {
            let indices = self.compile_expr(&indices_arr[0], false)?;
            let result_copy = if side_effect_only {
                self.no_op_value()
            } else {
                self.copy_if_string(rhs.clone(), rhs_type)
            };
            self.runtime.array_assign(
                &mut self.function,
                array_id,
                indices.tag,
                indices.float,
                indices.pointer,
                rhs.tag,
                rhs.float,
                rhs.pointer,
            );
            Ok(result_copy)
        } else {
            let values = self.compile_expressions_to_str(indices_arr)?;
            let indices = self.concat_indices(&values);
            // Skip copying assigned value if this side_effect_only
            let result_copy = if side_effect_only {
                self.no_op_value()
            } else {
                self.copy_if_string(rhs.clone(), rhs_type)
            };
            let str_tag = self.string_tag();
            let zero_f = self.zero_f();

            self.runtime.array_assign(
                &mut self.function,
                array_id,
                str_tag,
                zero_f,
                indices,
                rhs.tag,
                rhs.float,
                rhs.pointer,
            );
            Ok(result_copy)
        }
    }
}

pub fn fill_in(mut body: String, runtime: &dyn Runtime, scope: &FunctionScope) -> String {
    let var_name_mapping = scope.debug_mapping();
    let runtime_mapping = runtime.pointer_to_name_mapping();
    for (from, to) in var_name_mapping
        .into_iter()
        .chain(runtime_mapping.into_iter())
    {
        body = body.replace(&from, &to)
    }
    body
}

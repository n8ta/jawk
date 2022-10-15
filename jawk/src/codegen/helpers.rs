use crate::lexer::{BinOp};
use crate::parser::{ScalarType, TypedExpr};
use crate::printable_error::PrintableError;
use crate::runtime::{Runtime};
use gnu_libjit::{Context, Function, Label, Value};
use std::os::raw::{c_long};
use crate::codegen::{CodeGen, ValuePtrT, ValueT};

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

impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    // Helpers for commonly used values
    pub fn float_tag(&self) -> Value {
        self.c.float_tag.clone()
    }
    pub fn string_tag(&self) -> Value {
        self.c.string_tag.clone()
    }
    pub fn zero_f(&self) -> Value {
        self.c.zero_f.clone()
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
    pub fn drop_if_str(&mut self, value: &ValueT) {
        // self.runtime.column(&mut self.function, value.tag.clone(), value.float.clone(), value.pointer.clone());
        match value.typ {
            ScalarType::String => {
                self.drop(&value.pointer);
            }
            ScalarType::Variable => {
                let str_tag = self.string_tag();
                let mut done_lbl = Label::new();
                let is_string = self.function.insn_eq(&str_tag, &value.tag);
                self.function.insn_branch_if_not(&is_string, &mut done_lbl);
                self.drop(&value.pointer);
                self.function.insn_label(&mut done_lbl);
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
        ValueT::new(self.float_tag(), self.zero_f(), self.zero_ptr(), ScalarType::Float)
    }

    pub fn copy_if_string(&mut self, value: ValueT, typ: ScalarType) -> ValueT {
        let zero = self.function.create_float64_constant(0.0);
        let str_tag = self.string_tag();
        match typ {
            ScalarType::String => {
                let ptr = self.runtime.copy_string(&mut self.function, value.pointer);
                ValueT::new(str_tag, zero, ptr, ScalarType::String)
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
                let astr = self.runtime.number_to_string(&mut self.function, a.clone());
                let bstr = self.runtime.number_to_string(&mut self.function, b.clone());
                return self.runtime.binop(&mut self.function, astr, bstr, op);
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

    pub fn compile_exprs_to_string(&mut self, exprs: &Vec<TypedExpr>) -> Result<Vec<Value>, PrintableError> {
        let mut expressions = Vec::with_capacity(exprs.len());
        for expr in exprs {
            let val = self.compile_expr(expr, false)?;
            let string = self.val_to_string(&val, expr.typ);
            expressions.push(string)
        }
        Ok(expressions)
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
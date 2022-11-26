use gnu_libjit::Function;
use crate::codegen::function_codegen::FunctionCodegen;
use crate::codegen::ValueT;
use crate::parser::{TypedExpr};
use crate::printable_error::PrintableError;
use crate::runtime::Runtime;
use crate::typing::BuiltinFunc;

impl<'a> FunctionCodegen<'a> {
    fn compile_builtin(&mut self, builtin: &BuiltinFunc,
                       args: &Vec<TypedExpr>) -> Result<ValueT, PrintableError> {
        match builtin {
            BuiltinFunc::Atan2 => todo!(),
            BuiltinFunc::Close => todo!(),
            BuiltinFunc::Cos => todo!(),
            BuiltinFunc::Exp => todo!(),
            BuiltinFunc::Gsub => todo!(),
            BuiltinFunc::Index => todo!(),
            BuiltinFunc::Int => {
                let compiled = self.compile_expr(&arg[0], false)?;
                let float = self.val_to_float(&compiled, arg[0].typ);
                let floored = self.function.insn_trunc(&float);
                self.drop_if_str(compiled, arg[0].typ);
                Ok(ValueT::new(self.float_tag(), floored, self.zero_ptr()))
            }
            BuiltinFunc::Length => todo!(),
            BuiltinFunc::Log => todo!(),
            BuiltinFunc::Matches => todo!(),
            BuiltinFunc::Rand => todo!(),
            BuiltinFunc::Sin => todo!(),
            BuiltinFunc::Split => todo!(),
            BuiltinFunc::Sprintf => todo!(),
            BuiltinFunc::Sqrt => todo!(),
            BuiltinFunc::Srand => todo!(),
            BuiltinFunc::Sub => todo!(),
            BuiltinFunc::Substr => todo!(),
            BuiltinFunc::System => todo!(),
            BuiltinFunc::Tolower => {
                let compiled = self.compile_expr(&arg[0], false)?;
                let string = self.val_to_string(&compiled, arg[0].typ);
                self.runtime.to_lower(*s)
            }
            BuiltinFunc::Toupper => {
                let compiled = self.compile_expr(&arg[0], false)?;
                let string = self.val_to_string(&compiled, arg[0].typ);
                self.runtime.to_lower(*s)
            }
        }
    }
}
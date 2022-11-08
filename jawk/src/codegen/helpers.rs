use crate::lexer::{BinOp};
use crate::parser::{ScalarType, TypedExpr};
use crate::printable_error::PrintableError;
use crate::runtime::{Runtime};
use gnu_libjit::{Context, Function, Label, Value};
use std::os::raw::{c_long};
use crate::codegen::{CodeGen, ValuePtrT, ValueT};


impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    // Helpers for commonly used values

}
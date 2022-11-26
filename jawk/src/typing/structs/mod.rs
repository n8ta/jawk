mod function_map;
mod typed_user_function;
mod call;
mod typed_program;

pub use typed_user_function::TypedUserFunction;
pub use function_map::FunctionMap;
pub use call::{Call, CallArg};
pub use crate::typing::ityped_function::ITypedFunction;
pub use typed_program::TypedProgram;
mod analysis_results;
mod call;
mod function_map;
mod typed_program;
mod typed_user_function;

pub use crate::typing::ityped_function::ITypedFunction;
pub use analysis_results::{AnalysisResults, MapT};
pub use call::{Call, CallArg};
pub use function_map::FunctionMap;
pub use typed_program::TypedProgram;
pub use typed_user_function::TypedUserFunction;

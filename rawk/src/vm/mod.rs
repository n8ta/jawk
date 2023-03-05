mod bytecode;
mod vm_func;
mod vm_program;
mod machine;
pub mod runtime_scalar;

pub use bytecode::{Code, Label, LabelId};
pub use vm_func::VmFunc;
pub use vm_program::VmProgram;
pub use machine::VirtualMachine;
pub use runtime_scalar::{RuntimeScalar, StringScalar};
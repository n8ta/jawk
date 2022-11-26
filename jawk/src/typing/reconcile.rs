use crate::parser::{Arg, ArgT};
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;


// Reconcile the types of a call with the types of the target function
pub fn reconcile(
    call_args: &[ArgT],
    func_args: &mut [Arg],
    func_name: Symbol,
    update_callback: &mut dyn FnMut(Symbol),
) -> Result<(), PrintableError> {
    if call_args.len() != func_args.len() {
        return Err(PrintableError::new(format!("fatal: call to `{}` with {} args but accepts {} args", func_name, call_args.len(), func_args.len())));
    }
    for (func_arg, call_arg) in func_args.iter_mut().zip(call_args.iter()) {
        match (func_arg.typ, call_arg) {
            // Mismatch
            (ArgT::Scalar, ArgT::Array) => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context in function call to {}", func_arg.name, func_name))),
            (ArgT::Array, ArgT::Scalar) => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context in function call to {}", func_arg.name, func_name))),
            // Function doesn't known arg type so just accept caller type
            (ArgT::Unknown, ArgT::Scalar)
            | (ArgT::Unknown, ArgT::Array) => {
                func_arg.typ = *call_arg;
                update_callback(func_arg.name.clone());
            }
            (ArgT::Scalar, ArgT::Scalar) | (ArgT::Array, ArgT::Array) => {}
            (ArgT::Scalar, ArgT::Unknown) => {} // Reverse-prop not handled here
            (ArgT::Array, ArgT::Unknown) => {}  // Reverse-prop not handled here
            (ArgT::Unknown, ArgT::Unknown) => {} // No-op
        }
    }
    Ok(())
}
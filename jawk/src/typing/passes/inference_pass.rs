use std::rc::Rc;
use hashbrown::HashSet;
use crate::{PrintableError};
use crate::parser::{ArgT};
use crate::symbolizer::Symbol;
use crate::typing::{CallInfo, CallLink, ITypedFunction, TypedProgram};
use crate::typing::structs::CallArg;

fn get_type(program: &TypedProgram, func: &Rc<dyn ITypedFunction>, name: &Symbol) -> ArgT {
    if let Some((_idx, typ)) = func.get_arg_idx_and_type(name) {
        return typ;
    }
    if program.global_analysis.global_scalars.contains_key(name) {
        return ArgT::Scalar;
    }
    if program.global_analysis.global_arrays.contains_key(name) {
        return ArgT::Array;
    }
    ArgT::Unknown
}

fn get_types(program: &TypedProgram, link: &CallLink) -> CallInfo {
    let types = link.call.args.iter().map(
        |arg|
            match arg {
                CallArg::Variable(name) => {
                    get_type(&program, &link.source, name)
                }
                CallArg::Scalar => {
                    ArgT::Scalar
                }
            });
    types.collect()
}

fn propogate(program: &mut TypedProgram, link: &CallLink) -> Result<(HashSet<Symbol>, HashSet<Symbol>), PrintableError> {
    let caller_arg_types = get_types(program, &link);

    let dest = link.call.target.clone();
    let src = link.source.clone();

    if link.call.args.len() != dest.arity() {
        return Err(PrintableError::new(format!("Function `{}` accepts {} arguments but was called with {} from function `{}`", dest.name(), dest.arity(), link.call.args.len(), link.source.name())));
    }

    let updated_in_dest = dest.receive_call(&caller_arg_types)?;
    let updated_in_src = src.reverse_call(link, &dest.args(), &mut program.global_analysis)?;
    Ok((updated_in_dest, updated_in_src))
}

pub fn inference_pass(mut prog: TypedProgram) -> Result<TypedProgram, PrintableError> {
    let mut links: Vec<CallLink> = vec![];
    // Push every call between functions onto a stack as a link between them
    for (_name, func) in prog.functions.user_functions().iter() {
        for call in func.calls().iter() {
            links.push(CallLink { source: (*func).clone(), call: call.clone() });
        }
    };

    while let Some(link) = links.pop() {
        // While there are links left to analyze propogate any information in the source of the link to the destination
        let (updated_in_dest, updated_in_src) = propogate(&mut prog, &link)?;

        // If the destination updated any of its symbols push all of the destination's calls
        // that use those symbols back onto the stack to re-propogate
        if updated_in_dest.len() != 0 {
            for call in link.call.target.calls().iter() {
                if call.uses_any(&updated_in_dest) {
                    links.push(CallLink { source: link.call.target.clone(), call: call.clone() })
                }
            }
        }

        if updated_in_src.len() == 0 { continue; }

        // Loop through functions who call source
        for caller in link.source.callers().iter() {
            for call_to_source in caller.calls()
                .iter()
                .filter(|call_to_src| call_to_src.target.name() == link.source.name()) {
                // And push them back on the stack
                links.push(CallLink { source: caller.clone(), call: (*call_to_source).clone() })
            }
        }
    }
    Ok(prog)
}
use hashbrown::HashSet;
#[cfg(test)]
use crate::{Symbolizer};

use crate::{PrintableError};
use crate::lexer::Token::Print;
use crate::parser::{ArgT};
use crate::symbolizer::Symbol;
use crate::typing::TypedFunc;
use crate::typing::types::{TypedProgram, Call, CallArg};


pub struct CallLink {
    pub source: TypedFunc,
    pub call: Call,
}

type CallInfo = Vec<ArgT>;

fn get_type(program: &TypedProgram, func: &TypedFunc, name: &Symbol) -> ArgT {
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

pub fn variable_inference(mut prog: TypedProgram) -> Result<TypedProgram, PrintableError> {
    println!("---=-==-=--=-=-=-=-=-=-=-=-=-=-=-=");
    let mut links: Vec<CallLink> = vec![];
    // Push every call between functions onto a stack as a link between them
    for (_name, func) in &prog.functions {
        for call in func.calls().iter() {
            println!("Init, push link from {} with call {:?}", func.name(), call);
            links.push(CallLink { source: func.clone(), call: call.clone() });
        }
    };

    while let Some(link) = links.pop() {
        println!("\n\nanalyzing {} => {}", link.source.name(), link.call.target.name());
        // while there are links left to analyze
        // forward propogate any information in the source of the link to the destination
        let (updated_in_dest, updated_in_src) = propogate(&mut prog, &link)?;
        println!("Updated in dest {:?} in src {:?}", updated_in_dest, updated_in_src);

        // if the destination updated any of it's symbols push all of the destinations calls
        // that use those symbols back onto the stack to re-propogate
        if updated_in_dest.len() != 0 {
            for call in link.call.target.calls().iter() {
                if call.uses_any(&updated_in_dest) {
                    println!("Pushing link from {} to {}", link.call.target.name(), call.target.name());
                    links.push(CallLink { source: link.call.target.clone(), call: call.clone() })
                }
            }
        }

        if updated_in_src.len() == 0 { continue; }

        println!("source is called by {:?}", link.source.callers().iter().map(|t| t.name()));
        // Loop through functions who call source
        for caller in link.source.callers().iter() {
            for call_to_source in caller.calls().iter().filter(|call_to_src| call_to_src.target == link.source) {
                // And push them back on the stack
                println!("Pushing back link from {} to {}", caller.name(), call_to_source.target.name());
                links.push(CallLink { source: caller.clone(), call: call_to_source.clone() })
            }
        }
    }
    Ok(prog)
}
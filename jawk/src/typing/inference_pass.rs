#[cfg(test)]
use crate::{Symbolizer};

use crate::{PrintableError};
use crate::lexer::Token::Print;
use crate::parser::{ArgT};
use crate::symbolizer::Symbol;
use crate::typing::TypedFunc;
use crate::typing::types::{TypedProgram, Call, CallArg};


struct CallLink {
    source: TypedFunc,
    call: Call,
}

type CallInfo = Vec<ArgT>;

fn get_type(program: &TypedProgram, func: &TypedFunc, name: &Symbol) -> ArgT {
    if let Some((_idx, typ)) = func.get_arg_idx_and_type(name) {
        return typ
    }
    if program.global_analysis.global_scalars.contains_key(name) {
        return ArgT::Scalar
    }
    if program.global_analysis.global_arrays.contains_key(name) {
        return ArgT::Array
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

fn propogate(program: &mut TypedProgram, link: &CallLink) -> Result<Vec<Symbol>, PrintableError> {
    let caller_arg_types = get_types(program, &link);

    let dest = link.call.target.clone();
    let src = link.source.clone();

    if link.call.args.len() != dest.arity() {
        return Err(PrintableError::new(format!("Function `{}` accepts {} arguments but was called with {} from function `{}`", dest.name(), dest.arity(), link.call.args.len(), link.source.name())));
    }

    let updated_in_dest = dest.receive_call(caller_arg_types, &mut program.global_analysis)?;
    Ok(updated_in_dest)
    // for idx in 0..caller_arg_types.len() {
    //     let dest_arg_name = dest.func.args[idx].name.clone();
    //     dest.receive_call(caller_arg_types);
    //     if let Some(arg_type) = caller_arg_types[idx] {
    //         // Caller knows the type of this arg. Make sure it matches destination
    //         let res = match arg_type {
    //             ArgT::Scalar => dest.use_as_scalar(&dest_arg_name, &mut program.global_analysis)?,
    //             ArgT::Array => dest.use_as_array(&dest_arg_name, &mut program.global_analysis)?,
    //         };
    //         if let Some(res) = res {
    //             updated_in_dest.push(res)
    //         }
    //     }
    //
    //     //   (Source) ===>  Dest(da: Array, db: Scalar)
    //     //      dest(a,b)
    //
    //     // If destination knows the type of this arg reverse propogate it to the caller
    //     if let Some((dest_arg_idx, dest_arg_type)) = dest.get_arg_idx_and_type(&dest_arg_name) {
    //         if let Some(dest_arg_type) = dest_arg_type {
    //             let src_arg = &link.call.args[dest_arg_idx];
    //             match src_arg {
    //                 CallArg::Variable(src_arg_name) => {
    //                     let res = match dest_arg_type {
    //                         ArgT::Scalar => src.use_as_scalar(&src_arg_name, &mut program.global_analysis)?,
    //                         ArgT::Array => src.use_as_array(&src_arg_name, &mut program.global_analysis)?,
    //                     };
    //                     if let Some(res) = res {
    //                         updated_in_src.push(res);
    //                     }
    //                 },
    //                 CallArg::Scalar => {
    //                     src.use_as_scalar(&dest_arg_name, &mut program.global_analysis)?;
    //                 }
    //             }
    //         }
    //     }
    // }
    // Ok((updated_in_dest, updated_in_src))
}

pub fn variable_inference(mut prog: TypedProgram) -> Result<TypedProgram, PrintableError> {
    let mut links: Vec<CallLink> = vec![];
    // Push every call between functions onto a stack as a link between them
    for (_name, func) in &prog.functions {
        for call in func.calls().iter() {
            links.push(CallLink { source: func.clone(), call: call.clone() });
        }
    };

    while let Some(link) = links.pop() {
        // while there are links left to analyze
        // forward propogate any information in the source of the link to the destination
        let updated_in_dest = propogate(&mut prog, &link)?;

        // if the destination updated any of it's symbols push all of the destinations calls
        // that use those symbols back onto the stack to re-propogate
        if updated_in_dest.len() != 0 {
            for call in link.call.target.calls().iter() {
                if call.uses_any(&updated_in_dest) {
                    links.push(CallLink { source: link.call.target.clone(), call: call.clone() })
                }
            }
        }

        // let source_func_callers: Vec<Symbol> = {
        //     prog.functions.get_mut(&link.source).unwrap().callers.iter().cloned().collect()
        // };/
        // for caller in source_func_callers {
        //     let caller_func = prog.functions.get(&caller).unwrap();
        //     for call_to_source in caller_func.calls.iter().filter(|call| call.target == link.source) {
        //         links.push(CallLink { source: caller.clone(), call: call_to_source.clone() })
        //     }
        // }
    }
    Ok(prog)
}


#[cfg(test)]
fn fully_typed_prog(prog: &str) -> (TypedProgram, Symbolizer) {
    let res = function_pass_only_prog(prog);
    (variable_inference(res.0).unwrap(), res.1)
}

#[cfg(test)]
fn function_pass_only_prog(prog: &str) -> (TypedProgram, Symbolizer) {
    use crate::{lex, parse};
    let mut symbolizer = Symbolizer::new();
    let fa = crate::typing::function_pass::FunctionAnalysis::new();
    let prog = fa.analyze_program(parse(lex(prog,
                                            &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
    (prog, symbolizer)
}


#[test]
fn test_calls_forward_inference() {
    let (prog, mut sym) = function_pass_only_prog("function helper(arg) { return 1 } BEGIN { a[0] = 1; helper(a) }");
    let main = prog.functions.get(&sym.get("main function")).unwrap();
    let helper = prog.functions.get(&sym.get("helper")).unwrap();
    assert_eq!(main.calls().len(), 1);
    assert_eq!(main.calls().clone(), vec![Call::new(helper.clone(), vec![CallArg::new(sym.get("a"))])]);
}

#[test]
fn test_forward_inference() {
    /*
     fn main() {
        a[0] = 1;
        helper(a);
     }

     // infer arg is array
     fn helper(arg) {
        ....
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("function helper(arg) { return 1 } BEGIN { a[0] = 1; helper(a) }");
    let helper = symbolizer.get("helper");
    assert_eq!(prog.functions.len(), 2);
    assert_eq!(prog.functions.get(&helper).unwrap().args().len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().args()[0].typ, ArgT::Array);
}

#[test]
fn test_branching_forward_inference() {
    let (prog, mut symbolizer) = fully_typed_prog("\
    function helper1(arg1, arg2) { return 1; }
    BEGIN { a[0] = 1; helper1(5, a);  }
    ");
    let helper1 = symbolizer.get("helper1");
    assert_eq!(prog.functions.get(&helper1).unwrap().args()[0].typ, ArgT::Scalar);
    assert_eq!(prog.functions.get(&helper1).unwrap().args()[1].typ, ArgT::Array);
}

#[test]
fn test_recursive_inference() {
    /*
     fn main() {
        a = 1;
        helper(a);
     }

     // infer arg is scalar and terminate
     fn helper(arg) {
        helper(arg);
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("function helper(arg) { helper(1); } BEGIN { a = 1; helper(a) }");
    let helper = symbolizer.get("helper");
    assert_eq!(prog.functions.len(), 2);
    assert_eq!(prog.functions.get(&helper).unwrap().args().len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().args()[0].typ, ArgT::Scalar);
}

#[test]
fn test_calls_rev_inference() {
    let (prog, mut sym) = function_pass_only_prog("function helper(arg) { arg[0] = 1 } BEGIN { helper(a) }");
    let main = prog.functions.get(&sym.get("main function")).unwrap();
    let helper = prog.functions.get(&sym.get("helper")).unwrap();
    assert_eq!(main.calls().clone(), vec![Call::new(helper.clone(), vec![CallArg::new(sym.get("a"))])]);
    assert_eq!(helper.args()[0].typ, ArgT::Array);
}

#[test]
fn test_rev_inference() {
    /*
     fn main() {
        helper(a); // infer global a is an array
     }

     fn helper(arg) {
        arg[0] = 1;
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("function helper(arg) { arg[0] = 1 } BEGIN { helper(a) }");
    let a = symbolizer.get("a");
    let helper = symbolizer.get("helper");
    assert_eq!(prog.functions.len(), 2);
    assert_eq!(prog.functions.get(&helper).unwrap().args().len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().args()[0].typ, ArgT::Array);
    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains_key(&a));
}


#[test]
fn test_calls_chain_inference() {
    let (prog, mut sym) = function_pass_only_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { return 1; }\
        BEGIN { a[0] = 1; helper1(a) }");
    let main = prog.functions.get(&sym.get("main function")).unwrap();
    let helper1 = prog.functions.get(&sym.get("helper1")).unwrap();
    let helper2 = prog.functions.get(&sym.get("helper2")).unwrap();
    assert_eq!(main.calls().clone(), vec![Call::new(helper1.clone(), vec![CallArg::new(sym.get("a"))])]);
    assert_eq!(helper1.calls().clone(), vec![Call::new(helper2.clone(), vec![CallArg::new(sym.get("arg1"))])]);
    assert_eq!(helper2.args()[0].name, sym.get("arg2"));
    assert_eq!(helper1.args()[0].typ, ArgT::Unknown);
}

#[test]
fn test_forward_chained_inference_array() {
    /*
     fn main() {
        a[0] = 1; // global a is array (prior pass)
        helper1(a);
     }

     fn helper1(arg1) {  // infer arg1 is array
        helper2(arg1)
     }

     fn helper2(arg2) { // arg2 is array
         return 1;
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { return 1; }\
        BEGIN { a[0] = 1; helper1(a) }");
    let helper1 = symbolizer.get("helper1");
    let helper2 = symbolizer.get("helper2");
    let a = symbolizer.get("a");
    assert_eq!(prog.functions.len(), 3);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.args()[0].typ, ArgT::Array);

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.args()[0].typ, ArgT::Array);

    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains_key(&a));
}


#[test]
fn test_rev_chained_inference_array() {
    /*
     fn main() {
        helper1(a); // infer global a is array
     }

     fn helper1(arg1) {  // infer arg1 is array
        helper2(arg1)
     }

     fn helper2(arg2) { // arg2 is array (prior pass)
         arg2[0] = 1;
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { arg2[0] = 1; }\
        BEGIN { helper1(a) }");
    let a = symbolizer.get("a");
    let helper1 = symbolizer.get("helper1");
    let helper2 = symbolizer.get("helper2");

    assert_eq!(prog.functions.len(), 3);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.args()[0].typ, ArgT::Array);

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.args()[0].typ, ArgT::Array);

    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains_key(&a));
}

#[test]
fn test_rev_chained_inference_scalar() {
    /*
     fn main() {
        helper1(a); // infer global a is scalar
     }

     fn helper1(arg1) {  // infer arg1 is scalar
        helper2(arg1)
     }

     fn helper2(arg2) { // arg2 is scalar (prior pass)
         arg2[0] = 1;
     }
     */
    let (prog, mut symbolizer) = fully_typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { arg2++; }\
        BEGIN { helper1(a) }");
    let a = symbolizer.get("a");
    let helper1 = symbolizer.get("helper1");
    let helper2 = symbolizer.get("helper2");

    assert_eq!(prog.functions.len(), 3);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.args()[0].typ, ArgT::Scalar);

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.args()[0].typ, ArgT::Scalar);

    assert!(!prog.global_analysis.global_arrays.contains_key(&a));
    assert!(prog.global_analysis.global_scalars.contains_key(&a));
}
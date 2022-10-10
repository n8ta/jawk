use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use libc::link;
use crate::{PrintableError, Symbolizer};
use crate::parser::{Arg, ArgT, Function, Program, ScalarType, Stmt, TypedExpr, Expr};
use crate::parser::Stmt::Print;
use crate::symbolizer::Symbol;
use crate::typing::TypedFunc;
use crate::typing::types::{TypedProgram, AnalysisResults, MapT, Call, CallArg};


fn get_arg(func: &TypedFunc, name: &Symbol) -> Option<ArgT> {
    if let Some(arg) = func.func.args.iter().find(|a| a.name == *name) {
        arg.typ
    } else {
        None
    }
}

struct CallLink {
    source: Symbol,
    call: Call,
}

type CallInfo = Vec<Option<ArgT>>;

fn get_type(program: &TypedProgram, function: &Symbol, name: &Symbol) -> Option<ArgT> {
    let func = program.functions.get(function).unwrap();
    if let Some(typ) = get_arg(func, name) {
        return Some(typ);
    }
    if program.global_analysis.global_scalars.contains(name) {
        return Some(ArgT::Scalar);
    }
    if program.global_analysis.global_arrays.contains_key(name) {
        return Some(ArgT::Array);
    }
    None
}

fn get_types(program: &TypedProgram, link: &CallLink) -> CallInfo {
    let types = link.call.args.iter().map(
        |arg|
            match arg {
                CallArg::Variable(name) => {
                    get_type(&program, &link.source, name)
                }
                CallArg::Scalar => {
                    Some(ArgT::Scalar)
                }
            });
    types.collect()
}

fn forward_prop(program: &mut TypedProgram, link: &CallLink) -> Result<Vec<Symbol>, PrintableError> {
    let types = get_types(program, &link);
    let dest = program.functions.get_mut(&link.call.target).expect(&format!("function {} to exist", link.call.target));
    let mut updated_symbols_in_dest = vec![];
    for idx in 0..types.len() {
        let arg_name = dest.func.args[idx].name.clone();

        if let Some(arg_type) = types[idx] {
            let res = match arg_type {
                ArgT::Scalar => dest.use_as_scalar(&arg_name, &mut program.global_analysis)?,
                ArgT::Array => dest.use_as_array(&arg_name, &mut program.global_analysis)?,
            };
            if let Some(res) = res {
                updated_symbols_in_dest.push(res)
            }
        } else {
            // TODO: Reverse prop!
        }
    }
    Ok(updated_symbols_in_dest)
}

pub fn variable_inference(mut prog: TypedProgram) -> Result<TypedProgram, PrintableError> {
    let mut links: Vec<CallLink> = vec![];
    // Push every call between functions onto a stack as a link between them
    for (func_name, func) in &prog.functions {
        for call in &func.calls {
            links.push(CallLink { source: func_name.clone(), call: call.clone() });
        }
    };

    loop {
        // while there are links left to analyze
        if let Some(link) = links.pop() {
            // forward propogate any information in the source of the link to the destination
            let updated_syms_in_dest = forward_prop(&mut prog, &link)?;

            // if the destination updated any of it's symbols push all of the destinations calls
            // that use those symbols back onto the stack to re-propogate
            if updated_syms_in_dest.len() == 0 { continue; }

            for call in &prog.functions.get(&link.call.target).unwrap().calls {
                if call.uses_any(&updated_syms_in_dest) {
                    links.push(CallLink { source: link.call.target.clone(), call: call.clone() })
                }
            }
        } else {
            // No more links we are done
            break;
        }
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
    let mut prog = fa.analyze_program(parse(lex(prog,
                                                &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
    (prog, symbolizer)
}


#[test]
fn test_calls_forward_inference() {
    let (prog, mut sym) = function_pass_only_prog("function helper(arg) { return 1 } BEGIN { a[0] = 1; helper(a) }");
    let main = prog.functions.get(&sym.get("main function")).unwrap();
    assert_eq!(main.calls.len(), 1);
    assert_eq!(main.calls, vec![Call::new(sym.get("helper"), vec![CallArg::new(sym.get("a"))])]);
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
    assert_eq!(prog.functions.get(&helper).unwrap().func.args.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args[0].typ, Some(ArgT::Array));
}

#[test]
fn test_branching_forward_inference() {
    // let (prog, mut symbolizer) = fully_typed_prog("\
    // function helper1(arg1, arg2) { helper2(arg2, arg1); return 1; }
    // function helper2(arg1, arg2) { helper3(arg2, arg1); helper4(arg2); helper5(arg2); return 2; }
    // function helper3(arg31, arg32) { return 3 }
    // function helper4(arg) { return 4 }
    // function helper5(arg) { return 5 }
    // function helper6(arg) { return 6 }
    // BEGIN { a[0] = 1; helper1(5, a); helper6(a); }
    // ");
    let (prog, mut symbolizer) = fully_typed_prog("\
    function helper1(arg1, arg2) { return 1; }
    BEGIN { a[0] = 1; helper1(5, a);  }
    ");
    let helper1 = symbolizer.get("helper1");
    assert_eq!(prog.functions.get(&helper1).unwrap().func.args[0].typ, Some(ArgT::Scalar));
    assert_eq!(prog.functions.get(&helper1).unwrap().func.args[1].typ, Some(ArgT::Array));
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
    assert_eq!(prog.functions.get(&helper).unwrap().func.args.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args[0].typ, Some(ArgT::Scalar));
}

#[test]
fn test_calls_rev_inference() {
    let (prog, mut sym) = function_pass_only_prog("function helper(arg) { arg[0] = 1 } BEGIN { helper(a) }");
    let main = prog.functions.get(&sym.get("main function")).unwrap();
    let helper = prog.functions.get(&sym.get("helper")).unwrap();
    assert_eq!(main.calls, vec![Call::new(sym.get("helper"), vec![CallArg::new(sym.get("a"))])]);
    assert_eq!(helper.func.args[0].typ, Some(ArgT::Array));
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
    assert_eq!(prog.functions.get(&helper).unwrap().func.args.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args[0].typ, Some(ArgT::Array));
    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains(&helper));
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
    assert_eq!(main.calls, vec![Call::new(sym.get("helper1"), vec![CallArg::new(sym.get("a"))])]);
    assert_eq!(helper1.calls, vec![Call::new(sym.get("helper2"), vec![CallArg::new(sym.get("arg1"))])]);
    assert_eq!(helper2.func.args[0], Arg::new(sym.get("arg2"), None));
    assert_eq!(helper1.func.args[0].typ, None);
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
    assert_eq!(helper1.func.args[0].typ, Some(ArgT::Array));

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.func.args[0].typ, Some(ArgT::Array));

    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains(&a));
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

    assert_eq!(prog.functions.len(), 2);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.func.args[0].typ, Some(ArgT::Array));

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.func.args[0].typ, Some(ArgT::Array));

    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains(&a));
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

    assert_eq!(prog.functions.len(), 2);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.func.args[0].typ, Some(ArgT::Scalar));

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.func.args[0].typ, Some(ArgT::Scalar));

    assert!(!prog.global_analysis.global_arrays.contains_key(&a));
    assert!(prog.global_analysis.global_scalars.contains(&a));
}
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use crate::{PrintableError, Symbolizer};
use crate::parser::{Arg, ArgT, Function, Program, ScalarType, Stmt, TypedExpr, Expr};
use crate::symbolizer::Symbol;
use crate::typing::TypedFunc;
use crate::typing::types::{TypedProgram, AnalysisResults, MapT, Call, CallArg};

pub fn variable_inference(prog: TypedProgram) -> TypedProgram {

    prog
}


#[cfg(test)]
fn typed_prog(prog: &str) -> (TypedProgram, Symbolizer) {
    use crate::{lex, parse};
    let mut symbolizer = Symbolizer::new();
    let fa = crate::typing::function_pass::FunctionAnalysis::new();
    let mut prog = fa.analyze_program(parse(lex("function helper(arg) { arg[0] = 1 } BEGIN { helper(d) }",
                                                &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
    (variable_inference(prog), symbolizer)
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
    let (prog, mut symbolizer) = typed_prog("function helper(arg) { return 1 } BEGIN { a[0] = 1; helper(a) }");
    let helper = symbolizer.get_symbol("helper");
    assert_eq!(prog.functions.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args[0].typ, Some(ArgT::Array));
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
    let (prog, mut symbolizer) = typed_prog("function helper(arg) { arg[0] = 1 } BEGIN { helper(a) }");
    let  a = symbolizer.get_symbol("a");
    let  helper = symbolizer.get_symbol("helper");
    assert_eq!(prog.functions.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args.len(), 1);
    assert_eq!(prog.functions.get(&helper).unwrap().func.args[0].typ, Some(ArgT::Array));
    assert!(prog.global_analysis.global_arrays.contains_key(&a));
    assert!(!prog.global_analysis.global_scalars.contains(&helper));
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
    let (prog, mut symbolizer) = typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { return 1; }\
        BEGIN { a[0] = 1; helper1(a) }");
    let helper1 = symbolizer.get_symbol("helper1");
    let helper2 = symbolizer.get_symbol("helper2");
    let a = symbolizer.get_symbol("a");
    assert_eq!(prog.functions.len(), 2);

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
    let (prog, mut symbolizer) = typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { arg2[0] = 1; }\
        BEGIN { helper1(a) }");
    let a = symbolizer.get_symbol("a");
    let helper1 = symbolizer.get_symbol("helper1");
    let helper2 = symbolizer.get_symbol("helper2");

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
    let (prog, mut symbolizer) = typed_prog("\
        function helper1(arg1) { return helper2(arg1) }\
        function helper2(arg2) { arg2++; }\
        BEGIN { helper1(a) }");
    let a = symbolizer.get_symbol("a");
    let helper1 = symbolizer.get_symbol("helper1");
    let helper2 = symbolizer.get_symbol("helper2");

    assert_eq!(prog.functions.len(), 2);

    let helper1 = prog.functions.iter().find(|f| *f.0 == helper1).unwrap().1;
    assert_eq!(helper1.func.args[0].typ, Some(ArgT::Scalar));

    let helper2 = prog.functions.iter().find(|f| *f.0 == helper2).unwrap().1;
    assert_eq!(helper2.func.args[0].typ, Some(ArgT::Scalar));

    assert!(!prog.global_analysis.global_arrays.contains_key(&a));
    assert!(prog.global_analysis.global_scalars.contains(&a));
}
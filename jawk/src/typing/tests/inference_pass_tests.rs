#[cfg(test)]
mod inference_tests {
    use crate::parser::ArgT;
    use crate::Symbolizer;
    use crate::typing::{function_pass, inference_pass};
    use crate::typing::{ITypedFunction, TypedProgram};
    use crate::typing::structs::{Call, CallArg};

    fn fully_typed_prog(prog: &str) -> (TypedProgram, Symbolizer) {
        let res = function_pass_only_prog(prog);
        (inference_pass(res.0).unwrap(), res.1)
    }

    fn function_pass_only_prog(prog: &str) -> (TypedProgram, Symbolizer) {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let prog = function_pass(parse(lex(prog, &mut symbolizer).unwrap(), &mut symbolizer).unwrap()).unwrap();
        (prog, symbolizer)
    }


    #[test]
    fn test_callers_setup_correctly() {
        let (prog, mut symbolizer) = fully_typed_prog("\
        function helper1(arg1) { 1; }\
        BEGIN { helper1(a) }");
        let helper1 = symbolizer.get("helper1");
        let func = prog.functions.get(&helper1).unwrap();
        let callers = func.callers();
        assert_eq!(callers.len(), 1)
    }

    #[test]
    fn test_calls_forward_inference() {
        let (prog, mut sym) = function_pass_only_prog("function helper(arg) { return 1 } BEGIN { a[0] = 1; helper(a) }");
        let main = prog.functions.get(&sym.get("main function")).unwrap();
        let helper = prog.functions.get(&sym.get("helper")).unwrap();
        assert_eq!(main.calls().len(), 1);
        assert_eq!(main.calls().clone(), vec![Call::new(main.clone(),helper, vec![CallArg::new(sym.get("a"))])]);
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
        assert_eq!(main.calls().clone(), vec![Call::new(main.clone(), helper.clone(), vec![CallArg::new(sym.get("a"))])]);
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
        assert_eq!(main.calls().clone(), vec![Call::new(main.clone(), helper1.clone(), vec![CallArg::new(sym.get("a"))])]);
        assert_eq!(helper1.calls().clone(), vec![Call::new(helper1.clone(), helper2.clone(), vec![CallArg::new(sym.get("arg1"))])]);
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
}
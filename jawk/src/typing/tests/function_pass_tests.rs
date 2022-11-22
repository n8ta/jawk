#[cfg(test)]
mod tests {
    use crate::{analyze, Symbolizer};

    fn test_exception(program: &str, error_includes_msg: &str) {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let ast_result = analyze(parse(lex(program, &mut symbolizer).unwrap(), &mut symbolizer));
        if let Err(err) = ast_result {
            println!("Error msg: `{}\nShould include: `{}`", err.msg, error_includes_msg);
            assert!(err.msg.contains(error_includes_msg));
        } else {
            assert!(false, "type check should have failed with {}", error_includes_msg)
        }
    }

    fn strip(data: &str) -> String {
        let data: String = data.replace("\n", "")
            .replace(" ", "")
            .replace("\t", "")
            .replace(";", "")
            .replace("\n", "");
        println!("pre_strip: {}", data);
        if let Some(rest) = data.strip_prefix("functionmainfunction(){") {
            return rest.strip_suffix("}").unwrap().to_string();
        }
        data
    }

    #[cfg(test)]
    fn test_it(program: &str, expected: &str) {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let ast = analyze(parse(lex(program, &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
        println!("prog: {}", ast);
        let result_clean = strip(&format!("{}", ast));
        let expected_clean = strip(expected);
        if result_clean != expected_clean {
            println!("Got: \n{}", format!("{}", ast));
            println!("Expected: \n{}", expected);
        }
        assert_eq!(result_clean, expected_clean);
    }

    #[cfg(test)]
    fn test_it_funcs(program: &str, expected: &str) {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let ast = analyze(parse(lex(program, &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
        let result_clean = strip(&format!("{}", ast));
        let expected_clean = strip(expected);
        assert_eq!(result_clean, expected_clean);
    }

    #[test]
    fn test_typing_basic() {
        test_it("BEGIN { print \"a\" }", "print (s \"a\")");
    }

    #[test]
    fn test_typing_basic2() {
        test_it("BEGIN { print 123 }", "print (f 123)");
    }

    #[test]
    fn test_if_basic() {
        test_it(
            "BEGIN { a = 1; print a; if($1) { print a } } ",
            "(f a = (f 1)); print (f a); if (s $(f 1)) { print (f a) }",
        );
    }

    #[test]
    fn test_typed_loop() {
        test_it("BEGIN \
               { while (1) { x=1; } print x; }",
                "while(f 1){ (f x = (f1)); }print(vx)");
    }

    #[test]
    fn test_if_polluting() {
        test_it(
            "BEGIN { a = 1; print a; if($1) { a = \"a\"; } print a; print a;    } ",
            "(f a = (f 1)); print (f a); if (s $(f 1)) { (s a = (s \"a\")); } print (v a); print (v a)",
        );
    }

    #[test]
    fn test_if_nonpolluting() {
        test_it(
            "BEGIN { a = 1; print a; if($1) { a = 5; } print a; } ",
            "(f a = (f 1)); print (f a); if (s $(f 1)) { (f a = (f 5)); } print (f a);",
        );
    }

    #[test]
    fn test_ifelse_polluting() {
        test_it("BEGIN { a = 1; print a; if($1) { a = 5; } else { a = \"a\" } print a; } ",
                "(f a = (f 1)); print (f a); if (s $(f 1)) { (f a = (f 5)); } else { (s a = (s \"a\")) } print (v a);");
    }

    #[test]
    fn test_ifelse_swapping() {
        test_it("BEGIN { a = 1; print a; if($1) { a = \"a\"; } else { a = \"a\" } print a; } ",
                "(f a = (f 1)); print (f a); if (s $(f 1)) { (s a = (s \"a\")); } else { (s a = (s \"a\")) } print (s a);");
    }

    #[test]
    fn test_ifelse_swapping_2() {
        test_it("BEGIN { a = \"a\"; print a; if($1) { a = 3; } else { a = 4 } print a; } ",
                "(s a = (s \"a\")); print (s a); if (s $(f 1)) { (f a = ( f 3)); } else { (f a = (f 4)) } print (f a);");
    }

    #[test]
    fn test_if_else_polluting() {
        test_it("BEGIN { a = 1; print a; if($1) { a = \"a\"; } else { a = \"a\" } print a; } ",
                "(f a = (f 1)); print (f a); if (s $(f 1)) { (s a = (s \"a\"); ) } else { (s a = (s \"a\")); } print (s a)");
    }

    #[test]
    fn test_concat_loop() {
        test_it(
            "{ a = a $1 } END { print a; }",
            "while (f check_if_there_is_another_line) { (s a = (s (v a) (s$(f 1)))) }; print (v a);",
        );
    }

    #[test]
    fn test_while_break_typing() {
        test_it("BEGIN { while (1) { if (x == 33) { break } x = x + 1; } print x; }",
                "while (f1) { if (f(vx) == (f33)) { break } (f x = (f ( v x) + (f 1 ))) } print (v x)",
        )
    }

    #[test]
    fn test_while_break_known_type() {
        test_it("BEGIN { x = 5; while (1) { if (x == 33) { break } x = x + 1; } print x; }",
                "(f x  = (f 5)); while (f1) { if (f(fx) == (f33)) { break } (f x = (f ( f x) + (f 1 ))) } print (f x)",
        )
    }

    #[test]
    fn test_typing_while_x_uninit_1() {
        test_it("BEGIN { while ( (x=x+1) < 1) { }}",
                "while (f(f x = (f (v x) + (f 1))) < (f1)) {}")
    }


    #[test]
    fn test_typing_while_x_uninit_2() {
        test_it("BEGIN { while ( x < 1) { }}",
                "while (f (v x) < (f 1) ) {}")
    }

    #[test]
    fn test_typing_while_x_uninit_3() {
        test_it("BEGIN { x = x + 1; }",
                "(f x = (f (v x) + (f 1)))")
    }

    #[test]
    fn test_while_loop() {
        test_it(
            "BEGIN { while(123) { a = \"bb\"}; print a;}",
            "while (f 123) { (s a = (s \"bb\")) }; print (v a);",
        );
    }

    #[test]
    fn test_assignment() {
        test_it("BEGIN { x = 0; print x; }", "(f x = (f 0 )); print (f x);");
    }

    #[test]
    fn test_assignment_col() {
        test_it(
            "{ x = $0; } END { print x; }",
            "while(fcheck_if_there_is_another_line){ (s x = (s$(f 0) ))}; print (v x);",
        );
    }


    #[test]
    fn test_ternary() {
        test_it("\
    BEGIN { x = \"a\"; x ? (x=1) : (x=2); print x; }",
                "(s x = (s \"a\")); \n(f (s x) ? (f x = (f 1)) : (f x = (f 2))); \nprint (f x)");
    }

    #[test]
    fn test_ternary_2() {
        test_it("\
    BEGIN { x = \"a\"; x ? (x=1) : (x=\"a\"); print x; }",
                "(s x = (s \"a\")); \n(v (s x) ? (f x = (f 1)) : (s x = (s \"a\"))); \nprint (v x)");
    }

    #[test]
    fn test_ternary_3() {
        test_it("\
    BEGIN { x ? (x=1) : (x=\"a\"); print x; }",
                "(v (v x) ? (f x = (f 1)) : (s x = (s \"a\"))); \nprint (v x)");
    }

    #[test]
    fn test_ternary_4() {
        test_it("\
    BEGIN { x ? (x=1) : (x=4); print x; }",
                "(f (v x) ? (f x = (f 1)) : (f x = (f 4)));\nprint (f x)");
    }

    #[test]
    fn test_fails() {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let res = analyze(parse(lex("BEGIN { a = 0; a[0] = 1; }", &mut symbolizer).unwrap(), &mut symbolizer));
        assert!(res.is_err());
    }

    #[test]
    fn test_fails_2() {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let ast = analyze(parse(lex("BEGIN { a[0] = 1; a = 0;  }", &mut symbolizer).unwrap(), &mut symbolizer));
        assert!(ast.is_err());
    }

    #[test]
    fn test_fails_3() {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        let ast = analyze(parse(lex("BEGIN { if(x) { a[0] = 1; } a = 0;  }", &mut symbolizer).unwrap(), &mut symbolizer));
        assert!(ast.is_err());
    }

    #[test]
    fn test_calls() {}

    #[test]
    fn test_typing_scalar_function() {
        test_it_funcs("function a() { return 1; } BEGIN { print 1; }",
                      "function a() { return (f 1); } function mainfunction() { print (f 1) }");
    }

    #[test]
    fn test_arr_typing() {
        test_it("BEGIN { b[0] = d; }",
                "(v b[(f 0)] = (v d))");
    }

    #[test]
    fn test_typing_array_fails_mixed_ret() {
        test_exception("function a(arg) { if(arg) { return 1; } b[0] = 2; return b } BEGIN { print 0; }", "attempted to use")
    }

    #[test]
    fn test_typing_array_fails_no_ret() {
        test_exception("function a(arg) { if(arg) { b[0] = 1; return b; } } BEGIN { print 0; }", "attempt to use")
    }

    #[test]
    fn mixed_func_array() {
        test_exception("function a() { } BEGIN { a[0] = 1; }", "attempt to use")
    }

    #[test]
    fn mixed_func_scalar() {
        test_exception("function a() { } BEGIN { a = 1; }", "attempt to use")
    }

    #[test]
    fn mixed_scalar_array() {
        test_exception("BEGIN { a[0] = 1; a = 5; }", "attempt to use")
    }
}
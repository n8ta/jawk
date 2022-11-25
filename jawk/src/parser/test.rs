mod parser_tests {
    use crate::parser::{Program, parse, PatternAction, Stmt, TypedExpr, Expr, Function};
    use crate::lexer::{MathOp, BinOp, LogicalOp};
    use crate::lexer::Token;

    use crate::symbolizer::Symbolizer;
    use crate::lexer::lex;


    macro_rules! num {
    ($value:expr) => {
        texpr!(Expr::NumberF64($value))
    };}

    macro_rules! bnum {
    ($value:expr) => {
        Box::new(texpr!(Expr::NumberF64($value)))
    };}

    macro_rules! btexpr {
    ($value:expr) => {
        Box::new(texpr!($value))
    };}

    macro_rules! texpr {
    ($value:expr) => {
        TypedExpr::new($value)
    };}

    macro_rules! mathop {
    ($a:expr, $op:expr, $b:expr) => {
        texpr!(Expr::MathOp($a, $op, $b))
    };}

    macro_rules! binop {
    ($a:expr, $op:expr, $b:expr) => {
        texpr!(Expr::BinOp($a, $op, $b))
    };}

    macro_rules! sprogram {
    ($body:expr, $symbolizer:expr) => {
        Program::new($symbolizer.get("main function"),vec![], vec![], vec![PatternAction::new_action_only($body)], vec![])
    };}

    macro_rules! actual {
    ($name:ident, $body:expr, $symbolizer:ident) => {
        use crate::lexer::lex;
        use crate::symbolizer::Symbolizer;
        let mut $symbolizer = Symbolizer::new();
        let $name = parse_unwrap(lex($body, &mut $symbolizer).unwrap(), &mut $symbolizer);
    };}

    pub fn parse_unwrap(tokens: Vec<Token>, symbolizer: &mut Symbolizer) -> Program {
        parse(tokens, symbolizer).unwrap()
    }

    fn parse_it(input: &str, symbolizer: &mut Symbolizer) -> Program {
        parse_unwrap(lex(input, symbolizer).unwrap(), symbolizer)
    }

    #[test]
    fn test_ast_number() {
        let mut symbolizer = Symbolizer::new();

        let prog = Program::new(
            symbolizer.get("main function"), vec![], vec![], vec![PatternAction::new_action_only(Stmt::Expr(mathop!(
                        bnum!(1.0),
                        MathOp::Plus,
                        bnum!(2.0)
                    )))], vec![]);
        assert_eq!(
            parse_it("{1+2}", &mut symbolizer),
            prog
        );
    }


    #[test]
    fn test_ast_oop() {
        use crate::lexer::lex;
        let mut symbolizer = Symbolizer::new();
        let left = bnum!(1.0);
        let right = Box::new(mathop!(bnum!(3.0), MathOp::Star, bnum!(2.0)));
        let expected = Program::new_action_only(symbolizer.get("main function"), Stmt::Expr(mathop!(left, MathOp::Plus, right)));
        let actual = parse_it("{1 + 3 * 2;}", &mut symbolizer);
        assert_eq!(
            actual,
            expected, "\nactual {} expected {}", actual, expected
        );
    }

    #[test]
    fn test_ast_oop_2() {
        let mut symbolizer = Symbolizer::new();
        let left = Box::new(num!(2.0));
        let right = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(1.0)),
        MathOp::Star,
        Box::new(num!(3.0))
    )));
        let mult = Stmt::Expr(texpr!(Expr::MathOp(right, MathOp::Plus, left)));
        assert_eq!(
            parse_it("{1 * 3 + 2;}", &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), mult)
        );
    }

    #[test]
    fn test_ast_assign() {
        let mut symbolizer = Symbolizer::new();
        let stmt = Stmt::Expr(texpr!(Expr::ScalarAssign(symbolizer.get("abc"), bnum!(2.0))));
        assert_eq!(
            parse_unwrap(lex("{abc = 2.0; }", &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), stmt)
        );
    }

    #[test]
    fn test_mathop_exponent() {
        let mut symbolizer = Symbolizer::new();

        assert_eq!(
            parse_unwrap(lex("{2 ^ 2;}", &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new(
                symbolizer.get("main function"),
                vec![],
                vec![],
                vec![PatternAction::new_action_only(Stmt::Expr(mathop!(
                bnum!(2.0),
                MathOp::Exponent,
                bnum!(2.0)
            )))], vec![],
            )
        );
    }

    #[test]
    fn test_mathop_exponent_2() {
        let mut symbolizer = Symbolizer::new();
        let right = Box::new(num!(3.0));
        let left = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(2.0)),
        MathOp::Exponent,
        Box::new(num!(2.0))
    )));
        let expo = Stmt::Expr(texpr!(Expr::MathOp(left, MathOp::Star, right)));

        assert_eq!(
            parse_unwrap(lex("{2 ^ 2 * 3;}", &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), expo)
        );
    }

    #[test]
    fn test_unary_op() {
        let mut symbolizer = Symbolizer::new();
        let initial = Box::new(num!(1.0));
        let first = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Plus,
        initial
    )));
        let second = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Minus,
        first
    )));
        let third = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Plus,
        second
    )));

        let fourth = Stmt::Expr(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Minus,
        third
    )));

        assert_eq!(
            parse_unwrap(lex("{-+-+1;}", &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), fourth)
        );
    }

    #[test]
    fn test_unary_op2() {
        let mut symbolizer = Symbolizer::new();
        let initial = Box::new(num!(1.0));
        let first = Box::new(texpr!(Expr::BinOp(
        Box::new(num!(1.0)),
        BinOp::BangEq,
        initial
    )));
        let second = Box::new(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Plus,
        first
    )));
        let third = Box::new(texpr!(Expr::BinOp(
        Box::new(num!(1.0)),
        BinOp::BangEq,
        second
    )));

        let fourth = Stmt::Expr(texpr!(Expr::MathOp(
        Box::new(num!(0.0)),
        MathOp::Minus,
        third
    )));

        let expected = parse_unwrap(lex("{-!+!1;}", &mut symbolizer).unwrap(), &mut symbolizer);
        let actual = Program::new_action_only(symbolizer.get("main function"), fourth);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_if_else() {
        let mut symbolizer = Symbolizer::new();
        let str = "{ if (1) { print 2; } else { print 3; }}";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        assert_eq!(
            actual,
            Program::new_action_only(
                symbolizer.get("main function"), Stmt::If(
                    num!(1.0),
                    Box::new(Stmt::Print(num!(2.0))),
                    Some(Box::new(Stmt::Print(num!(3.0)))),
                ))
        );
    }

    #[test]
    fn test_if_only() {
        let mut symbolizer = Symbolizer::new();
        let str = "{if (1) { print 2; }}";
        assert_eq!(
            parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), Stmt::If(num!(1.0), Box::new(Stmt::Print(num!(2.0))), None))
        );
    }

    #[test]
    fn test_print() {
        let mut symbolizer = Symbolizer::new();
        let str = "{print 1;}";
        assert_eq!(
            parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(symbolizer.get("main function"), Stmt::Print(num!(1.0)))
        );
    }

    #[test]
    fn test_group() {
        let mut symbolizer = Symbolizer::new();
        let str = "{{print 1; print 2;}}";
        assert_eq!(
            parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer),
            Program::new_action_only(
                symbolizer.get("main function"), Stmt::Group(vec![
                    Stmt::Print(num!(1.0)),
                    Stmt::Print(num!(2.0)),
                ]))
        );
    }

    #[test]
    fn test_if_else_continues() {
        let mut symbolizer = Symbolizer::new();
        let str = "{if (1) { print 2; } else { print 3; } 4.0;}";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        assert_eq!(
            actual,
            Program::new_action_only(
                symbolizer.get("main function"), Stmt::Group(vec![
                    Stmt::If(
                        num!(1.0),
                        Box::new(Stmt::Print(num!(2.0))),
                        Some(Box::new(Stmt::Print(num!(3.0)))),
                    ),
                    Stmt::Expr(num!(4.0)),
                ]))
        );
    }

    #[test]
    fn test_paser_begin_end() {
        let mut symbolizer = Symbolizer::new();
        let a = symbolizer.get("a");
        let str =
            "a { print 5; } BEGIN { print 1; } begin { print 2; } END { print 3; } end { print 4; }";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        let begins = vec![Stmt::Print(num!(1.0)), Stmt::Print(num!(2.0))];
        let ends = vec![Stmt::Print(num!(3.0)), Stmt::Print(num!(4.0))];
        let generic = PatternAction::new(
            Some(texpr!(Expr::Variable(a))),
            Stmt::Print(num!(5.0)),
        );
        assert_eq!(actual, Program::new(symbolizer.get("main function"), begins, ends, vec![generic], vec![]));
    }

    #[test]
    fn test_pattern_only() {
        let mut symbolizer = Symbolizer::new();
        let str = "test";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        assert_eq!(
            actual,
            Program::new(
                symbolizer.get("main function"),
                vec![],
                vec![],
                vec![PatternAction::new_pattern_only(texpr!(Expr::Variable(
                symbolizer.get("test")
            )))], vec![],
            )
        );
    }

    #[test]
    fn test_print_no_semicolon() {
        let mut symbolizer = Symbolizer::new();
        let str = "{ print 1 }";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        assert_eq!(
            actual,
            Program::new(
                symbolizer.get("main function"),
                vec![],
                vec![],
                vec![PatternAction::new_action_only(Stmt::Print(num!(1.0)))], vec![])
        );
    }

    #[test]
    fn test_column() {
        let mut symbolizer = Symbolizer::new();
        let str = "$0+2 { print a; }";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        let body = Stmt::Print(texpr!(Expr::Variable(symbolizer.get("a"))));

        let col = Expr::Column(bnum!(0.0));
        let binop = texpr!(Expr::MathOp(btexpr!(col), MathOp::Plus, bnum!(2.0)));

        let pa = PatternAction::new(Some(binop), body);
        assert_eq!(actual, Program::new(symbolizer.get("main function"), vec![], vec![], vec![pa], vec![]));
    }

    #[test]
    fn test_nested_column() {
        let mut symbolizer = Symbolizer::new();
        let str = "$$0 { print a; }";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        let body = Stmt::Print(texpr!(Expr::Variable(symbolizer.get("a"))));

        let col = Expr::Column(bnum!(0.0));
        let col = Expr::Column(btexpr!(col));

        let pa = PatternAction::new(Some(texpr!(col)), body);
        assert_eq!(actual, Program::new(symbolizer.get("main function"), vec![], vec![], vec![pa], vec![]));
    }

    #[test]
    fn test_while_l00p() {
        let mut symbolizer = Symbolizer::new();
        let str = "{ while (123) { print 1; } }";
        let actual = parse_unwrap(lex(str, &mut symbolizer).unwrap(), &mut symbolizer);
        let body = Stmt::While(num!(123.0), Box::new(Stmt::Print(num!(1.0))));
        assert_eq!(
            actual,
            Program::new(symbolizer.get("main function"), vec![], vec![], vec![PatternAction::new_action_only(body)], vec![])
        );
    }

    #[test]
    fn test_lt() {
        actual!(actual, "{ 1 < 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(bnum!(1.0), BinOp::Less, bnum!(3.0))));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_gt() {
        actual!(actual, "{ 1 > 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(bnum!(1.0), BinOp::Greater, bnum!(3.0))));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    // test lteq
    #[test]
    fn test_lteq() {
        actual!(actual, "{ 1 <= 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(bnum!(1.0), BinOp::LessEq, bnum!(3.0))));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_gteq() {
        actual!(actual, "{ 1 >= 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(
        bnum!(1.0),
        BinOp::GreaterEq,
        bnum!(3.0)
    )));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_eqeq() {
        actual!(actual, "{ 1 == 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(bnum!(1.0), BinOp::EqEq, bnum!(3.0))));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_bangeq() {
        actual!(actual, "{ 1 != 3 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(bnum!(1.0), BinOp::BangEq, bnum!(3.0))));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_bangeq_oo() {
        actual!(actual, "{ 1 != 3*4 }", symbolizer);
        let body = Stmt::Expr(texpr!(Expr::BinOp(
        bnum!(1.0),
        BinOp::BangEq,
        Box::new(texpr!(Expr::MathOp(bnum!(3.0), MathOp::Star, bnum!(4.0))))
    )));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_cmp_oop1() {
        actual!(actual, "{ 3*3 == 9 }", symbolizer);
        let left = mathop!(bnum!(3.0), MathOp::Star, bnum!(3.0));
        let body = Stmt::Expr(binop!(Box::new(left), BinOp::EqEq, bnum!(9.0)));
        assert_eq!(actual, sprogram!(body, &mut symbolizer));
    }

    #[test]
    fn test_cmp_oop2() {
        actual!(actual, "{ a = 1*3 == 4 }", symbolizer);

        let left = texpr!(Expr::MathOp(bnum!(1.0), MathOp::Star, bnum!(3.0)));
        let body = btexpr!(Expr::BinOp(Box::new(left), BinOp::EqEq, bnum!(4.0)));
        let stmt = Stmt::Expr(texpr!(Expr::ScalarAssign(symbolizer.get("a"), body)));
        assert_eq!(actual, sprogram!(stmt, symbolizer));
    }

    #[test]
    fn test_for_loop() {
        actual!(actual, "{ for (a = 0; a < 1000; a = a + 1) { print a; } }", symbolizer);
        let a = symbolizer.get("a");
        let init = texpr!(Expr::ScalarAssign(a.clone(), btexpr!(Expr::NumberF64(0.0))));
        let test = texpr!(Expr::BinOp(
        btexpr!(Expr::Variable(a.clone())),
        BinOp::Less,
        bnum!(1000.0)
    ));
        let incr = texpr!(Expr::ScalarAssign(
        a.clone(),
        btexpr!(Expr::MathOp(
            btexpr!(Expr::Variable(a.clone())),
            MathOp::Plus,
            btexpr!(Expr::NumberF64(1.0))
        ))
    ));
        let body = Stmt::Print(texpr!(Expr::Variable(a.clone())));
        let expected = Stmt::Group(vec![
            Stmt::Expr(init),
            Stmt::While(test, Box::new(Stmt::Group(vec![body, Stmt::Expr(incr)]))),
        ]);
        assert_eq!(actual, sprogram!(expected, &mut symbolizer))
    }

    #[test]
    fn test_logical_and() {
        actual!(actual, "{ a && b && c }", symbolizer);
        let a = btexpr!(Expr::Variable(symbolizer.get("a")));
        let b = btexpr!(Expr::Variable(symbolizer.get("b")));
        let c = btexpr!(Expr::Variable(symbolizer.get("c")));
        let a_and_b = btexpr!(Expr::LogicalOp(a, LogicalOp::And, b));
        let expected = Stmt::Expr(texpr!(Expr::LogicalOp(a_and_b, LogicalOp::And, c)));
        assert_eq!(actual, sprogram!(expected,  &mut symbolizer))
    }

    #[test]
    fn test_logical_or() {
        actual!(actual, "{ a || b || c }", symbolizer);
        let a = btexpr!(Expr::Variable(symbolizer.get("a")));
        let b = btexpr!(Expr::Variable(symbolizer.get("b")));
        let c = btexpr!(Expr::Variable(symbolizer.get("c")));
        let a_and_b = btexpr!(Expr::LogicalOp(a, LogicalOp::Or, b));
        let expected = Stmt::Expr(texpr!(Expr::LogicalOp(a_and_b, LogicalOp::Or, c)));
        assert_eq!(actual, sprogram!(expected, &mut symbolizer))
    }

    #[test]
    fn string_concat() {
        actual!(actual, "{ print (a b) } ", symbolizer);
        let a = texpr!(Expr::Variable(symbolizer.get("a")));
        let b = texpr!(Expr::Variable(symbolizer.get("b")));
        let print = Stmt::Print(texpr!(Expr::Concatenation(vec![a, b])));
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }

    #[test]
    fn string_concat2() {
        actual!(actual, "{ print (\"a\" \"b\") } ", symbolizer);
        let a = texpr!(Expr::String(symbolizer.get("a")));
        let b = texpr!(Expr::String(symbolizer.get("b")));
        let print = Stmt::Print(texpr!(Expr::Concatenation(vec![a, b])));
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }

    #[test]
    fn string_concat_ooo() {
        actual!(actual, "{ print (a b - c) } ", symbolizer);
        let a = texpr!(Expr::Variable(symbolizer.get("a")));
        let b = btexpr!(Expr::Variable(symbolizer.get("b")));
        let c = btexpr!(Expr::Variable(symbolizer.get("c")));
        let b_minus_c = texpr!(Expr::MathOp(b, MathOp::Minus, c));
        let expected = Stmt::Print(texpr!(Expr::Concatenation(vec![a, b_minus_c])));
        assert_eq!(actual, sprogram!(expected, &mut symbolizer));
    }

    #[test]
    fn string_concat_ooo_2() {
        actual!(actual, "{ print (a - c b ) } ", symbolizer);
        let a = btexpr!(Expr::Variable(symbolizer.get("a")));
        let b = texpr!(Expr::Variable(symbolizer.get("b")));
        let c = btexpr!(Expr::Variable(symbolizer.get("c")));
        let a_minus_c = texpr!(Expr::MathOp(a, MathOp::Minus, c));
        let expected = Stmt::Print(texpr!(Expr::Concatenation(vec![a_minus_c, b])));
        assert_eq!(actual, sprogram!(expected, &mut symbolizer));
    }

    #[test]
    fn string_concat_ooo_3() {
        actual!(actual, "{ print (a < b c ) } ", symbolizer);
        let a = btexpr!(Expr::Variable(symbolizer.get("a")));
        let b = texpr!(Expr::Variable(symbolizer.get("b")));
        let c = texpr!(Expr::Variable(symbolizer.get("c")));
        let b_concat_c = btexpr!(Expr::Concatenation(vec![b, c]));
        let expected = Stmt::Print(texpr!(Expr::BinOp(a, BinOp::Less, b_concat_c)));
        assert_eq!(actual, sprogram!(expected, &mut symbolizer));
    }

    #[test]
    fn string_concat_ooo_4() {
        actual!(actual, "{ print (a b < c ) } ", symbolizer);
        let a = texpr!(Expr::Variable(symbolizer.get("a")));
        let b = texpr!(Expr::Variable(symbolizer.get("b")));
        let c = btexpr!(Expr::Variable(symbolizer.get("c")));
        let a_concat_b = btexpr!(Expr::Concatenation(vec![a, b]));
        let expected = Stmt::Print(texpr!(Expr::BinOp(a_concat_b, BinOp::Less, c)));
        assert_eq!(actual, sprogram!(expected, &mut symbolizer));
    }

    #[test]
    fn string_concat_two_cols() {
        actual!(actual, "{ print $1 $2 } ", symbolizer);
        let one = texpr!(Expr::Column(bnum!(1.0)));
        let two = texpr!(Expr::Column(bnum!(2.0)));
        let concat = texpr!(Expr::Concatenation(vec![one, two]));
        let print = Stmt::Print(concat);
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }


    #[test]
    fn array_membership() {
        actual!(actual, "{ 1 in a } ", symbolizer);
        let expr = texpr!(Expr::InArray{name: symbolizer.get("a"),  indices: vec![num!(1.0)]});
        let print = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }

    #[test]
    fn multi_dim_array_membership() {
        actual!(actual, "{ (1,2,3) in a } ", symbolizer);
        let expr = texpr!(Expr::InArray{name: symbolizer.get("a"),  indices: vec![num!(1.0),num!(2.0),num!(3.0)]});
        let print = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }

    #[test]
    fn multi_multi_dim_array_membership() {
        actual!(actual, "{ (1,2,3) in a in b} ", symbolizer);
        let expr = texpr!(
        Expr::InArray{name: symbolizer.get("b"),
            indices: vec![
                Expr::InArray{name: symbolizer.get("a"),  indices: vec![num!(1.0),num!(2.0),num!(3.0)]}.into()]});
        let print = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(print, &mut symbolizer));
    }

    #[test]
    fn array_access() {
        actual!(actual, "{ a[0] }", symbolizer);
        let expr = texpr!(Expr::ArrayIndex{name: symbolizer.get("a"),indices: vec![Expr::NumberF64(0.0).into()]});
        let stmt = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }


    #[test]
    fn array_access_multi() {
        actual!(actual, "{ a[0,1,2,3] }", symbolizer);
        let expr = texpr!(Expr::ArrayIndex{name: symbolizer.get("a"),indices: vec![num!(0.0), num!(1.0),num!(2.0),num!(3.0)]});
        let stmt = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }

    #[test]
    fn array_access_multi_expr() {
        actual!(actual, "{ a[0+1] }", symbolizer);
        let zero = bnum!(0.0);
        let one = bnum!(1.0);
        let op = Expr::MathOp(zero, MathOp::Plus, one).into();
        let expr = texpr!(Expr::ArrayIndex{name: symbolizer.get("a"),indices: vec![op]});
        let stmt = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }

    #[test]
    fn array_access_nested() {
        actual!(actual, "{ a[a[0]] }", symbolizer);
        let expr = texpr!(Expr::ArrayIndex{name: symbolizer.get("a"),indices: vec![Expr::NumberF64(0.0).into()]});
        let outer = texpr!(Expr::ArrayIndex {name: symbolizer.get("a"), indices: vec![expr]});
        assert_eq!(actual, sprogram!(Stmt::Expr(outer), &mut symbolizer));
    }

    #[test]
    fn array_access_assign() {
        actual!(actual, "{ a[0] = 1 }", symbolizer);
        let expr = texpr!(Expr::ArrayAssign{name: symbolizer.get("a"),indices: vec![Expr::NumberF64(0.0).into()], value: bnum!(1.0)});
        assert_eq!(actual, sprogram!(Stmt::Expr(expr), &mut symbolizer));
    }


    #[test]
    fn array_access_assign_multi_dim() {
        actual!(actual, "{ a[0,2] = 1 }", symbolizer);
        let a = symbolizer.get("a");
        let expr = Expr::ArrayAssign { name: a, indices: vec![num!(0.0), num!(2.0)], value: Box::new(num!(1.0)) }.into();
        assert_eq!(actual, sprogram!(Stmt::Expr(expr), &mut symbolizer));
    }

    #[test]
    fn test_expr_call_nonary() {
        actual!(actual, "{ a() }", symbolizer);
        let a = symbolizer.get("a");
        let expr = Expr::Call { target: a, args: vec![] };
        assert_eq!(actual, sprogram!(Stmt::Expr(expr.into()), &mut symbolizer));
    }

    #[test]
    fn test_expr_call_unary() {
        actual!(actual, "{ a(1) }", symbolizer);
        let a = symbolizer.get("a");
        let expr = Expr::Call { target: a, args: vec![num!(1.0)] };
        assert_eq!(actual, sprogram!(Stmt::Expr(expr.into()), &mut symbolizer));
    }

    #[test]
    fn test_expr_call_many() {
        actual!(actual, "{ a(1,3,5) }", symbolizer);
        let a = symbolizer.get("a");
        let expr = Expr::Call { target: a, args: vec![num!(1.0), num!(3.0), num!(5.0)] }.into();
        assert_eq!(actual, sprogram!(Stmt::Expr(expr), &mut symbolizer));
    }

    #[test]
    fn array_assign_multi_expr() {
        actual!(actual, "{ a[0+1, a[0]] }", symbolizer);
        let a = symbolizer.get("a");
        let zero = bnum!(0.0);
        let one = bnum!(1.0);
        let op = Expr::MathOp(zero, MathOp::Plus, one).into();
        let a_zero = Expr::ArrayIndex { name: a, indices: vec![num!(0.0)] }.into();
        let expr = texpr!(Expr::ArrayIndex{name: symbolizer.get("a"),indices: vec![op, a_zero]});
        let stmt = Stmt::Expr(expr);
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }

    #[test]
    fn test_printf_simple() {
        actual!(actual, "{ printf 1 }", symbolizer);
        let stmt = Stmt::Printf { fstring: num!(1.0), args: vec![] }.into();
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }

    #[test]
    fn test_printf_multi() {
        actual!(actual, "{ printf \"%s%s%s\", 1, 2, 3 }", symbolizer);
        let stmt = Stmt::Printf { fstring: Expr::String(symbolizer.get("%s%s%s")).into(), args: vec![num!(1.0), num!(2.0), num!(3.0)] }.into();
        assert_eq!(actual, sprogram!(stmt, &mut symbolizer));
    }

    #[test]
    fn test_function() {
        actual!(actual, "function abc(a,b,c) { print 1; } BEGIN { print 1 }", symbolizer);
        let a = symbolizer.get("a");
        let b = symbolizer.get("b");
        let c = symbolizer.get("c");
        let body = Stmt::Print(Expr::NumberF64(1.0).into());
        let function = Function::new(symbolizer.get("abc"), vec![a, b, c], body);
        let begin = Stmt::Print(Expr::NumberF64(1.0).into());
        assert_eq!(actual, Program::new(symbolizer.get("main function"), vec![begin], vec![], vec![], vec![function]))
    }

    #[test]
    fn test_call() {
        actual!(actual, "BEGIN { a(1,\"2\"); }", symbolizer);
        let a = symbolizer.get("a");
        let args = vec![
            Expr::NumberF64(1.0).into(),
            Expr::String(symbolizer.get("2")).into(),
        ];
        let begin = Stmt::Expr(Expr::Call { target: a, args }.into());
        assert_eq!(actual, Program::new(symbolizer.get("main function"), vec![begin], vec![], vec![], vec![]))
    }
}
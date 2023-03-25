use crate::test::{test_runner, long_number_file, ONE_LINE, SUB_RULES, SUB_ESCAPING, REDIRECT, NUMBERS, NUMBERS2, FLOAT_NUMBERS, NUMERIC_STRING, ABC, PERF_ARRAY_PROGRAM, EMPTY_INDEX_PROGRAM, TTX1, test_runner_multifile};
use crate::test::awks::Awk;
#[macro_export]
macro_rules! test {
        ($name:ident,$prog:expr,$file:expr,$stdout:expr) => {
            #[test]
            fn $name() {
                test_runner(stringify!($name), $prog, $file, $stdout, 0);
            }
        };
    }


#[macro_export]
macro_rules! test_except {
        ($name:ident,$prog:expr,$file:expr,$stdout:expr,$except:expr) => {
            #[test]
            fn $name() {
                test_runner(stringify!($name), $prog, $file, $stdout, $except);
            }
        };
    }

#[test]
fn prog_awk_test() {
    let str = std::fs::read_to_string("/Users/n8ta/code/jawk/rawk/prog.awk").unwrap();
    test_runner("run prog.awk", &str, "1 2 3\n4 5 6\n", "", 0);
}


test!(test_str_escape, r##"BEGIN { a = "\a\n\r\t\1"; print a }  "##, "", vec![7,10, 0xd, 9, 0x1, 10]);
test_except!(test_sub_rules, SUB_RULES, ONE_LINE, "-\\a-\n", Awk::Onetrueawk as usize);

test!(test_perf_concat_loop, "BEGIN { a = \"\"; b = \"\"; x = 0; while (x < 5000) {     a = a \"a\";     b = b \"a\";     x = x + 1;     if (a > b) {         print \"a is not eq to b\";    } } print x; print \"done\"; }", "", "5000\ndone\n");
test!(test_print_begin_int, "BEGIN {print 1;}", ONE_LINE, "1\n");
test!(test_print_int, "{print 1;}", ONE_LINE, "1\n");
test!(test_print_str, "BEGIN {print \"abc\";}", ONE_LINE, "abc\n");
test!(test_print_str_loop, "{print \"abc\";}", ONE_LINE, "abc\n");
test!(test_just_begin, "BEGIN { print 1; }", ONE_LINE, "1\n");

test!(test_assign_undef_to_undef, "BEGIN { x = x; }", ONE_LINE, "");
test!(
        test_print_assign_to_undef,
        "BEGIN { print (x = x + 1); }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_simple_exponential,
        "BEGIN { print (x = 2 ^ 2); }",
        ONE_LINE,
        "4\n"
    );
test!(
        test_simple_exponential_order_op_pre,
        "BEGIN { print (x = 3 * 2 ^ 2); }",
        ONE_LINE,
        "12\n"
    );
test!(
        test_simple_exponential_order_op_post,
        "BEGIN { print (x = 2 ^ 2 * 3); }",
        ONE_LINE,
        "12\n"
    );
test!(
        test_e2e_begin_end,
        "BEGIN { print 1; } END { print 3; } END { print 4; }",
        ONE_LINE,
        "1\n3\n4\n"
    );
test!(
        test_oo_beg_end,
        "END { print 3; } { print 2; } BEGIN {print 1;}",
        ONE_LINE,
        "1\n2\n3\n"
    );
test!(test_str_leak, "BEGIN { a = \"b\"; }", ONE_LINE, "");
test!(test_empty, "BEGIN { }", ONE_LINE, "");
test!(test_1_assgn, "BEGIN {x = 1; }", ONE_LINE, "");
test!(test_4_assgn, "BEGIN {x = 4; print x }", ONE_LINE, "4\n");
test!(test_cmpop2, "BEGIN { print (3 < 5) }", ONE_LINE, "1\n");
test!(test_cmpop1, "BEGIN { print (5 < 3) }", ONE_LINE, "0\n");
test!(
        test_dup_beg_end,
        "END { print 4; } END { print 3; } { print 2; } BEGIN { print 0; } BEGIN {print 1;} ",
        ONE_LINE,
        "0\n1\n2\n4\n3\n"
    );
test!(test_simple_assignment, "{x = 0; print x;}", ONE_LINE, "0\n");
test!(test_simple_assgn, "{x = 0; print x }", ONE_LINE, "0\n");
test!(
        test_assignment_in_ifs0,
        "{x = 0; if (1) { x = 1 }; print x }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_assignment_in_ifs,
        "{x = 0; if (1) { x = 1 } else { x = 2.2 }; print x }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_nested_if_assignment,
        "{x = 0; if (0) { x = 1 } else { x = 2.2 }; print x }",
        ONE_LINE,
        "2.2\n"
    );
test!(
        test_mixed_int_float_assignment,
        "{x = 0; if (x) { x = 1 } else { x = 2.2 }; print x }",
        ONE_LINE,
        "2.2\n"
    );
test!(test_deeply_nested_mixed_assignment, "{x = 0; if (1) { if (1) { x = 1 } else { x = 2.2 } } else { if (1) { x = 1 } else { x = 4.2 } }; print x }", ONE_LINE, "1\n");
test!(test_deeply_nested_mixed_assignment2, "{x = 0; if (1) { if (1) { x = 1 } else { x = 2.2 } } else { if (1) { x = 1 } else { x = 4.2 } }; { x = 4; x=5; x=5.5; print x; } }", ONE_LINE, "5.5\n");
test!(test_int_plus_float, "{print 1 + 1.1}", ONE_LINE, "2.1\n");
test!(test_float_plus_int, "{print 1.1 + 1}", ONE_LINE, "2.1\n");
test!(test_grouping, "{print (1.1 + 3.3) + 1}", ONE_LINE, "5.4\n");
test!(test_float_add, "{print (1.0 + 2.0)}", ONE_LINE, "3\n");
test!(
        test_column_access_1_line,
        "{print $1; print $2; print $3; print $0}",
        ONE_LINE,
        "1\n2\n3\n1 2 3\n"
    );
test!(
        test_column_access_many_line,
        "{print $1; print $2; print $3; print $0}",
        NUMBERS,
        "1\n2\n3\n1 2 3\n4\n5\n6\n4 5 6\n7\n8\n9\n7 8 9\n"
    );

test!(
        test_if_no_else_truthy,
        "{if (1) { print 123; }}",
        ONE_LINE,
        "123\n"
    );
test!(
        test_float_truthyness,
        "{if (0) { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "cde\n"
    );
test!(
        test_float_truthyness2,
        "{if (1) { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "abc\n"
    );
test!(
        test_float_truthyness3,
        "{if (100) { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "abc\n"
    );
test!(
        test_float_truthyness4,
        "{if (1000) { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "abc\n"
    );

test!(
        test_str_truthyness0,
        "{a = \"\"; if (a) { print 5 } }",
        ONE_LINE,
        ""
    );
test!(
        test_str_truthyness1,
        "{if (\"\") { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "cde\n"
    );
test!(
        test_str_truthyness2,
        "{if (\"a\") { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "abc\n"
    );
test!(
        test_str_truthyness3,
        "{if (\"aaaaklasdjksfdakljfadskljafsdkljfas\") { print \"abc\" } else { print \"cde\" }}",
        ONE_LINE,
        "abc\n"
    );
test!(test_str_truthyness4, "{if (\"aaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfasaaaaklasdjksfdakljfadskljafsdkljfas\") { print \"abc\" } else { print \"cde\" }}", ONE_LINE, "abc\n");

test!(
        test_assign_then_print_simple,
        "{ a = 1.1; print a }",
        ONE_LINE,
        "1.1\n"
    );
test!(
        test_assign_then_print_sep,
        "{ a = 1.1 } { print a }",
        ONE_LINE,
        "1.1\n"
    );
test!(
        test_assign_then_end,
        "{ a = 1.1 } END { print a }",
        ONE_LINE,
        "1.1\n"
    );
test!(
        test_print_col0,
        "{ a = $0 } END { print a }",
        NUMBERS,
        "7 8 9\n"
    );
test!(
        test_print_col1,
        "{ a = $1 } END { print a }",
        NUMBERS,
        "7\n"
    );
test!(
        test_print_col2,
        "{ a = $2 } END { print a }",
        NUMBERS,
        "8\n"
    );
test!(
        test_print_col3,
        "{ a = $3 } END { print a }",
        NUMBERS,
        "9\n"
    );
test!(
        test_print_col_big,
        "{ a = $44 } END { print a }",
        NUMBERS,
        "\n"
    );
test!(
        test_eqeq_true,
        "{ if (0==0) { print 123; } else {print 456;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_eqeq_false,
        "{ if (0==1) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_bangeq_true,
        "{ if (0!=0) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_bangeq_false,
        "{ if (0!=1) { print 123; } else {print 456;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_lt_true,
        "{ if (0 < 123) { print 123; } else {print 456;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_lt_false,
        "{ if (123 < 12) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_lteq_true,
        "{ if (0 <= 1) { print 123; } else {print 123;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_lteq_false,
        "{ if (1 <= 0) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_gt_true,
        "{ if (1 > 0) { print 123; } else {print 456;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_gt_false,
        "{ if (0 > 1) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_gteq_true,
        "{ if (1 >= 0) { print 123; } else {print 456;} }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_gteq_false,
        "{ if (0 >= 1) { print 123; } else {print 456;} }",
        ONE_LINE,
        "456\n"
    );
test!(
        test_while_0,
        "{ while (x < 4) { x = x + 1; print x; } print 555; }",
        ONE_LINE,
        "1\n2\n3\n4\n555\n"
    );
test!(
        test_long_loop,
        "{ x = 0; while (x<50) { x = x + 1; } print x; }",
        ONE_LINE,
        "50\n"
    );
test!(
        test_if_no_else_truthy_str,
        "{if (1) { print \"truthy\"; }}",
        ONE_LINE,
        "truthy\n"
    );
test!(
        test_mixed_logical0,
        "BEGIN { x = 0; x = x && \"123\"; print x; }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_mixed_logical1,
        "BEGIN { x = 1; x = x && \"123\"; print x; }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_mixed_logical2,
        "BEGIN { x = 1; x = \"123\" && x; print x; }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_mixed_logical3,
        "BEGIN { x = 1; x = x || \"123\"; print x; }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_mixed_logical4,
        "BEGIN { x = 0; x = x || \"123\"; print x; }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_mixed_logical5,
        "BEGIN { x = 0; x = x || \"\"; print x; }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_mixed_logical6,
        "BEGIN { x = 1; x = \"123\" && x; print x; }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_mixed_logical7,
        "BEGIN { print (0 && 123) }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_mixed_addition0,
        "BEGIN { x = x + \"123\"; print x; }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_mixed_addition1,
        "BEGIN { x = 0; x = x + \"123\"; print x; }",
        ONE_LINE,
        "123\n"
    );
test!(
        test_mixed_addition2,
        "BEGIN { x = 0; x = x + \"123\"; x = x + 5; print x; }",
        ONE_LINE,
        "128\n"
    );
test!(
        test_mixed_addition3,
        "BEGIN { x = 0; x = x + (\"123\" + 44 + \"33\"); x = x + 5; print x; }",
        ONE_LINE,
        "205\n"
    );
test!(
        test_mixed_addition4,
        "BEGIN { x = 0; x = x + (\"1\" + 2); print x; }",
        ONE_LINE,
        "3\n"
    );
test!(
        test_assignment_expr,
        "BEGIN { x = (y = 123); print x}",
        ONE_LINE,
        "123\n"
    );
test!(
        test_assignment_expr2,
        "BEGIN { x = ((y = 123) + (z = 4)); print x}",
        ONE_LINE,
        "127\n"
    );
test!(
        test_nested_assignment,
        "BEGIN { a = b = c = d = e = f = 4 < 10; print d; print a; }",
        ONE_LINE,
        "1\n1\n"
    );
test!(
        test_short_circuit_or,
        "BEGIN { print (4 || ((4)/0)) }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_short_circuit_or2,
        "BEGIN { print (4 || ((4)/0) || ((4)/0) )}",
        ONE_LINE,
        "1\n"
    );
test!(
        test_short_circuit_or3,
        "BEGIN { print (0 || 4) }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_short_circuit_and,
        "BEGIN { print (0 && ((4)/0)) }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_short_circuit_and2,
        "BEGIN { print (123 && 5) }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_short_circuit_and3,
        "BEGIN { print (123 && 0) }",
        ONE_LINE,
        "0\n"
    );
test!(test_nested_column, "{ print ($$$$1) }", REDIRECT, "5\n");
test!(
        test_nested_column_oop,
        "{ print ($$$$1 + 100) }",
        REDIRECT,
        "105\n"
    );
test!(
        test_concat,
        "BEGIN { print (\"a\" \"b\") }",
        REDIRECT,
        "ab\n"
    );
test!(
        test_concat2,
        "BEGIN { print (\"a\" \"b\" \"cccc\" \"ddd\") }",
        REDIRECT,
        "abccccddd\n"
    );
test!(
        test_concat3,
        "BEGIN { a = \"a\"; print (a \"b\") }",
        REDIRECT,
        "ab\n"
    );
test!(
        test_concat_cols,
        "BEGIN { a = \"a\"; print (a) }",
        ONE_LINE,
        "a\n"
    );
test!(
        test_concat_unused,
        "BEGIN { z = \"abc\" \"def\"; }",
        ONE_LINE,
        ""
    );
test!(test_concat_cols2, "{ print ($1 $2) }", ONE_LINE, "12\n");
test!(test_concat_cols3, "{ print ($1 $2 $3) }", ONE_LINE, "123\n");
test!(
        test_concat_multiline,
        "{ a = a $1;} END{ print a}",
        NUMBERS,
        "147\n"
    );
test!(
        test_concat_multiline_intermed,
        "{ a = a $1; print a}",
        NUMBERS,
        "1\n14\n147\n"
    );

test!(
        test_binop_1,
        "BEGIN { print (\"a\" < \"a\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_2,
        "BEGIN { print (\"a\" < \"aa\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_3,
        "BEGIN { print (\"a\" > \"a\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_4,
        "BEGIN { print (\"a\" > \"aa\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_5,
        "BEGIN { print (\"aaaa\" > \"aa\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_6,
        "BEGIN { print (\"a\" <= \"a\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_7,
        "BEGIN { print (\"a\" >= \"a\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_8,
        "BEGIN { print (\"a\" >= \"aaa\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_9,
        "BEGIN { print (\"aaaaaaaa\" >= \"aaa\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_10,
        "BEGIN { print (\"aaa\" == \"aaa\") }",
        ONE_LINE,
        "1\n"
    );
test!(
        test_binop_11,
        "BEGIN { print (\"aaa\" == \"aafa\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_12,
        "BEGIN { print (\"aaa\" != \"aaa\") }",
        ONE_LINE,
        "0\n"
    );
test!(
        test_binop_13,
        "BEGIN { print (\"aaa3\" != \"aaa\") }",
        ONE_LINE,
        "1\n"
    );
test!(test_while_simple_0, "BEGIN { x = 0; while (x) { print x } }", "", "");
test!(
        test_assign_ops_0,
        "BEGIN { a = 3; print a += 1 }",
        ONE_LINE,
        "4\n"
    );
test!(
        test_assign_ops_1,
        "BEGIN { a = 1; b = 3; a += b += 4; print a; print b; }",
        ONE_LINE,
        "8\n7\n"
    );
test!(test_assign_ops_2, "BEGIN { a = 1; b = 3; c = 5; d = 7; a += b +=c -= d ^= 3; print a; print b; print c; print d  }", ONE_LINE, "-334\n-335\n-338\n343\n");
test!(test_looping_concat, "BEGIN { a = \"\"; b = \"\"; x = 0; while (x < 50) {a = a \"a\"; b = b \"b\"; x += 1; } print a; print b; print x; }", ONE_LINE, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n50\n");
test!(
        test_concat_undef,
        "BEGIN { a = a \"a\"; print a; }",
        ONE_LINE,
        "a\n"
    );

test!(test_loop_concat_long1, "BEGIN {a = \"\";        b = \"\";        x = 0;        while (x < 100) { a = a \"a\";                b = b \"a\";                x = x + 1;                if (a > b) {print \"a is not eq to b\";                }}print x;        print \"done\";}", ONE_LINE, "100\ndone\n");
test!(test_loop_concat_long2, "BEGIN {a = \"\";        b = \"\";        x = 0;        while (x < 100) { a = a \"a\";                b = b \"a\";                x = x + 1;                if (a != b) {print \"a is not eq to b\";                }}print x;        print \"done\";}", ONE_LINE, "100\ndone\n");

test!(test_pattern_only_1_4, "$1 == $4", NUMBERS, "");
test!(
        test_pattern_only_1_4_2,
        "$1 == $4",
        NUMBERS2,
        "4 5 6 4\n7 8 9 7\n"
    );

test!(test_pattern_long, "$1 == $4", long_number_file(), "");

test!(
        test_numeric_string1,
        "{ print ($1 > 2) }",
        NUMERIC_STRING,
        "0\n1\n1\n"
    );

test!(gawk_strnum_5, "{ print($1 == \"+3.14\") }", "+3.14", "1\n");
test!(gawk_strnum_7, "{ print($1 == 3.14) }", "+3.14", "1\n");

test!(gawk_strnum_0, "{ print($0 == \"    +3.14\") }", PI, "1\n");
test!(gawk_strnum_3, "{ print($0 == 3.14) }", PI, "1\n");
test!(gawk_strnum_1, "{ print($0 == \"+3.14\") }", PI, "0\n");
test!(gawk_strnum_2, "{ print($0 == \"3.14\") }", PI, "0\n");
test!(gawk_strnum_4, "{ print($1 == \" +3.14\") }", PI, "0\n");
test!(gawk_strnum_6, "{ print($1 == \"3.14\") }", PI, "0\n");

const NUM2: &'static str = "002";
test!(split_numstr_n1, "{ split($0, a); }", NUM2, "");
test!(split_numstr_0, "{ split($0, a); print a[1]; }", NUM2, "002\n");
test!(split_numstr_1, "{ split($0, a); print a[1]; print( a[1] < 2); }", NUM2, "002\n0\n");

test!(
        test_numeric_string2,
        "{ print ($0 < $1 ) }",
        NUMERIC_STRING,
        "0\n0\n0\n"
    );
test!(
        test_numeric_string3,
        "{ print (\"04\" > \"005\") }",
        NUMERIC_STRING,
        "1\n1\n1\n"
    );
test!(
        test_numeric_string4,
        "{ print (\"04\" >= \"005\") }",
        NUMERIC_STRING,
        "1\n1\n1\n"
    );
test!(
        test_post_increment,
        "BEGIN { a = 4; print a++ + a++}",
        NUMERIC_STRING,
        "9\n"
    );
test!(
        test_post_decrement,
        "BEGIN { a = 4; print a-- - a--}",
        NUMERIC_STRING,
        "1\n"
    );
test!(
        test_post_decrement_and_increment,
        "BEGIN { a = 4; print a++ - a--}",
        NUMERIC_STRING,
        "-1\n"
    );
test!(
        test_exp_post_increment,
        "BEGIN { a = 3; print 2 ^ a++; print a }",
        NUMERIC_STRING,
        "8\n4\n"
    );
test!(
        test_post_increment_exp,
        "BEGIN { a = 3; print a++ ^ 2; print a}",
        NUMERIC_STRING,
        "9\n4\n"
    );
test!(
        test_pre_increment,
        "BEGIN { a = 3; print ++a; print a}",
        NUMERIC_STRING,
        "4\n4\n"
    );
test!(
        test_pre_decrement,
        "BEGIN { a = 3; print --a; print a}",
        NUMERIC_STRING,
        "2\n2\n"
    );
test!(
        test_post_pre_increment,
        "BEGIN { a = 3; print a++ + ++a; print a}",
        NUMERIC_STRING,
        "8\n5\n"
    );

test!(
        test_post_pre_decrement,
        "BEGIN { a = 3; print a-- + --a; print a}",
        NUMERIC_STRING,
        "4\n1\n"
    );
test!(test_mod_2, "BEGIN { print (3 % 2) }", NUMERIC_STRING, "1\n");
test!(
        test_ternary_false,
        "BEGIN { print 0 ? 1 : 2 }",
        NUMERIC_STRING,
        "2\n"
    );
test!(
        test_ternary_true,
        "BEGIN { print 1 ? 1 : 2 }",
        NUMERIC_STRING,
        "1\n"
    );
test!(
        test_ternary_arith,
        "BEGIN { print 1 ? 1+1 : 2+2 }",
        NUMERIC_STRING,
        "2\n"
    );

test!(
        test_ternary_nested,
        "BEGIN { x = 2; y = 3; print x ? ( y ? \"true\" : 3 ) : 4 }",
        ONE_LINE,
        "true\n"
    );

test!(
        test_ternary_nested_flat1,
        "BEGIN { x = 3; y = 0; print x ? y ? 33 : 44 : 55; }",
        ONE_LINE,
        "44\n"
    );
test!(
        test_ternary_nested_flat2,
        "BEGIN { x = 0; y = 0; print x ? y ? 33 : 44 : 55; }",
        ONE_LINE,
        "55\n"
    );
test!(
        test_ternary_nested_flat3,
        "BEGIN { x = 0; z = 3; print x ? y : z ? 2 : 3 }",
        ONE_LINE,
        "2\n"
    );
test!(
        test_ternary_nested_flat4,
        "BEGIN { x = 0; z = 3; y = 5; print (x ? 0 : 2) ? y : z ? 2 : 3 }",
        ONE_LINE,
        "5\n"
    );
test!(test_unary_1, "BEGIN { print (-+-!0) }", ONE_LINE, "1\n");
test!(test_unary_op2, "BEGIN { print (+-+2) }", ONE_LINE, "-2\n");
test!(
        test_unary_op_w_decrement,
        "BEGIN { print (+-+2) }",
        ONE_LINE,
        "-2\n"
    );
test!(
        test_unary_op_w_postdecrement_bang,
        "BEGIN {x = 1; print(!x--); print(x)}",
        NUMERIC_STRING,
        "0\n0\n"
    );
test!(
        test_unary_op_w_predecrement_plus,
        "BEGIN {x = 1; print(+--x); print(x)}",
        NUMERIC_STRING,
        "0\n0\n"
    );
test!(
        test_regex_1,
        "BEGIN { print \"123\" ~ \"1\"}",
        ONE_LINE,
        "1\n"
    );
test!(
        test_regex_2,
        "BEGIN { print \"123\" !~ \"1\"}",
        ONE_LINE,
        "0\n"
    );
test!(
        test_regex_3,
        "BEGIN { print \"123\" ~ /1/}",
        ONE_LINE,
        "1\n"
    );
test!(
        test_regex_4,
        "BEGIN { print \"123\" !~ /1/}",
        ONE_LINE,
        "0\n"
    );

test!(
        test_regex_5,
        "BEGIN { print \"123\" ~ /3/}",
        ONE_LINE,
        "1\n"
    );

test!(test_array_get_1, "BEGIN { print a[0] }", ONE_LINE, "\n");

test!(
        test_array_set_get_single,
        "BEGIN { a[0] = 5; print a[0]; a[1] = 2; print a[1]; a[1] = 3; print a[1]; }",
        ONE_LINE,
        "5\n2\n3\n"
    );

test!(
        test_array_get_multi,
        "BEGIN { print a[0, 1] }",
        ONE_LINE,
        "\n"
    );

test!(
        test_array_set_get_multi,
        "BEGIN { a[0,1] = 5; print a[0, 1] }",
        ONE_LINE,
        "5\n"
    );

test!(
        test_in_array_1,
        "BEGIN { a[5] = 3; print 5 in a; }",
        ONE_LINE,
        "1\n"
    );

test!(
        test_in_array_2,
        "BEGIN { a[5] = 3; print (5) in a; }",
        ONE_LINE,
        "1\n"
    );

test!(
        test_in_array_3,
        "BEGIN { a[4] = 4; a[1,2,3] = 3; print (1,2,3) in a; print (123 in a) }",
        ONE_LINE,
        "1\n0\n"
    );

test!(
        test_multidim_array_in,
        "BEGIN {a[0,1] = 3 ; print a[0,1]; }",
        ONE_LINE,
        "3\n"
    );

test!(
        test_multidim_array_in_str,
        "BEGIN {a[\"0-1\"] = 3 ; print a[\"0-1\"]; }",
        ONE_LINE,
        "3\n"
    );

test!(
        test_multi_in_array_1,
        "BEGIN { a[5] = 3; b[3] = 2; b[2] = 1; b[1] = 5; print 3 in b in b; }",
        ONE_LINE,
        "1\n"
    );

// test!(
// test_perf_array,
// PERF_ARRAY_PROGRAM,
// ONE_LINE,
// "800020000\n"
// );

test!(
    test_two_arrays,
    "BEGIN { a[0] = 1; a[1] =1; b[0] = 2; b[1] = 3; x=2; while (x++ < 40) { a[x] = a[x-1] + a[x-2]; b[x] = b[x-1] + b[x-2]; print a[x]; print b[x] }}",
    ONE_LINE,
    "1\n3\n1\n3\n2\n6\n3\n9\n5\n15\n8\n24\n13\n39\n21\n63\n34\n102\n55\n165\n89\n267\n144\n432\n233\n699\n377\n1131\n610\n1830\n987\n2961\n1597\n4791\n2584\n7752\n4181\n12543\n6765\n20295\n10946\n32838\n17711\n53133\n28657\n85971\n46368\n139104\n75025\n225075\n121393\n364179\n196418\n589254\n317811\n953433\n514229\n1542687\n832040\n2496120\n1346269\n4038807\n2178309\n6534927\n3524578\n10573734\n5702887\n17108661\n9227465\n27682395\n14930352\n44791056\n24157817\n72473451\n39088169\n117264507\n"
);

test!(test_simple_concat, "BEGIN { a[0] = 1 1 }", ONE_LINE, "");
test!(test_leak, "BEGIN { while (x++ < 1) { }}", ONE_LINE, "");

test!(
    test_array_with_str,
    "BEGIN { while (x++ < 30) { a[x] = a[x-1] \".\"; print a[x] }}",
    ONE_LINE,
    ".\n..\n...\n....\n.....\n......\n.......\n........\n.........\n..........\n...........\n............\n.............\n..............\n...............\n................\n.................\n..................\n...................\n....................\n.....................\n......................\n.......................\n........................\n.........................\n..........................\n...........................\n............................\n.............................\n..............................\n"
);
test!(test_array_basic,"BEGIN { x = \"1\"; while (x++<3) { a[x] = 1; print a[x] }}", ONE_LINE, "1\n1\n");

test!(
        test_array_override_with_int,
        "BEGIN { a[0] = \"1\"; a[0] = 1; }",
        ONE_LINE,
        ""
    );

test!(
        test_break_simple,
        "BEGIN { while (1) { break } }",
        ONE_LINE,
        ""
    );

test!(
        test_break_loop_uninit,
        "BEGIN { while (1) { if (x == 33) { break } x = x + 1; } print x; }",
        ONE_LINE,
        "33\n"
    );

test!(
        test_break_loop_known_type,
        "BEGIN { x = 5; while (1) { if (x == 33) { break } x = x + 1; } print x; }",
        ONE_LINE,
        "33\n"
    );

test!(
        test_break_2,
        "BEGIN { while (1) { if (x) { break } break } }",
        ONE_LINE,
        ""
    );

test!(
        drop_on_end_0,
        "BEGIN { x = 1; x = \"A\"; x = 4}",
        ONE_LINE,
        ""
    );
test!(
        drop_on_end_1,
        "BEGIN { x = \"A\"; x = 4}",
        ONE_LINE,
        ""
    );

test!(
    test_double_break_loop,
    "BEGIN {while(1) {     z=0; while(1) {if(z==30){break}z++;a++}        y++; if(y==40) {break}} print y; print a;}",
    ONE_LINE,
    "40\n1200\n"
);

test!(test_double_break_loop_2,"BEGIN {while(1) { z=0; while(1) {z++; break; } break; }  }",ONE_LINE,"");

// test!(
//     test_printf_simple_f,
//     "BEGIN {printf \"test\"}",
//     ONE_LINE,
//     "test"
// );

test!(
        test_func_const_only,
        "function uses_nil() { print \"1\";  } BEGIN { uses_nil();}",
        ONE_LINE,
        "1\n"
    );

test!(
        test_func_global_float_only,
        "function uses_nil() { print global_1;  } BEGIN { global_1 = 3; uses_nil();}",
        ONE_LINE,
        "3\n"
    );

test!(
        test_simple_func_global,
        "function uses_nil() { a = 1; } BEGIN { }",
        ONE_LINE,
        ""
    );

test!(
        test_func_global_assign_no_read,
        "function uses_nil() { a = 3; print a; } BEGIN { uses_nil(); }",
        ONE_LINE,
        "3\n"
    );

test!(
        test_func_global_assign_n_read,
        "function uses_nil() { a = 3; print a; } BEGIN { uses_nil();  print a; }",
        ONE_LINE,
        "3\n3\n"
    );

test!(
    test_func_global_string_only,
    "function uses_global() { print global_1;  } BEGIN { global_1 = \"abc\"; print global_1; uses_global(); print global_1; global_1 = \"ddd\"; print global_1; uses_global(); print global_1;}",
    ONE_LINE,
    "abc\nabc\nabc\nddd\nddd\nddd\n"
);

test!(
        test_func_global_arr_only,
        "function uses_nil() { print global_1[0];  } BEGIN { global_1[0] = 5; uses_nil();}",
        ONE_LINE,
        "5\n"
    );

test!(
        test_func_call_0,
        "function uses_scalar(scalar) { print scalar;  } BEGIN { uses_scalar(1);}",
        ONE_LINE,
        "1\n"
    );

test!(
        test_func_call_1,
        "function a(arr) { arr[0] = 123; } BEGIN { a(b); print b[0]; }",
        ONE_LINE,
        "123\n"
    );

test!(
        test_func_call_2,
        "function a(arg) { print $arg } { a(1); a(2); a(3); }",
        ONE_LINE,
        "1\n2\n3\n"
    );


test!(
        test_call_global,
        "function a() { print b; } BEGIN { b = 5; a(); }",
        ONE_LINE,
        "5\n"
    );

test!(
        test_func_call_arr,
        "function a(array) { print array[0]; } BEGIN { arr[0] = 5; a(arr) }",
        ONE_LINE,
        "5\n"
    );

test!(
        test_scalar_func_call,
        "function a(b,c,d) {  print (b + c + d); }  BEGIN { a(1,2,3); }",
        ONE_LINE,
        "6\n"
    );
test!(
        test_assign_arg,
        "function takes(a) { a = 2; print a; } BEGIN {takes(1) }",
        ONE_LINE,
        "2\n"
    );
test!(
        test_scalar_call_str_const_inlined,
        "function f(a) { print a; } BEGIN { f(\"1\") }",
        ONE_LINE,
        "1\n"
    );
test!(test_scalar_call_str_const_var,"function f(ss) { print ss; } BEGIN { s = \"s\";  f(s) }",ONE_LINE,"s\n");

test!(test_simple_return,"function a() { return 2 } BEGIN { print a() }",ONE_LINE,"2\n");

test!(test_ret_scalar_func_call,"function a(b,c,d) {  print (b + c + d); }  BEGIN { print a(1,2,3); }",ONE_LINE,"6\n\n");

test!(test_ret_string_func_call,"function a(b,c,d) { return b  c  d; }  BEGIN { print a(\"1\",\"2\",\"3\"); }",ONE_LINE,"123\n");
test!(test_scalar_call_simple,"function f(scl) { print scl; } BEGIN { scalar = 5;f(scalar) }",ONE_LINE,"5\n");
test!(test_mixed_call,"function f(arr, scalar, arr2) { print arr[0]; print scalar; print arr2[1] } BEGIN { global_a_1[0] = 1; scalar = \"scalar\"; global_arr_2[1] = 2; f(global_a_1, scalar, global_arr_2) }",ONE_LINE,"1\nscalar\n2\n"
);

test!(test_str_to_float_0, "BEGIN { print 1 + \"1a\" }", ONE_LINE, "2\n");
test!(test_str_to_float_1, "BEGIN { print 1 + \"1.a\" }", ONE_LINE, "2\n");
test!(test_str_to_float_2, "BEGIN { print 1 + \"1.3a\" }", ONE_LINE, "2.3\n");
test!(test_str_to_float_3, "BEGIN { print 1 + \"1.3..a\" }", ONE_LINE, "2.3\n");
test!(test_str_to_float_4, "BEGIN { print 1 + \".1.3..a\" }", ONE_LINE, "1.1\n");

test!(test_native_int_0,"BEGIN { print int(\"123\") }", ONE_LINE, "123\n");
test!(test_native_int_1,"BEGIN { print int(\"33cc\") }", ONE_LINE, "33\n");
test!(test_native_int_2,"BEGIN { print int(\"1\" \"2\" \"3\" \"a\") }", ONE_LINE, "123\n");
test!(test_native_int_3,"BEGIN { print int(5) }", ONE_LINE, "5\n");
test!(test_native_int_4,"BEGIN { print int(\"\") }", ONE_LINE, "0\n");
test!(test_native_int_5,"BEGIN { print int(2.999) }", ONE_LINE, "2\n");
test!(test_native_int_6,"BEGIN { print int(-2.999) }", ONE_LINE, "-2\n");

test!(test_native_lower_0, "BEGIN { print tolower(\"ABCabc\"); }", ONE_LINE, "abcabc\n");
test!(test_native_lower_1, "BEGIN { print tolower(\"\"); }", ONE_LINE, "\n");
test!(test_native_lower_2, "BEGIN { print tolower(\"..--=\"); }", ONE_LINE, "..--=\n");
test!(test_native_lower_3, "BEGIN { print tolower(\"≥≥≥≥\"); }", ONE_LINE, "≥≥≥≥\n");

test!(test_native_sin_float_0, "BEGIN { print sin(0);  }", ONE_LINE, "0\n");
test!(test_native_sin_float_1, "BEGIN { print (sin(3.141592) < 0.0001); }", ONE_LINE, "1\n");
test!(test_native_sin_float_2, "BEGIN { print (sin(3.141592/2) > 0.999); }", ONE_LINE, "1\n");
test!(test_native_sin_float_3, "BEGIN { print (sin(3.141592/2) <= 1)  }", ONE_LINE, "1\n");
test!(test_native_sin_str, "BEGIN { print sin(\"0\"); } ", ONE_LINE, "0\n");
test!(test_native_sin_int, "BEGIN { print int(100 * sin(123)) }", ONE_LINE, "-45\n");
test!(test_native_sin_int_concat, "BEGIN { print int(100 * sin(\"1\" \"2\" \"3\")) }", ONE_LINE, "-45\n");

test!(test_native_cos_0, "BEGIN { print int(100*cos(1)) }", ONE_LINE, "54\n");
test!(test_native_cos_1, "BEGIN { print int(100*cos(0)) }", ONE_LINE, "100\n");
test!(test_native_cos_2, "BEGIN { print int(100*cos(3.141592)) }", ONE_LINE, "-99\n");
test!(test_native_cos_3, "BEGIN { print int(100*cos(3.141592/2)) }", ONE_LINE, "0\n");
test!(test_native_cos_4, "BEGIN { print int(100*cos(1231231231231)) }", ONE_LINE, "-97\n");
test!(test_native_cos_str, "BEGIN { print int(100*cos(\"3.141592\")) }", ONE_LINE, "-99\n");
test!(test_native_cos_str_concat, "BEGIN { print int(100*cos(\"3\" \".1415\")) }", ONE_LINE, "-99\n");

test!(test_native_log_0, "BEGIN { print int(100*log(3)) }", ONE_LINE, "109\n");
test!(test_native_log_1, "BEGIN { print int(100*log(0.123)) }", ONE_LINE, "-209\n");
test!(test_native_log_2, "BEGIN { print int(100*log(123.123)) }", ONE_LINE, "481\n");
test!(test_native_log_3, "BEGIN { print int(100*log(\"123.123\")) }", ONE_LINE, "481\n");

test!(test_native_sqrt_0, "BEGIN { print sqrt(100) }", ONE_LINE, "10\n");
test!(test_native_sqrt_1, "BEGIN { print sqrt(9) }", ONE_LINE, "3\n");
test!(test_native_sqrt_2, "BEGIN { print int(100*sqrt(3))}", ONE_LINE, "173\n");
test!(test_native_sqrt_3, "BEGIN { print sqrt(\"100\") }", ONE_LINE, "10\n");
test!(test_native_sqrt_4, "BEGIN { print sqrt(\"1\" \"0\" \"0\") }", ONE_LINE, "10\n");

test!(test_native_exp_0, "BEGIN { print int(100*exp(1)) }", ONE_LINE, "271\n");
test!(test_native_exp_1, "BEGIN { print exp(0) }", ONE_LINE, "1\n");
test!(test_native_exp_2, "BEGIN { print exp(\"0\") }", ONE_LINE, "1\n");
test!(test_native_exp_3, "BEGIN { print int(exp(\"0\" \"1\" \"2\"))}", ONE_LINE, "162754\n");
test!(test_native_exp_4, "BEGIN { print int(100*exp(1.1)) }", ONE_LINE, "300\n");
test!(test_native_exp_5, "BEGIN { print int(100*exp(-1)) }", ONE_LINE, "36\n");

test!(test_s_rand_no_args, "BEGIN { srand(); }", ONE_LINE, "");
test!(test_s_rand_0, "BEGIN { srand(123); print srand(5);}", ONE_LINE, "123\n");
test!(test_s_rand_1, "BEGIN { srand(123); print srand(5); print srand(6);}", ONE_LINE, "123\n5\n");
test!(test_s_rand_2, "BEGIN { srand(123); x0 = rand(); x00 = rand(); srand(123); x1 = rand(); x11=rand(); print (x0 == x1); print (x00 == x11); print (x0 != x00)}", ONE_LINE, "1\n1\n1\n");

test!(test_atan2_0, "BEGIN { print int(1000*atan2(1, 1)) }", ONE_LINE, "785\n");
test!(test_atan2_1, "BEGIN { print atan2(0, 1) }", ONE_LINE, "0\n");
test!(test_atan2_2, "BEGIN { print atan2(\"\", 1) }", ONE_LINE, "0\n");
test!(test_atan2_3, "BEGIN { a = \"\"; print atan2(a, 1) }", ONE_LINE, "0\n");
test!(test_atan2_4, "BEGIN { print int(1000*atan2(\"0.3\", 0.1)) }", ONE_LINE, "1249\n");
test!(test_atan2_5, "BEGIN { print int(1000*atan2(\"2\", \"3\")) }", ONE_LINE, "588\n");

test!(test_length_0, "BEGIN { print length(1111); print length(\"1234\"); print length(\"\") }", ONE_LINE, "4\n4\n0\n");
test!(test_length_1, "BEGIN { print length(1) + length(12); }", ONE_LINE, "3\n");
test!(test_length_2, "{ a += length($2); } END { print a }", "1 22 333\n4444 55555 666666\n7777777 88888888 999999999", "15\n");
test!(test_length_3, "{ print  length(); }", "123\n33345", "3\n5\n");
test!(test_length_4, "BEGIN { print  length(45e2); }", ONE_LINE, "4\n");

test!(test_split_0, "BEGIN { print split(a,b); print b[0] }", ONE_LINE, "0\n\n");
test!(test_split_1, "BEGIN { split(a,b,c); print b[0] }", ONE_LINE, "\n");
test!(test_split_2, "BEGIN { split(\"abc def\", b); print b[1]; print b[2] }", ONE_LINE, "abc\ndef\n");
test!(test_split_ere_0, "BEGIN { split(\"abcZZZdef\", b, \"Z+\"); print b[1]; print b[2] }", ONE_LINE, "abc\ndef\n");
test!(test_split_ere_1, "BEGIN { split(\"abc4def\", b, 4); print b[1]; print b[2] }", ONE_LINE, "abc\ndef\n");
test!(test_split_overwrite, "BEGIN { b[1] = \"should be free'd\"; b[5] = \"existing\";  split(\"abc def\",  b); print b[1]; print b[2]; print b[5]; }", ONE_LINE, "abc\ndef\n\n");
test!(test_split_ret_0, "BEGIN { print split(\"abc def\", b); }", ONE_LINE, "2\n");
test!(test_split_ret_1, "BEGIN { print split(\"abcdef\", b); }", ONE_LINE, "1\n");
test!(test_split_ret_2, "BEGIN { print split(\"\", b); }", ONE_LINE, "0\n");
test!(test_split_fs_clears, "BEGIN { a[1] = 1; a[2] = 2; a[3] = 3; split(\"X Y\", a); print a[1]; print a[2]; print a[3] }", ONE_LINE, "X\nY\n\n");
test!(test_split_ere_clears, "BEGIN { a[1] = 1; a[2] = 2; a[3] = 3; split(\"XQQQQQY\", a, \"Q+\"); print a[1]; print a[2]; print a[3] }", ONE_LINE, "X\nY\n\n");

test!(test_array_unrolled, "BEGIN { a[1] = 3; print a[\"1\"]; print a[\"1\"]; print a[\"1\"]; print a[\"1\"]; print a[\"1\"] }", ONE_LINE, "3\n3\n3\n3\n3\n");
test!(test_constants_loop, "BEGIN { a[1] = 1; while(x++<10) { print a[\"1\"] } }", ONE_LINE, "1\n1\n1\n1\n1\n1\n1\n1\n1\n1\n");
test!(test_array_loop, "BEGIN { a[1] = 4; while(x++<10) { print a[\"1\"]; } }", ONE_LINE, "4\n4\n4\n4\n4\n4\n4\n4\n4\n4\n");
test!(test_array_print_twice, "BEGIN { a[1.0] = 3;  print a[1.0]; print a[1.0]  }", ONE_LINE, "3\n3\n");
test!(test_array_exact_match_no_dec, "BEGIN { a[1] = 3; a[\"1\"] = 4; print a[1]; print a[\"1\"]; }", ONE_LINE, "4\n4\n");
test!(test_array_exact_mismatch, "BEGIN { a[1.1] = 3; a[\"1\"] = 4; print a[1.1]; print a[\"1\"]; }", ONE_LINE, "3\n4\n");
test!(test_array_exact_match_decimal, "BEGIN { a[1.0] = 3; a[\"1\"] = 4; print a[1.0]; print a[\"1\"]; }", ONE_LINE, "4\n4\n");

test!(test_idx_inexact_0, "BEGIN { a[\"1.1a\"] = 4; a[1.1] = 3; print a[1.1]; print a[\"1.1a\"] }", ONE_LINE, "3\n4\n");
test!(test_idx_inexact_1, "BEGIN { a[\"1.\"] = 4; a[1.] = 3; print a[\"1.\"]; print a[1.] }", ONE_LINE, "4\n3\n");
test!(test_idx_exact_noe, "BEGIN { a[\"1.1\"] = 4; a[1.1] = 3; print a[1.1]; print a[\"1.1\"] }", ONE_LINE, "3\n3\n");
test!(test_idx_exact_e, "BEGIN { a[\"1.1e1\"] = 4; a[11] = 3; print a[\"1.1e1\"]; print a[11] }", ONE_LINE, "4\n3\n");

test!(test_native_substr_0, "BEGIN { a = \"abc\"; print substr(a, 1, 1); }", ONE_LINE, "a\n");
test!(test_native_substr_1, "BEGIN { a = \"abc\"; print substr(a, 1, 2); }", ONE_LINE, "ab\n");
test!(test_native_substr_2, "BEGIN { a = \"abc\"; print substr(a, 1, 3); }", ONE_LINE, "abc\n");
test!(test_native_substr_3, "BEGIN { a = \"abc\"; print substr(a, 1, 0); }", ONE_LINE, "\n");
test!(test_native_substr_4, "BEGIN { a = \"abc\"; print substr(a, 1, -1); }", ONE_LINE, "\n");
test!(test_native_substr_5, "BEGIN { a = \"abcdefghi\"; print substr(a, 2, 3); }", ONE_LINE, "bcd\n");
test!(test_native_substr_6, "BEGIN { a = \"abcdefghi\"; print substr(a, 2, 3000); }", ONE_LINE, "bcdefghi\n");
test!(test_native_substr_7, "BEGIN { a = \"abcdefghi\"; print substr(a, 2, 5); }", ONE_LINE, "bcdef\n");
test!(test_native_substr_8, "BEGIN { a = \"abc\"; print substr(a, 0); }", ONE_LINE, "abc\n");
test!(test_native_substr_9, "BEGIN { a = \"abc\"; print substr(a, 1); }", ONE_LINE, "abc\n");
test!(test_native_substr_10, "BEGIN { a = \"abc\"; print substr(a, 3); }", ONE_LINE, "c\n");
test!(test_native_substr_11, "BEGIN { a = \"abc\"; print substr(a, 4); }", ONE_LINE, "\n");
test!(test_native_substr_12, "BEGIN { a = \"abc\"; print substr(a, -1); }", ONE_LINE, "abc\n");
test!(test_native_substr_13, "BEGIN { a = \"abc\"; print substr(a, 1.5); }", ONE_LINE, "abc\n");
test!(test_native_substr_14, "BEGIN { a = \"abc\"; print substr(a, 1.99999); }", ONE_LINE, "abc\n");

test!(test_native_index_0, "BEGIN { a = \"abc111ee\"; print index(a, \"abc\") }", ONE_LINE, "1\n");
test!(test_native_index_1, "BEGIN { a = \"abc111ee\"; print index(a, \"abcD\") }", ONE_LINE, "0\n");
test!(test_native_index_2, "BEGIN { a = \"abc111ee\"; print index(a, \"bc111\") }", ONE_LINE, "2\n");
test!(test_native_index_3, "BEGIN { a = \"abc111ee\"; print index(a, \"e\") }", ONE_LINE, "7\n");
test!(test_native_index_4, "BEGIN { a = \"abc111ee\"; print index(a, \"ee\") }", ONE_LINE, "7\n");
test!(test_native_index_5, "BEGIN { a = \"a\"; print index(a, \"aaa\") }", ONE_LINE, "0\n");
test!(test_native_index_6, "BEGIN { a = \"\"; print index(a, \"aaa\") }", ONE_LINE, "0\n");
test_except!(test_native_index_7, EMPTY_INDEX_PROGRAM, ONE_LINE, "1\n", Awk::Onetrueawk as usize);

test!(test_native_sub_assign, "BEGIN { c = \"a\"; print sub(\"a\", \"b\", c); }", ONE_LINE, "1\n");

test!(test_native_gsub_0, r#"BEGIN {a = "aaa"; print gsub("a", "zz", a); print a;}"#, ONE_LINE, "3\nzzzzzz\n");
test!(test_native_gsub_1, r#"BEGIN {a = "aaa"; print gsub("a*", "zz", a); print a;}"#, ONE_LINE, "1\nzz\n");
test!(test_native_gsub_2, r#"BEGIN {a = "aaa"; print gsub("a+", "zz", a); print a;}"#, ONE_LINE, "1\nzz\n");
test!(test_native_gsub_3, r#"BEGIN {a = "Qa&&&&aQ"; print gsub("a\\&*a", "zz", a); print a;}"#, ONE_LINE, "1\nQzzQ\n");
test!(test_native_gsub_4, r#"BEGIN {a = "----"; print gsub("-", "--", a); print a;}"#, ONE_LINE, "4\n--------\n");

test!(test_native_sub_var_0, "BEGIN { a = \"aaa\"; print gsub(\"a\", \"b\", a); print a; }", ONE_LINE, "3\nbbb\n");
test!(test_native_sub_var_1, "BEGIN { a = \"aaa\"; print sub(\"a\", \"b\", a); print a; }", ONE_LINE, "1\nbaa\n");
test!(test_native_sub_var_2, "BEGIN { a = \"aaa\"; print sub(\"a\", \"bbb\", a); print a; }", ONE_LINE, "1\nbbbaa\n");
test!(test_native_sub_var_3, "BEGIN { a = \"caa\"; print sub(\"a\", \"bbb\", a); print a; }", ONE_LINE, "1\ncbbba\n");
test!(test_native_sub_var_4, "BEGIN { a = \"aab\"; print sub(\"b\", \"ZZZZ\", a); print a; }", ONE_LINE, "1\naaZZZZ\n");
test!(test_native_sub_var_5, "BEGIN { a = \"aaa\"; print sub(\"a\", \"\", a); print a; }", ONE_LINE, "1\naa\n");
test!(test_native_sub_var_6, "BEGIN { a = \"aaa\"; print sub(\"aaa\", \"\", a); print a; }", ONE_LINE, "1\n\n");
test!(test_native_sub_var_7, "BEGIN { a = \"aaa\"; print sub(\"aaaa\", \"\", a); print a; }", ONE_LINE, "0\naaa\n");

test!(test_native_sub_array_0, "BEGIN { a[1] = \"aaa\"; print sub(\"a\", \"b\", a[1]); print a[1]; }", ONE_LINE, "1\nbaa\n");
test!(test_native_sub_array_1, "BEGIN { a[1] = \"aaa\"; print sub(\"a\", \"bbb\", a[1]); print a[1]; }", ONE_LINE, "1\nbbbaa\n");
test!(test_native_sub_array_2, "BEGIN { a[1] = \"caa\"; print sub(\"a\", \"bbb\", a[1]); print a[1]; }", ONE_LINE, "1\ncbbba\n");
test!(test_native_sub_array_3, "BEGIN { a[1] = \"aab\"; print sub(\"b\", \"ZZZZ\", a[1]); print a[1]; }", ONE_LINE, "1\naaZZZZ\n");
test!(test_native_sub_array_4, "BEGIN { a[1] = \"aaa\"; print sub(\"a\", \"\", a[1]); print a[1]; }", ONE_LINE, "1\naa\n");
test!(test_native_sub_array_5, "BEGIN { a[1] = \"aaa\"; print sub(\"aaa\", \"\", a[1]); print a[1]; }", ONE_LINE, "1\n\n");
test!(test_native_sub_array_6, "BEGIN { a[1] = \"aaa\"; print sub(\"aaaa\", \"\", a[1]); print a[1]; }", ONE_LINE, "0\naaa\n");

test!(test_string_escaping, r#"BEGIN { print "\a\b\t\n\v\f\r"; }"#, ONE_LINE, vec![0x7,0x8,0x9,0xa,0xb,0xc,0xd,0xa]);
test!(test_quote_escaping, r#"BEGIN { print "-\"-"; }"#, ONE_LINE, "-\"-\n");
test!(test_slash_escapitng, r#"BEGIN { print "/ \\ \\\\"; }"#, ONE_LINE, "/ \\ \\\\\n");

test!(test_no_ret_0, "function f() { } BEGIN { print f() }", ONE_LINE, "\n");
test!(test_no_ret_1, "function f() { } BEGIN { print (f()==0) }", ONE_LINE, "1\n");
test!(test_no_ret_2, "function f() { } BEGIN { print (f()==\"\") }", ONE_LINE, "1\n");
test!(test_no_ret_3, "function f() { } BEGIN { print (f()==1) }", ONE_LINE, "0\n");
test!(test_no_ret_4, "function f() { } { print (f()==$1) }", "1\n", "0\n");
test!(test_no_ret_5, "function f() { } { print (f()==$1) }", "0\n", "1\n");

test!(test_logical_or_0, "\
    function f() { print 333; return 1; } \
    function g() { print 555; return 0; } \
    BEGIN { print (f() || g()); }", ONE_LINE, "333\n1\n");
test!(test_logical_or_1, "function f() { print 333; return 0; } function g() { print 555; return 1; } BEGIN { print (f() || g()); }", ONE_LINE, "333\n555\n1\n");
test!(test_logical_or_2, "function f() { print 333; return 0; } function g() { print 555; return 0; } BEGIN { print (f() || g()); }", ONE_LINE, "333\n555\n0\n");

test!(test_ez1, "BEGIN { a = \"2\"; }", "", "");
test!(test_ez2, "BEGIN { a = 2; print a; }", "", "2\n");

test!(test_native_sub_amp_0, "BEGIN { a = \"a\"; sub(\"a\", \"-&-\", a); print a; }", ONE_LINE, "-a-\n");
test!(test_native_sub_amp_amp, "BEGIN { a = \"a\"; sub(\"a\", \"-&-&-\", a); print a; }", ONE_LINE, "-a-a-\n");
test!(test_native_sub_amp_esc_1, "BEGIN { a = \"a\"; sub(\"a\", \"-\\\\&-\", a); print a; }", ONE_LINE, "-&-\n");
test!(test_native_sub_amp_esc_2, "BEGIN { a = \"a\"; sub(\"a\", \"-\\\\&&-\", a); print a; }", ONE_LINE, "-&a-\n");
test!(test_native_sub_amp_4, "BEGIN { a = \"aaabc\"; sub(\"a+\", \"-&.&REPL-\", a); print a; }", ONE_LINE, "-aaa.aaaREPL-bc\n");
test_except!(test_native_sub_escaping, SUB_ESCAPING, ONE_LINE, "\\\n", Awk::Onetrueawk as usize);

test!(test_fs_1, "{ print $2; FS = \"b\"; }", "abc\nabc\nabc", "\nc\nc\n");
test!(test_fs_2, "{ print $2; FS = \"a\"; }", "abc\nabc\nabc", "\nbc\nbc\n");

test!(test_rs_0, "BEGIN { RS = 1; } { print $0; }", "1234123412341234", "\n234\n234\n234\n234\n");
test!(test_rs_1, "BEGIN { RS = 1; } { print $2; }", "1234123412341234", "\n\n\n\n\n");
test!(test_rs_2, "BEGIN { RS = 1; } { print $1; }", "1234123412341234", "\n234\n234\n234\n234\n");
test_except!(test_rs_3, "BEGIN { RS = 1 } { print $0; RS = 2;  }", "123123", "\n\n31\n3\n", Awk::Goawk as usize);


test!(test_match_0, "BEGIN { print match(\"abc\", \"d\"); }", "", "0\n");
test!(test_match_1, "BEGIN { print match(\"abc\", \"d\"); print RSTART; print RLENGTH; }", "", "0\n0\n-1\n");
test!(test_match_2, "BEGIN { print match(\"abc\", \"a\"); }", "", "1\n");
test!(test_match_3, "BEGIN { print match(\"abc\", \"a\"); print match(\"abc\", \"b\"); print match(\"abc\", \"c\"); }", "", "1\n2\n3\n");
test!(test_match_4, "BEGIN { print match(\"abbbbc\", \"b+\"); print RSTART; print RLENGTH; }", "", "2\n2\n4\n");

// TODO: Things I have yet to impl

// test!(test_nf_0, "{ print NF }", ONE_LINE, "3\n");
// test!(test_nf_1, "{ print NF }", "1 2 3\n1 2 3 4\n", "3\n4\n");
// test!(test_nf_2, "{ print NF; $4 = \"a\"; print NF; print $0 }", ONE_LINE, "3\n4\n1 2 3 4\n");
// test!(test_nf_3, "{ print NF; $5 = \"a\"; print NF; print $0 }", ONE_LINE, "3\n5\n1 2 3 4  5\n");

// test!(test_ofs_print_sep, "BEGIN { print 1, 2, 3; OFS = \"--\"; print 1,2,3 }", ONE_LINE, "1 2 3\n1--2--3\n");

// test!(test_native_col_0_sub_0, "{ sub(\"a\", \"b\"); print $0; }", "aaa", "baa\n");
// test!(test_native_col_0_sub_1, "{ sub(\"a\", \"b\"); print $0; }", "aaa", "baa\n");
// test!(test_native_col_0_sub_2, "{ sub(\"a\", \"b\"); print $0; }", "caa", "baa\n");
// test!(test_native_col_0_sub_3, "{ sub(\"a\", \"b\"); print $0; }", "aab", "baa\n");
// test!(test_native_col_0_sub_4, "{ sub(\"a\", \"b\"); print $0; }", "aaa", "baa\n");
// test!(test_native_col_0_sub_5, "{ sub(\"a\", \"b\"); print $0; }", "aaa", "baa\n");
// test!(test_native_col_0_sub_6, "{ sub(\"a\", \"b\"); print $0; }", "aaa", "baa\n");

const PI: &'static str = "    +3.14";
// test!(space_rule_simple, "{ print length($1); }", "    abc", "abc");
// test!(gawk_strnum_space_rule_0, "{ print($1 == \"+3.14\") }", PI, "1\n");
// test!(gawk_strnum_space_rule_1, "{ print($1 == 3.14) }", PI, "1\n");
// test!(test_mixed_array,"BEGIN {SUBSEP = \"-\"; a[0,1] = 3 ; print a[\"0-1\"]; }",ONE_LINE,"3\n");

// test!(test_col_asgn_0, "{ $1 = \"zz\"; print $0 }", "1  2   3\n  4  5     6    ","zz 2 3\nzz 5 6\n");
// #[test]
// fn setting_argv() {
//     test_runner_multifile("adding_to_argv",
//                           "{ print $0; ARGV[2] = \"b\"; ARGC = 3; print ARGC }",
//                           vec![("file_a_data", "a"), ("file_b_data", "b")],
//                           "file_a_data\nfile_b_data\n", 0);
// }



#![allow(dead_code)]

use std::fs;
use std::io::Write;
use crate::{analyze, lex, parse, Symbolizer, compile_and_capture};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const ONE_LINE: &'static str = "1 2 3\n";
const REDIRECT: &'static str = "2 3 4 5\n";
const NUMBERS: &'static str = "1 2 3\n4 5 6\n7 8 9\n";
const NUMBERS2: &'static str = "1 2 3 4\n4 5 6 4\n7 8 9 7";
const FLOAT_NUMBERS: &'static str = "1.1 2.2 3.3\n4.4 5.5 6.6\n7.7 8.8 9.9";
const NUMERIC_STRING: &'static str = "1 2 3\n04 005 6\n07 8 9";

fn test_once(interpreter: &str, prog: &str, file: &PathBuf) -> (String, Duration) {
    // Run a single awk once and capture the output
    let start = Instant::now();
    let output = std::process::Command::new(interpreter)
        .args(vec![prog, file.to_str().unwrap()])
        .output()
        .unwrap();
    let dir = start.elapsed();
    (
        String::from_utf8(output.stdout).expect("cannot convert output to utf8"),
        dir,
    )
}

fn long_number_file() -> String {
    let mut string = String::new();
    let mut counter: i16 = 0;
    for _ in 0..1_000 {
        for idx in 0..1_000 {
            counter = counter.wrapping_add(idx);
            string.push_str(&format!("{} ", counter));
        }
        string.push('\n');
    }
    string
}

fn test_against(interpreter: &str, prog: &str, oracle_output: &str, file: &PathBuf) {
    match std::process::Command::new(interpreter).output() {
        Ok(_) => {}
        Err(_err) => return, // this interpreter doesn't exist
    }

    let output = test_once(interpreter, prog, file);

    assert_eq!(
        output.0,
        oracle_output,
        "LEFT {} - RIGHT oracle didnt match",
        interpreter
    );
}

const PERF_RUNS: u128 = 15;

fn append_result(test_name: &str, interp: &str, our_total: u128, other_total: u128) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("text_results")
        .unwrap();

    let str = format!("{}\t{}\t{}\tjawk\t{}\n", test_name, interp, other_total, our_total);
    file.write_all(str.as_bytes()).unwrap();
}

fn test_perf(test_name: &str, interpreter: &str, prog: &str, oracle_output: &str, file: &PathBuf) {
    match std::process::Command::new(interpreter).output() {
        Ok(_) => {}
        Err(_err) => return, // this interpreter doesn't exist
    }
    let mut our_total = 0;
    let mut other_total = 0;

    for _ in 0..PERF_RUNS {
        let our_result = test_once("./target/release/jawk", prog, file);
        other_total += test_once(interpreter, prog, file).1.as_micros();
        our_total += our_result.1.as_micros();
        assert_eq!(our_result.0, oracle_output, "perf-test : LEFT jawk, RIGHT oracle didn't match. DID YOU DO A RELEASE BUILD?");
    }

    // 6ms per run
    if our_total >= 6 * PERF_RUNS * 1000 {
        append_result(test_name, interpreter, our_total, other_total);
    }

    assert!(our_total < other_total || our_total < 6 * PERF_RUNS * 1000, "perf-test: jawk={}ms {}={}ms", our_total / 1000, interpreter, other_total / 1000);
}

fn test_it<S: AsRef<str>>(test_name: &str, prog: &str, file: S, oracle_output: &str) {
    println!("Program:\n{}", prog);
    let mut symbolizer = Symbolizer::new();
    let program = analyze(parse(lex(&prog, &mut symbolizer).unwrap(), &mut symbolizer)).unwrap();
    println!("Ast:\n{}", &program);

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("tmp");
    std::fs::write(file_path.clone(), file.as_ref()).unwrap();
    let file_path_string = file_path.to_str().unwrap().to_string();
    let res = compile_and_capture(program, &[file_path_string], &mut symbolizer, false).unwrap();
    assert_eq!(
        res.strings_in(), res.strings_out(),
        "runtime strings_in didn't match string_out. Possible mem leak `{}` in vs `{}` out",
        res.strings_in(), res.strings_out()
    );
    assert_eq!(res.output(), oracle_output, "LEFT jawk -- RIGHT oracle, did not match");

    test_against("awk", prog, oracle_output, &file_path);
    // test_against("mawk", prog, oracle_output, &file_path);
    test_against("goawk", prog, oracle_output, &file_path);
    test_against("onetrueawk", prog, oracle_output, &file_path);

    if std::env::vars().any(|f| f.0 == "jperf" && (f.1 == "true" || f.1 == "true\n")) {
        test_perf(test_name, "awk", prog, oracle_output, &file_path);
        test_perf(test_name, "mawk", prog, oracle_output, &file_path);
        test_perf(test_name, "goawk", prog, oracle_output, &file_path);
        test_perf(test_name, "onetrueawk", prog, oracle_output, &file_path);
    }
}

macro_rules! test {
    ($name:ident,$prog:expr,$file:expr,$stdout:expr) => {
        #[test]
        fn $name() {
            test_it(stringify!($name), $prog, $file, $stdout);
        }
    };
}

test!(test_print_begin_int, "BEGIN {print 1;}", ONE_LINE, "1\n");
test!(test_print_int, "{print 1;}", ONE_LINE, "1\n");
test!(test_print_str, "BEGIN {print \"abc\";}", ONE_LINE, "abc\n");
test!(test_print_str_loop, "{print \"abc\";}", ONE_LINE, "abc\n");
test!(test_just_begin, "BEGIN { print 1; }", ONE_LINE, "1\n");

test!(
    test_assign_undef_to_undef,
    "BEGIN { x = x; }",
    ONE_LINE,
    ""
);
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
    test_assign_then_print,
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
    test_while,
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
test!(test_concat_unused, "BEGIN { z = \"abc\" \"def\"; }", ONE_LINE, "");
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
test!(
    test_assign_ops,
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

// TODO: Efficient IO
// test!(test_pattern_long, "$1 == $4", long_number_file(), ".");

// TODO: Numeric strings
// test!(
//     test_numeric_string1,
//     "{ print ($1 > 2 ) }",
//     NUMERIC_STRING,
//     "."
// );

// // const NUMERIC_STRING: &'static str = "1 2 3\n04 005 6\n07 8 9";
// test!(
//     test_numeric_string2,
//     "{ print ($0 < $1 ) }",
//     NUMERIC_STRING,
//     "0\n0\n0\n"
// );
// test!(
//     test_numeric_string3,
//     "{ print (\"04\" > \"005\") }",
//     NUMERIC_STRING,
//     "1\n1\n1\n"
// );
// test!(
//     test_numeric_string4,
//     "{ print (\"04\" >= \"005\") }",
//     NUMERIC_STRING,
//     "1\n1\n1\n"
// );
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
    test_array_get_1,
    "BEGIN { print a[0] }",
    ONE_LINE,
    "\n"
);

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
    test_mixed_array,
    "BEGIN {SUBSEP = \"-\"; a[0,1] = 3 ; print a[\"0-1\"]; }",
    ONE_LINE,
    "3\n"
);

test!(
    test_multi_in_array_1,
    "BEGIN { a[5] = 3; b[3] = 2; b[2] = 1; b[1] = 5; print 3 in b in b; }",
    ONE_LINE,
    "1\n"
);

test!(
    test_perf_array,
    "BEGIN { while (x<40000) { arr[x] = 1+x++  }; sum = 0; x = 0; while (x++ < 40000) { sum += arr[x] }; print sum}",
    ONE_LINE,
    "800020000\n"
);

test!(
    test_two_arrays,
    "BEGIN { a[0] = 1; a[1] =1; b[0] = 2; b[1] = 3; x=2; while (x++ < 40) { a[x] = a[x-1] + a[x-2]; b[x] = b[x-1] + b[x-2]; print a[x]; print b[x] }}",
    ONE_LINE,
    "1\n3\n1\n3\n2\n6\n3\n9\n5\n15\n8\n24\n13\n39\n21\n63\n34\n102\n55\n165\n89\n267\n144\n432\n233\n699\n377\n1131\n610\n1830\n987\n2961\n1597\n4791\n2584\n7752\n4181\n12543\n6765\n20295\n10946\n32838\n17711\n53133\n28657\n85971\n46368\n139104\n75025\n225075\n121393\n364179\n196418\n589254\n317811\n953433\n514229\n1542687\n832040\n2496120\n1346269\n4038807\n2178309\n6534927\n3524578\n10573734\n5702887\n17108661\n9227465\n27682395\n14930352\n44791056\n24157817\n72473451\n39088169\n117264507\n"
);

test!(
    test_simple_concat,
    "BEGIN { a[0] = 1 1 }",
    ONE_LINE,
    ""
);
test!(
    test_leak,
    "BEGIN { while (x++ < 1) { }}",
    ONE_LINE,
    ""
);

test!(
    test_array_with_str,
    "BEGIN { while (x++ < 30) { a[x] = a[x-1] \".\"; print a[x] }}",
    ONE_LINE,
    ".\n..\n...\n....\n.....\n......\n.......\n........\n.........\n..........\n...........\n............\n.............\n..............\n...............\n................\n.................\n..................\n...................\n....................\n.....................\n......................\n.......................\n........................\n.........................\n..........................\n...........................\n............................\n.............................\n..............................\n"
);

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

test!(drop_on_end,
    "BEGIN { x = 1; x = \"A\"; x = 4}",
    ONE_LINE,
    ""
);

test!(
    test_double_break_loop,
    "BEGIN {while(1) {     z=0; while(1) {if(z==30){break}z++;a++}        y++; if(y==40) {break}} print y; print a;}",
    ONE_LINE,
    "40\n1200\n"
);

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
    "1\n");

test!(
    test_func_global_float_only,
    "function uses_nil() { print global_1;  } BEGIN { global_1 = 3; uses_nil();}",
    ONE_LINE,
    "3\n");

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
    "1\n");

test!(
    test_func_call_1,
    "function a(arr) { arr[0] = 123; } BEGIN { a(b); print b[0]; }",
    ONE_LINE,
    "123\n"
);

test!(test_func_call_2,
    "function a(arg) { print $arg } { a(1); a(2); a(3); }",
    ONE_LINE,
    "1\n2\n3\n"
);

test!(test_call_global,
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
test!(test_assign_arg, "function takes(a) { a = 2; print a; } BEGIN {takes(1) }", ONE_LINE, "2\n");
test!(test_scalar_call_str_const_inlined, "function f(a) { print a; } BEGIN { f(\"1\") }", ONE_LINE, "1\n");
test!(test_scalar_call_str_const_var, "function f(ss) { print ss; } BEGIN { s = \"s\";  f(s) }", ONE_LINE, "s\n");

test!(test_simple_return, "function a() { return 2 } BEGIN { print a() }", ONE_LINE, "2\n");

test!(
    test_ret_scalar_func_call,
    "function a(b,c,d) {  print (b + c + d); }  BEGIN { print a(1,2,3); }",
    ONE_LINE,
    "6\n\n"
);

test!(
    test_ret_string_func_call,
    "function a(b,c,d) { return b  c  d; }  BEGIN { print a(\"1\",\"2\",\"3\"); }",
    ONE_LINE,
    "123\n"
);


test!(test_mixed_call,
"function f(arr, scalar, arr2) { print arr[0]; print scalar; print arr2[1] } BEGIN { global_a_1[0] = 1; scalar = \"scalar\"; global_arr_2[1] = 2; f(global_a_1, scalar, global_arr_2) }",
    ONE_LINE,
    "1\nscalar\n2\n"
);

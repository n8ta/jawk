mod integration_tests;
mod io_capture;

#[cfg(test)]
#[allow(dead_code)]
use std::cell::RefCell;
use crate::{analyze, lex, parse, Symbolizer};
use std::fs;
use std::io::{Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::from_utf8_unchecked;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use crate::compiler::{compile, validate_program};
use crate::test::io_capture::IoCapture;
use crate::vm::VirtualMachine;

const SUB_ESCAPING: &'static str = r#"BEGIN { a = "a"; sub("a", "\\\\", a); print a }"#;
const SUB_RULES: &'static str = r##"BEGIN { a = "a"; sub("a", "-\\\\a-", a); print a; }"##;
const ONE_LINE: &'static str = "1 2 3\n";
const REDIRECT: &'static str = "2 3 4 5\n";
const NUMBERS: &'static str = "1 2 3\n4 5 6\n7 8 9\n";
const NUMBERS2: &'static str = "1 2 3 4\n4 5 6 4\n7 8 9 7";
const FLOAT_NUMBERS: &'static str = "1.1 2.2 3.3\n4.4 5.5 6.6\n7.7 8.8 9.9";
const NUMERIC_STRING: &'static str = "1 2 3\n04 005 6\n07 8 9";
const ABC: &'static str = "abc\nabc\nabc";
const PERF_ARRAY_PROGRAM: &'static str = "BEGIN { while (x<40000) { arr[x] = 1+x++  }; sum = 0; x = 0; while (x++ < 40000) { sum += arr[x] }; print sum}";
const EMPTY_INDEX_PROGRAM: &'static str = "BEGIN { a = \"\"; print index(a, \"\") }";
const TTX1: &'static str = "BEGIN {    width = 3; height = 3 ;    min_x = -2.1; max_x = 0.6;    min_y = -1.2; max_y = 1.2;    iters = 32;
        colors[0] = \".\";    colors[1] = \"-\";    colors[2] = \"+\";    colors[3] = \"*\";    colors[4] = \"%%\";    colors[5] = \"#\";    colors[6] = \"$\";    colors[7] = \"@\";    colors[8] = \" \";
    inc_y = (max_y-min_y) / height;    inc_x = (max_x-min_x) / width;    y = min_y;    for (row=0; row<height; row++) {        x = min_x;        for (col=0; col<width; col++) {            zr = zi = 0;            for (i=0; i<iters; i++) {                old_zr = zr;                zr = zr*zr - zi*zi + x;                zi = 2*old_zr*zi + y;                if (zr*zr + zi*zi > 4) { break; }            }
            idx = 0;            zzz = i*8/iters;            if (zzz < 1) {                idx = 0;            };            if (zzz < 2) {                idx = 1;            };            if (zzz < 3) {                idx = 2;            };            if (zzz < 4) {                idx = 3;            };            if (zzz < 5) {                idx = 4;            };            if (zzz < 6) {                idx = 5;            };            if (zzz < 7) {                idx = 6;            };            if (zzz < 8) {                idx = 7;            };            printf colors[idx];            x += inc_x;        }        y += inc_y;        print \"\";    }}";


fn test_once(interpreter: &str, prog: &str, file: &PathBuf) -> (Vec<u8>, Duration) {
    // Run a single awk once and capture the output
    let start = Instant::now();
    let mut args = vec![prog];
    if interpreter == "gawk" {
        args = vec!["--posix", args[0]]
    }
    args.push(file.to_str().unwrap());
    let output = std::process::Command::new(interpreter)
        .args(args)
        .output()
        .unwrap();
    let dir = start.elapsed();
    (output.stdout, dir)
}

const PERF_RUNS: u128 = 5;

pub fn test_runner<S: AsRef<str>, StdoutT: Into<Vec<u8>>>(test_name: &str, prog: &str, file: S, oracle_output: StdoutT) {
    let oracle_output: Vec<u8> = oracle_output.into();
    println!("Program:\n{}", prog);
    let mut symbolizer = Symbolizer::new();
    let program =
        analyze(parse(lex(&prog, &mut symbolizer).unwrap(), &mut symbolizer).unwrap(), &mut symbolizer).unwrap();
    // println!("Ast:\n{}", &program);

    let vm_prog = compile(program).unwrap();
    let prog_pretty = vm_prog.pretty_print();
    let prog_pretty = unsafe { String::from_utf8_unchecked(prog_pretty) };
    println!("{}", &prog_pretty);

    validate_program(&vm_prog);

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("tmp");
    std::fs::write(file_path.clone(), file.as_ref()).unwrap();
    let file_path_string = file_path.to_str().unwrap().to_string();

    let fake_stdout = Box::new(IoCapture::new());
    let fake_stderr = Box::new(IoCapture::new());

    let vm = VirtualMachine::new(
        vm_prog,
        vec![file_path_string],
        fake_stdout.clone(),
        fake_stderr.clone());

    let _ = vm.run();

    // These strings may not be valid utf but who cares it's a test
    let output = fake_stdout.collect();
    let output = unsafe { from_utf8_unchecked(&output) };
    let expected = unsafe { from_utf8_unchecked(&oracle_output) };

    assert_eq!(
        output,
        expected,
        "LEFT jawk -- RIGHT oracle, did not match"
    );

    let run_perf_tests = std::env::vars().any(|f| f.0 == "jperf" && (f.1 == "true" || f.1 == "true\n"));

    for interpreter in ["gawk", "goawk", "mawk", "onetrueawk"] {
        if (prog == PERF_ARRAY_PROGRAM && interpreter == "mawk")
            || (prog == EMPTY_INDEX_PROGRAM || prog == SUB_ESCAPING || prog == SUB_RULES) && interpreter == "onetrueawk" {
            continue;
        }
        if run_perf_tests {
            test_perf(test_name, interpreter, prog, &oracle_output, &file_path);
        } else {
            test_against(interpreter, prog, &oracle_output, &file_path);
        }
    }
}

fn test_against(interpreter: &str, prog: &str, oracle_output: &[u8], file: &PathBuf) {
    let output = test_once(interpreter, prog, file);

    assert_eq!(
        output.0, oracle_output,
        "LEFT {} - RIGHT oracle didnt match",
        interpreter
    );
}

fn test_perf(
    test_name: &str,
    interpreter: &str,
    prog: &str,
    oracle_output: &[u8],
    file: &PathBuf,
) {
    match std::process::Command::new(interpreter).output() {
        Ok(_) => {}
        Err(_err) => return, // this interpreter doesn't exist
    }
    let mut our_total = 0;
    let mut other_total = 0;

    for _ in 0..PERF_RUNS {
        let our_result = test_once("./target/release/rawk", prog, file);
        other_total += test_once(interpreter, prog, file).1.as_micros();
        our_total += our_result.1.as_micros();
        assert_eq!(
            our_result.0, oracle_output,
            "perf-test : LEFT jawk, RIGHT oracle didn't match. DID YOU DO A RELEASE BUILD?"
        );
    }

    if (other_total / PERF_RUNS) / 1000 > 5 {
        assert!(
            our_total < other_total,
            "perf-test: jawk={}ms {}={}ms",
            our_total / 1000,
            interpreter,
            other_total / 1000
        );
    }

    if other_total > our_total {
        append_result(test_name, interpreter, our_total, other_total);
    }
}


fn append_result(test_name: &str, interp: &str, our_total: u128, other_total: u128) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("text_results")
        .unwrap();

    let str = format!(
        "{}\t{}\t{}\tjawk\t{}\n",
        test_name, interp, other_total, our_total
    );
    file.write_all(str.as_bytes()).unwrap();
}

pub fn long_number_file() -> String {
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
#![allow(unused_imports)]

extern crate core;

use std::io::{BufWriter, stderr, stdout, Write};
use crate::args::AwkArgs;
use crate::compiler::{compile, validate_program};
use crate::parser::Expr;
use crate::printable_error::PrintableError;

use crate::typing::AnalysisResults;

pub use crate::lexer::lex;
pub use crate::parser::parse;
pub use crate::symbolizer::Symbolizer;
pub use crate::typing::analyze;
use crate::vm::{Code, VirtualMachine};

mod lexer;
mod parser;
mod printable_error;
mod typing;
mod symbolizer;
mod args;
mod global_scalars;
mod vm;
mod compiler;
mod util;
mod stackt;
mod stack_counter;
mod awk_str;
#[cfg(test)]
mod test;
mod specials;
mod runtime;

pub type IO = Box<dyn Write>;

pub fn runner(args: Vec<String>, out: IO, mut err: IO) -> Result<(IO, IO), PrintableError> {
    let args = AwkArgs::new(args)?;

    let mut symbolizer = Symbolizer::new();
    let ast = analyze(parse(lex(&args.program, &mut symbolizer)?, &mut symbolizer, )?, &mut symbolizer)?;
    if args.debug {
        println!("{}", ast);
    }
    let prog = compile(ast)?;
    if args.debug {
        let prog_pretty = prog.pretty_print();
        let prog_pretty = unsafe { String::from_utf8_unchecked(prog_pretty) };
        println!("{}", prog_pretty);
        validate_program(&prog);
    }
    let vm = VirtualMachine::new(prog, args.files, out, err);
    let (mut out, mut err) = vm.run();
    if let Err(err) = out.flush() {
        return Err(PrintableError::new(format!("Failed to write to stdout. Error: {}", err)))
    }
    if let Err(err) = err.flush() {
        return Err(PrintableError::new(format!("Failed to write to stderr. Error: {}", err)))
    }
    Ok((out, err))
}

fn main() {
    let mut out = Box::new(BufWriter::new(stdout().lock()));
    let err = Box::new(stderr().lock());
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = runner(args, out, err) {
        eprintln!("{}", err);
    }
}

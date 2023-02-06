use std::io::{BufWriter, stderr, stdout, Write};
use crate::args::AwkArgs;
use crate::compiler::compile;
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
mod awk_str;
mod symbolizer;
mod args;
mod global_scalars;
mod vm;
mod compiler;
mod arrays;
mod columns;
mod util;
#[cfg(test)]
mod tests;
mod stackt;
mod stack_counter;

pub fn runner(args: Vec<String>) -> Result<(), PrintableError> {
    let args = AwkArgs::new(args)?;

    let mut symbolizer = Symbolizer::new();
    let ast = analyze(parse(
        lex(&args.program, &mut symbolizer)?,
        &mut symbolizer,
    )?)?;
    if args.debug {
        println!("{}", ast);
    }
    let prog = compile(ast)?;
    let mut out = Box::new(BufWriter::new(stdout().lock()));
    let mut err = Box::new(stderr().lock());
    let mut vm = VirtualMachine::new(prog, args.files,  out, err);
    let (mut out, err) = vm.run();
    out.flush().unwrap();
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = runner(args) {
        eprintln!("{}", err);
    }
}

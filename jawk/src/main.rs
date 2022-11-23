#![deny(unused_must_use)]

use libc::c_int;
use crate::args::AwkArgs;
use crate::parser::{Expr};
use crate::printable_error::PrintableError;

use crate::typing::{AnalysisResults};

pub use crate::codegen::{compile_and_capture, compile_and_run};
pub use crate::typing::{analyze};
pub use crate::lexer::{lex};
pub use crate::parser::{parse};
pub use crate::symbolizer::Symbolizer;

mod args;
mod codegen;
mod columns;
mod lexer;
mod integration_tests;
mod parser;
mod printable_error;
mod runtime;
mod typing;
mod symbolizer;
mod global_scalars;


pub fn runner(args: Vec<String>) -> Result<(), PrintableError> {
    let args = AwkArgs::new(args)?;
    let source = args.program.load()?;

    let mut symbolizer = Symbolizer::new();
    for x in 0..1000 {
        let ast = analyze(parse(lex(&source, &mut symbolizer).unwrap(), &mut symbolizer))?;
    }
    let ast = analyze(parse(lex(&source, &mut symbolizer).unwrap(), &mut symbolizer))?;
    if args.debug {
        println!("{}", ast);
    }

    if args.debug {
        codegen::compile_and_capture(ast, &args.files, &mut symbolizer, true)?;
    } else {
        codegen::compile_and_run(ast, &args.files, &mut symbolizer)?;
    }
    Ok(())
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = runner(args) {
        eprintln!("{}", err);
    }
    // Fuck cleanup just sys call out so it's faster
    unsafe { libc::exit(0 as c_int) }
}

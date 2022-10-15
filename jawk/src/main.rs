#![deny(unused_must_use)]

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


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let args = match AwkArgs::new(args) {
        Ok(args) => args,
        Err(_) => return,
    };
    let source = match args.program.load() {
        Ok(program) => program,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };
    // 1. Lex into token
    // 2. Parse into tree
    // 3. Type checking pass
    // 4. Run it


    let mut symbolizer = Symbolizer::new();
    // 1,2,3
    let ast = analyze(parse(lex(&source, &mut symbolizer).unwrap(), &mut symbolizer));

    // 4
    let program = match ast {
        Ok(results) => results,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    if args.debug {
        println!("{:?}", program);
        println!("{}", program);
    }

    // 5
    if args.debug {
        if let Err(err) = codegen::compile_and_capture(program, &args.files, &mut symbolizer, true) {
            eprintln!("{}", err);
        }
    } else {
        if let Err(err) = codegen::compile_and_run(program, &args.files, &mut symbolizer) {
            eprintln!("{}", err);
        }
    }
}

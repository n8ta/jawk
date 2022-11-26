#[cfg(test)]
mod inference_pass_tests;
#[cfg(test)]
mod function_pass_tests;

#[cfg(test)]
mod tests {
    use crate::{analyze, Symbolizer};
    use crate::printable_error::PrintableError;
    use crate::typing::TypedProgram;

    pub fn gen_ast(program: &str) -> Result<TypedProgram, PrintableError> {
        use crate::{lex, parse};
        let mut symbolizer = Symbolizer::new();
        analyze(parse(lex(program, &mut symbolizer).unwrap(), &mut symbolizer).unwrap())
    }

    pub fn test_exception(program: &str, error_includes_msg: &str) {
        let ast_result = gen_ast(program);
        if let Err(err) = ast_result {
            println!("Error msg: `{}\nShould include: `{}`", err.msg, error_includes_msg);
            assert!(err.msg.contains(error_includes_msg));
        } else {
            assert!(false, "type check should have failed with {}", error_includes_msg)
        }
    }
}
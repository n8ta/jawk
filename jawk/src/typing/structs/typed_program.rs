use std::fmt::{Display, Formatter};
use crate::symbolizer::Symbol;
use crate::typing::{AnalysisResults, FunctionMap};

pub struct TypedProgram {
    pub functions: FunctionMap,
    pub global_analysis: AnalysisResults,
}

impl Display for TypedProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Tests will print the program and compare it with another string
        // keep function order consistent by sorting.
        let mut sorted: Vec<Symbol> = self.functions.user_functions().iter().map(|(sym, _)| sym.clone()).collect();
        sorted.sort();
        for func_name in &sorted {
            let func = self.functions.get_user_function(func_name).unwrap();
            write!(f, "{}\n", func)?;
        }
        Ok(())
    }
}

impl TypedProgram {
    pub fn new(functions: FunctionMap, results: AnalysisResults) -> Self {
        Self { functions, global_analysis: results }
    }
}
use std::fmt::{Debug, Display, Formatter};
use hashbrown::{HashMap, HashSet};
use immutable_chunkmap::map::Map;
use crate::global_scalars::SymbolMapping;
use crate::parser::{ScalarType};
use crate::symbolizer::Symbol;
use crate::typing::structs::FunctionMap;
use crate::typing::{ITypedFunction, TypedUserFunction};

pub type MapT = Map<Symbol, ScalarType, 1000>;

#[derive(Debug, PartialEq)]
pub struct AnalysisResults {
    pub global_scalars: SymbolMapping,
    pub global_arrays: SymbolMapping,
    pub str_consts: HashSet<Symbol>,
}

impl AnalysisResults {
    pub fn new() -> Self {
        Self {
            global_scalars: SymbolMapping::new(),
            global_arrays: SymbolMapping::new(),
            str_consts: Default::default(),
        }
    }
}

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
            let func = self.functions.get(func_name).unwrap();
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
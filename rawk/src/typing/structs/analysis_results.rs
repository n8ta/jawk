use std::collections::HashSet;
use crate::global_scalars::SymbolMapping;
use crate::parser::ScalarType;
use crate::symbolizer::Symbol;
use immutable_chunkmap::map::Map;
use std::fmt::Debug;
use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::typing::ids::{GlobalArrayId, GlobalScalarId};

pub type MapT = Map<Symbol, ScalarType, 1000>;

#[derive(Debug, PartialEq)]
pub struct AnalysisResults {
    pub global_scalars: SymbolMapping<GlobalScalarId>,
    pub global_arrays: SymbolMapping<GlobalArrayId>,
    pub str_consts: HashSet<Rc<AwkStr>>,
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

use std::collections::HashSet;
use crate::global_scalars::SymbolMapping;
use crate::parser::ScalarType;
use crate::symbolizer::Symbol;
use immutable_chunkmap::map::Map;
use std::fmt::Debug;
use crate::awk_str::{RcAwkStr};
use crate::specials::{ARR_SPECIAL_NAMES};
use crate::Symbolizer;
use crate::typing::{GlobalArrayId, GlobalScalarId};

pub type MapT = Map<Symbol, ScalarType, 1000>;

#[derive(Debug, PartialEq)]
pub struct AnalysisResults {
    pub global_scalars: SymbolMapping<GlobalScalarId>,
    pub global_arrays: SymbolMapping<GlobalArrayId>,
    pub str_consts: HashSet<RcAwkStr>,
}

impl AnalysisResults {
    pub fn empty() -> Self {
        Self {
            global_scalars: SymbolMapping::new(),
            global_arrays: SymbolMapping::new(),
            str_consts: HashSet::new(),
        }
    }

    pub fn new(global_scalars: SymbolMapping<GlobalScalarId>,
               global_arrays: SymbolMapping<GlobalArrayId>,
               str_consts: HashSet<RcAwkStr>,) -> Self {
        Self {
            global_scalars,
            global_arrays,
            str_consts,
        }
    }
}


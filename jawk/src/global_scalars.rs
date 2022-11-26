use crate::symbolizer::Symbol;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolMapping {
    mapping: HashMap<Symbol, i32>,
}

impl SymbolMapping {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }
    pub fn insert(&mut self, symbol: &Symbol) {
        if self.mapping.contains_key(&symbol) {
            return;
        } else {
            self.mapping
                .insert(symbol.clone(), self.mapping.len() as i32)
        };
    }
    pub fn get(&self, symbol: &Symbol) -> Option<&i32> {
        self.mapping.get(symbol)
    }

    pub fn mapping(&self) -> &HashMap<Symbol, i32> {
        &self.mapping
    }

    pub fn contains_key(&self, symbol: &Symbol) -> bool {
        self.mapping.contains_key(symbol)
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    pub fn all_symbols(&self) -> Vec<Symbol> {
        self.mapping.keys().cloned().collect()
    }
}

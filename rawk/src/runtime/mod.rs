pub mod arrays;
#[allow(non_snake_case)]
pub mod columns;
pub mod converter;
pub mod regex_cache;
pub mod rc_manager;
pub mod special_manager;

pub use arrays::{*};
pub use columns::{*};
pub use rc_manager::{*};
pub use regex_cache::{*};
pub use converter::{*};

use crate::runtime::columns::Columns;
use crate::runtime::converter::Converter;
use crate::runtime::regex_cache::RegexCache;

pub struct VmRuntime {
    pub arrays: Arrays,
    pub columns: Columns,
    pub converter: Converter,
    pub regex_cache: RegexCache,
    pub srand_seed: f64,
}

impl VmRuntime {
    pub fn new(files: Vec<String>, array_count: usize) -> Self {
        Self {
            arrays: Arrays::new(array_count),
            columns: Columns::new(files),
            converter: Converter::new(),
            regex_cache: RegexCache::new(),
            srand_seed: 09171998.0,
        }
    }
}
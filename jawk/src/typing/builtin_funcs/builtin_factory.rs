use std::cell::RefCell;
use std::rc::Rc;
use hashbrown::HashSet;
use crate::Symbolizer;
use crate::symbolizer::Symbol;
use crate::typing::structs::Call;
use crate::typing::{BuiltinFunc, TypedUserFunction};
use crate::typing::builtin_funcs::builtin_func::NUM_BUILTIN_VARIANTS;
use crate::typing::builtin_funcs::typed_builtin::TypedBuiltin;

#[derive(Debug)]
pub struct BuiltinShared {
    pub callers: RefCell<HashSet<Rc<TypedUserFunction>>>,
    pub calls: RefCell<Vec<Call>>,
}

impl BuiltinShared {
    pub fn new() -> Self { Self { callers: RefCell::new(HashSet::new()), calls: RefCell::new(vec![]) } }
}

pub struct BuiltinFactory {
    // The ITypedFunction interface includes the calls() and callers() fields that are always empty
    // for builtins. BuiltinShared is reused between all builtins to save allocations
    shared: Rc<BuiltinShared>,
    cache: [Option<Rc<TypedBuiltin>>; NUM_BUILTIN_VARIANTS],
    names: [Symbol; NUM_BUILTIN_VARIANTS],
    symbolizer: Symbolizer,
}

impl BuiltinFactory {
    pub fn new(mut symbolizer: Symbolizer) -> Self {
        let names= BuiltinFunc::names_as_symbols(&mut symbolizer);
        let cache= [None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None];
        Self {
            shared: Rc::new(BuiltinShared::new()),
            cache,
            names,
            symbolizer,
        }
    }
    pub fn get(&mut self, builtin: BuiltinFunc) -> Rc<TypedBuiltin> {
        unsafe {
            // Safe as long as NUM_BUILTIN_VARIANTS is correct and builtin is actually an enum variant not some other number
            debug_assert!((builtin as i64) < 21 && (builtin as i64) >= 0);
            if let Some(builtin) = self.cache.get_unchecked(builtin as usize) {
                builtin.clone()
            } else {
                let name = self.names.get_unchecked(builtin as usize);
                let args = builtin.args(&mut self.symbolizer);
                let typed_builtin = Rc::new(TypedBuiltin::new(name.clone(), args, builtin, self.shared.clone()));
                self.cache[builtin as usize] = Some(typed_builtin.clone());
                typed_builtin
            }
        }
    }
}

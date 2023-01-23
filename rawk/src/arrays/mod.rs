mod split;

use hashbrown::HashMap;
use hashbrown::hash_map::Drain;
use crate::awk_str::{RcAwkStr};
use crate::vm::RuntimeScalar;

pub use split::{split_on_string, split_on_regex};

#[derive(Hash, Clone, Eq, PartialEq)]
pub struct MapKey {
    key: RcAwkStr,
}

impl MapKey {
    pub fn new(key: RcAwkStr) -> Self {
        Self { key }
    }
}

struct AwkMap {
    map: HashMap<MapKey, RuntimeScalar>,
}

impl AwkMap {
    fn access(&self, key: &MapKey) -> Option<&RuntimeScalar> {
        self.map.get(key)
    }
    fn assign(&mut self, key: &MapKey, val: RuntimeScalar) -> Option<RuntimeScalar> {
        self.map.insert(key.clone(), val)
    }
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    fn in_array(&mut self, key: &MapKey) -> bool {
        self.map.contains_key(key)
    }

    fn drain(&mut self) -> Drain<'_, MapKey, RuntimeScalar> {
        self.map.drain()
    }
}

pub struct Arrays {
    arrays: Vec<AwkMap>,
}

impl Arrays {
    pub fn new() -> Self {
        Self { arrays: Vec::new() }
    }
    pub fn allocate(&mut self, count: u16) {
        self.arrays = Vec::with_capacity(count as usize);
        for _ in 0..count {
            self.arrays.push(AwkMap::new())
        }
    }
    pub fn clear(&mut self, array_id: u16) -> Drain<'_, MapKey, RuntimeScalar> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.drain()
    }

    #[inline(never)]
    pub fn access(&mut self, array_id: u16, key: RcAwkStr) -> Option<&RuntimeScalar> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.access(&MapKey::new(key))
    }

    pub fn assign(
        &mut self,
        array_id: u16,
        indices: RcAwkStr,
        value: RuntimeScalar,
    ) -> Option<RuntimeScalar> {
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.assign(&MapKey::new(indices), value)
    }

    pub fn in_array(&mut self, array_id: u16, indices: RcAwkStr) -> bool {
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.in_array(&MapKey::new(indices))
    }
}

use hashbrown::HashMap;
use std::rc::Rc;
use hashbrown::hash_map::Drain;
use crate::awk_str::AwkStr;
use crate::runtime::value::RuntimeValue;

#[derive(Hash, Clone, Eq, PartialEq)]
pub struct MapKey {
    key: Rc<AwkStr>,
}

impl MapKey {
    // Does not drop the Rc<String> count
    pub fn new(key: Rc<AwkStr>) -> Self {
        Self { key }
    }
}

struct AwkMap {
    map: HashMap<MapKey, RuntimeValue>,
}

impl AwkMap {
    fn access(&self, key: &MapKey) -> Option<&RuntimeValue> {
        self.map.get(key)
    }
    fn assign(&mut self, key: &MapKey, val: RuntimeValue) -> Option<RuntimeValue> {
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

    fn drain(&mut self) -> Drain<'_, MapKey, RuntimeValue> {
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
    pub fn allocate(&mut self, count: usize) {
        self.arrays = Vec::with_capacity(count);
        for _ in 0..count {
            self.arrays.push(AwkMap::new())
        }
    }
    pub fn clear(&mut self, array_id: i32) -> Drain<'_, MapKey, RuntimeValue> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.drain()
    }

    #[inline(never)]
    pub fn access(&mut self, array_id: i32, key: Rc<AwkStr>) -> Option<&RuntimeValue> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.access(&MapKey::new(key))
    }

    pub fn assign(
        &mut self,
        array_id: i32,
        indices: Rc<AwkStr>,
        value: RuntimeValue,
    ) -> Option<RuntimeValue> {
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.assign(&MapKey::new(indices), value)
    }

    pub fn in_array(&mut self, array_id: i32, indices: Rc<AwkStr>) -> bool {
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.in_array(&MapKey::new(indices))
    }
}

use std::rc::Rc;
use hashbrown::HashMap;
use crate::codegen::FLOAT_TAG;

#[derive(Hash, PartialEq, Eq, Clone)]
struct HashFloat {
    bytes: [u8; 8],
}

impl HashFloat {
    pub fn new(num: f64) -> Self {
        Self { bytes: num.to_le_bytes() }
    }
    pub fn to_float64(&self) -> f64 {
        f64::from_le_bytes(self.bytes)
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum MapKey {
    String(Rc<String>),
    Float(HashFloat),
}


pub type MapValue = (i8, f64, *mut String);

impl MapKey {
    pub fn new(val: MapValue) -> Self {
        let tag = val.0;
        let num = val.1;
        let str = val.2;
        if tag == FLOAT_TAG {
            MapKey::Float(HashFloat::new(num))
        } else {
            let str = unsafe { Rc::from_raw(str) };
            match str.parse::<f64>() {
                Ok(float) => MapKey::Float(HashFloat::new(float)),
                Err(err) => MapKey::String(str)
            }
        }
    }
}

struct AwkMap {
    map: HashMap<MapKey, MapValue>,
}

impl AwkMap {
    fn access(&self, key: &MapKey) -> Option<&MapValue> {
        self.map.get(key)
    }
    fn assign(&mut self, key: &MapKey, val: MapValue) -> Option<MapValue> {
        self.map.insert(key.clone(), val)
    }
    fn new() -> Self {
        Self { map: HashMap::new() }
    }
}

pub struct Arrays {
    arrays: Vec<AwkMap>,
}

impl Arrays {
    pub fn new() -> Self {
        Self {
            arrays: Vec::new(),
        }
    }
    pub fn allocate(&mut self, count: usize) {
        self.arrays = Vec::with_capacity(count);
        for _ in 0..count {
            self.arrays.push(AwkMap::new())
        }
    }

    pub fn access(&mut self, array_id: i32, key: MapValue) -> Option<&MapValue> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.access(&MapKey::new(key))
    }

    pub fn assign(&mut self,
                  array_id: i32,
                  indices: MapValue,
                  value: MapValue,
    ) -> Option<MapValue> {
        let array = self.arrays.get_mut(array_id as usize).expect("array to exist based on id");
        array.assign(&MapKey::new(indices), value)
    }
}
use std::fmt::{Debug, Formatter};
use crate::codegen::FLOAT_TAG;
use hashbrown::HashMap;
use std::rc::Rc;
use mawk_regex::Regex;
use crate::awk_str::AwkStr;
use crate::runtime::float_parser::string_exactly_float;

#[derive(Hash, PartialEq, Eq, Clone)]
struct HashFloat {
    bytes: [u8; 8],
}

impl HashFloat {
    pub fn new(num: f64) -> Self {
        Self {
            bytes: num.to_le_bytes(),
        }
    }
    #[allow(dead_code)]
    pub fn to_float64(&self) -> f64 {
        f64::from_le_bytes(self.bytes)
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum MapKey {
    String(Rc<AwkStr>),
    Float(HashFloat),
}

impl Debug for MapKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MapKey::String(str) => {
                let s = String::from_utf8(str.bytes().to_vec()).unwrap();
                f.write_str(&s)
            }
            MapKey::Float(flt) => {
                write!(f, "{}", flt.to_float64())
            }
        }
    }
}

pub struct MapValue {
    pub tag: i8,
    pub float: f64,
    pub ptr: *const AwkStr,
}

impl MapValue {
    pub fn new(tag: i8, float: f64, ptr: *const AwkStr) -> Self { Self { tag, float, ptr } }
}

impl Debug for MapValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.tag == FLOAT_TAG {
            write!(f, "float:{}", self.float)
        } else {
            let rced = unsafe { Rc::from_raw(self.ptr) };
            let s = String::from_utf8(rced.bytes().to_vec()).unwrap();
            Rc::into_raw(rced);
            f.write_str("str:'").unwrap();
            f.write_str(&s).unwrap();
            f.write_str("'")
        }
    }
}

impl MapKey {
    // Does not drop the Rc<String> count
    pub fn new(val: MapValue) -> Self {
        if val.tag == FLOAT_TAG {
            MapKey::Float(HashFloat::new(val.float))
        } else {
            let str = unsafe { Rc::from_raw(val.ptr) };
            let res = if let Some(flt) = string_exactly_float(&str) {
                MapKey::Float(HashFloat::new(flt))
            } else {
                MapKey::String(str.clone())
            };
            Rc::into_raw(str);
            res
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
        Self {
            map: HashMap::new(),
        }
    }
    fn in_array(&mut self, key: &MapKey) -> bool {
        self.map.contains_key(key)
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

    pub fn split(&mut self, array_id: i32, string: Rc<String>, ere: Option<Regex>) {
        // if let Some(regex) = ere {
        //     regex.matches()
        // } else {
        //
        // }
    }

    pub fn access(&mut self, array_id: i32, key: MapValue) -> Option<&MapValue> {
        println!("\taccessing: {:?}", key);
        let array = self
            .arrays
            .get_mut(array_id as usize)
            .expect("array to exist based on id");
        array.access(&MapKey::new(key))
    }

    pub fn assign(
        &mut self,
        array_id: i32,
        indices: MapValue,
        value: MapValue,
    ) -> Option<MapValue> {
        println!("\tassigning: {:?}", indices);
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.assign(&MapKey::new(indices), value)
    }

    pub fn in_array(&mut self, array_id: i32, indices: MapValue) -> bool {
        let array = unsafe { self.arrays.get_unchecked_mut(array_id as usize) };
        array.in_array(&MapKey::new(indices))
    }
}

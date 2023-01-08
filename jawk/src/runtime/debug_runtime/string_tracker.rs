use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::codegen::Tag;
use crate::runtime::value::RuntimeValue;

pub struct StringTracker {
    pub string_out: usize,
    pub strings_in: usize,
}

impl StringTracker {
    pub fn new() -> Self { Self { strings_in: 0, string_out: 0 } }
    pub fn string_out(&mut self, src: &str, str: &[u8]) {
        let str = unsafe { String::from_utf8_unchecked(str.to_vec()) };
        println!("\t===> {} '{}'", src, str);
        // stdout.write_all("\t===> ".as_bytes()).unwrap();
        // stdout.write_all(&src.as_bytes()).unwrap();
        // stdout.write_all(" ".as_bytes()).unwrap();
        // stdout.write_all(&string).unwrap();
        // stdout.write_all(&[10]).unwrap();
        self.string_out += 1;
    }
    pub fn string_in(&mut self, src: &str, str: &[u8]) {
        let str = unsafe { String::from_utf8_unchecked(str.to_vec()) };
        println!("\t<=== {} '{}'", src, str);
        // let mut stdout = stdout();
        // stdout.write_all("\t<=== ".as_bytes()).unwrap();
        // stdout.write_all(&src.as_bytes()).unwrap();
        // stdout.write_all(" ".as_bytes()).unwrap();
        // stdout.write_all(&string).unwrap();
        // stdout.write_all(&[10]).unwrap();
        self.strings_in += 1;
    }

    // Track a value known to be a string coming from JIT side of FFI
    pub fn string_from_ffi(&mut self, ptr: *const AwkStr, src: &str) -> Rc<AwkStr> {
        let rc = unsafe { Rc::from_raw(ptr) };
        self.string_in(src, &*rc);
        rc
    }

    // Track a value coming from JIT side of FFI
    pub fn value_from_ffi(&mut self, tag: Tag, flt: f64, ptr: *const AwkStr, src: &str) -> RuntimeValue {
        match tag {
            Tag::FloatTag => {}
            Tag::StringTag | Tag::StrnumTag => {
                let ptr = unsafe { Rc::from_raw(ptr) };
                self.string_in(src, &*ptr);
                Rc::into_raw(ptr);
            }
        }
        RuntimeValue::new(tag, flt, ptr)
    }
    // Track a value going TO the awk side of FFI
    pub fn clone_to_ffi(&mut self, value: &RuntimeValue, src: &str) -> (Tag, f64, *const AwkStr) {
        let cloned  = value.clone();
        match cloned.0 {
            Tag::FloatTag => {}
            Tag::StringTag | Tag::StrnumTag => {
                let rc = unsafe { Rc::from_raw(cloned.2) };
                self.string_out(src, &*rc);
                Rc::into_raw(rc);
            }
        };
        cloned
    }
}
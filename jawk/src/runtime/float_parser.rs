use std::slice;
use lexical_core::write_float_options::Options;
use crate::awk_str::AwkStr;

pub struct FloatParser {
    buffer: [u8; 256],
    options: Options,
}

const FORMAT: u128 = lexical_core::format::STANDARD;

impl FloatParser {
    pub fn new() -> Self {
        let mut options = lexical_core::WriteFloatOptions::new();
        unsafe {
            options.set_trim_floats(true);
        }
        Self {
            buffer: [0; 256],
            options,
        }
    }
    pub fn parse(&mut self, flt: f64) -> Vec<u8> {
        #[cfg(debug_assertions)]
        lexical_core::write_with_options::<_, FORMAT>(flt, &mut self.buffer, &self.options);

        let res = unsafe {
            lexical_core::write_with_options_unchecked::<_, FORMAT>(
                flt,
                &mut self.buffer,
                &self.options,
            )
        };

        #[cfg(debug_assertions)]
        String::from_utf8_lossy(res).to_string();

        res.to_vec()
    }
}


// Permissive awk string to float
// 1.1a => 1.1
// 1.1 => 1.1
pub fn string_to_float(string: &AwkStr) -> f64 {
    if string.len() == 0 {
        0.0
    } else {
        let mut digits = 0;
        let mut dot_seen = false;
        let bytes = string.bytes();
        for chr in bytes.iter() {
            // [0..9]
            if (48..58).contains(chr) {
                digits += 1;
                continue;
                // 46 == '.'
            } else if *chr == 46 && !dot_seen {
                digits += 1;
                dot_seen = true;
            } else {
                break;
            }
        }
        let number_bytes: &[u8] = &bytes[0..digits];
        let number = std::str::from_utf8(number_bytes).expect("Compiler bug parsing float");
        match number.parse() {
            Ok(flt) => flt,
            Err(_err) => 0.0, // TODO: Is this right?
        }
    }
}

// Exact match string to float
// 1.1a => None
// 1.1 => 1.1
pub fn string_exactly_float(string: &AwkStr) -> Option<f64> {
    if string.len() == 0 {
        None
    } else {
        let mut digits = 0;
        let mut dot_seen = false;
        let bytes = string.bytes();
        for chr in bytes.iter() {
            // [0..9]
            if (48..58).contains(chr) {
                digits += 1;
                continue;
                // 46 == '.'
            } else if *chr == 46 && !dot_seen {
                digits += 1;
                dot_seen = true;
            } else {
                return None;
            }
        }
        let number = std::str::from_utf8(bytes).expect("Compiler bug parsing float");
        match number.parse() {
            Ok(flt) => Some(flt),
            Err(_err) => None,
        }
    }
}
use std::ffi::{CStr, CString};
use lexical_core::write_float_options::Options;
use libc::{c_char};
use mawk_regex::Regex;
use crate::util::{memchr_libc_ptr};


pub struct FloatWriter {
    buffer: [u8; 256],
    options: Options,
}

const FORMAT: u128 = lexical_core::format::STANDARD;

impl FloatWriter {
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
    pub fn num_to_string(&mut self, flt: f64) -> &[u8] {
        #[cfg(debug_assertions)]
        lexical_core::write_with_options::<_, FORMAT>(flt, &mut self.buffer, &self.options);

        let res = unsafe {
            // fuck it we'll do it live
            lexical_core::write_with_options_unchecked::<_, FORMAT>(
                flt,
                &mut self.buffer,
                &self.options,
            )
        };

        #[cfg(debug_assertions)]
        String::from_utf8_lossy(res).to_string();

        res
    }
}

#[allow(non_snake_case)]
pub struct Converter {
    // The printf format for converting numbers to strings
    // (except for output statements, where OFMT is used); "%.6g" by default.
    CONVFMT: CString,

    // The printf format for converting numbers to strings in output statements
    // (see Output Statements); "%.6g" by default. The result of the conversion is unspecified
    // if the value of OFMT is not a floating-point format specification.
    OFMT: CString,

    // Used for output
    buffer: Vec<u8>,

    // Used only for writing floats like 1.0 and -3333.0 which have exact int representations very quickly
    float_writer: FloatWriter,

    // Used for str -> num conversions
    float_regex: Regex,
}

impl Converter {
    pub fn new() -> Self {
        let default = if let Ok(x) = CString::new("%.6g") { x } else { unreachable!() };
        Self {
            CONVFMT: default.clone(),
            OFMT: default,
            buffer: Vec::with_capacity(128),
            float_regex: Regex::new(FLOAT_REGEX.as_bytes()),
            float_writer: FloatWriter::new(),
        }
    }

    pub fn num_to_str_internal(&mut self, num: f64) -> &[u8] {
        Converter::num_to_str(&mut self.buffer, &mut self.float_writer, &self.CONVFMT, num)
    }

    pub fn num_to_str_output(&mut self, num: f64) -> &[u8] {
        Converter::num_to_str(&mut self.buffer, &mut self.float_writer, &self.OFMT, num)
    }

    fn num_to_str<'a>(buffer: &'a mut Vec<u8>, fw: &'a mut FloatWriter, fmt_str: &CStr, num: f64) -> &'a [u8] {
        buffer.clear();
        let rounded = num.round();
        if rounded == num {
            return fw.num_to_string(rounded)
        }
        if let Err(bytes_needed) = unsafe { snprintf(fmt_str, buffer, num) } {
            buffer.reserve(bytes_needed);
            if let Err(_idx) = unsafe { snprintf(fmt_str, buffer, num) } {
                panic!("Compiler bug snprintf not behaving as expected")
            }
        }
        buffer.as_slice()
    }

    pub fn str_to_num(&mut self, data: &[u8]) -> Option<f64> {
        str_to_num(data, &self.float_regex)
    }

    pub fn set_convfmt(&mut self,  _bytes: &[u8]) {
        todo!()
    }
    pub fn set_ofmt(&mut self,  _bytes: &[u8]) {
        todo!()
    }
}

const FLOAT_REGEX: &'static str =
    "^(([0-9]+\\.([0-9]*)?)|(([0-9]*)?\\.?[0-9]+))(e[0-9]+)?";
const SPACE: u8 = 32;
const PLUS: u8 = 43;
const MINUS: u8 = 45;

fn str_to_num(bytes: &[u8], float_regex: &Regex) -> Option<f64> {
    if bytes.len() == 0 {
        None
    } else {
        let mut idx = 0;
        let mut sign = 1.0; // floats are positive unless they start with a -

        // Skip front <blanks>
        while let Some(byte) = bytes.get(idx) {
            if *byte == SPACE {
                idx += 1;
                continue;
            } else {
                break;
            }
        }

        // Grab 1 plus or minus sign
        if let Some(byte) = bytes.get(idx) {
            if *byte == PLUS {
                idx += 1;
            } else if *byte == MINUS {
                sign = -1.0;
                idx += 1;
            }
        }

        let bytes_to_parse = &bytes[idx..];
        let reg_match = float_regex.match_idx(bytes_to_parse);
        if let Some(reg_match) = reg_match {
            let bytes_of_float = &bytes_to_parse[reg_match.start..reg_match.start + reg_match.len];
            let number = std::str::from_utf8(bytes_of_float).expect("Compiler bug parsing float");
            match number.parse::<f64>() {
                Ok(flt) => {
                    Some(flt * sign)
                }
                Err(_err) => {
                    panic!("Regex should only match floats that can be parsed");
                }
            }
        } else {
            None
        }
    }
}

// Attempt to snprintf into buffer. Err returns number of additional bytes needed if
unsafe fn snprintf(fstring: &CStr, buffer: &mut Vec<u8>, num: f64) -> Result<(), usize> {
    debug_assert!(buffer.capacity() > 0);
    let cap = buffer.capacity();
    let count = unsafe {
        libc::snprintf(buffer.as_ptr() as *mut c_char,
                       cap,
                       fstring.as_ptr() as *const c_char,
                       num) as usize
    };
    if count >= cap {
        Err(count)
    } else {
        // find null ptr so we can fix vec len
        let idx = memchr_libc_ptr(
            buffer.as_ptr() as *const std::os::raw::c_void,
            cap,
            0).unwrap();
        buffer.set_len(idx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_conv() {
        let mut conv = Converter::new();
        assert_eq!(conv.num_to_str_output(111.11133333333), "111.111".as_bytes());
        assert_eq!(conv.num_to_str_output(1.1), "1.1".as_bytes());
    }

    #[test]
    fn test_conv_large_integer() {
        let mut conv = Converter::new();
        assert_eq!(conv.num_to_str_output(117264507.0), "117264507".as_bytes());
        assert_eq!(conv.num_to_str_output(117264507.1), "1.17265e+08".as_bytes());
    }

    use mawk_regex::{Match, Regex};
    use crate::vm::converter::{Converter, FLOAT_REGEX};

    #[test]
    fn test_float_reg() {
        let regex = Regex::new(FLOAT_REGEX.as_bytes());
        assert_eq!(regex.match_idx("1".as_bytes()), Some(Match { start: 0, len: 1 }));
        assert_eq!(regex.match_idx("1.1".as_bytes()), Some(Match { start: 0, len: 3 }));
        assert_eq!(regex.match_idx("1123Z".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx("1123".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx("123312.".as_bytes()), Some(Match { start: 0, len: 7 }));
        assert_eq!(regex.match_idx("333.3333".as_bytes()), Some(Match { start: 0, len: 8 }));
        assert_eq!(regex.match_idx(".".as_bytes()), None);
        assert_eq!(regex.match_idx(".333".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx("0.0".as_bytes()), Some(Match { start: 0, len: 3 }));
        assert_eq!(regex.match_idx("0.".as_bytes()), Some(Match { start: 0, len: 2 }));
        assert_eq!(regex.match_idx(".0".as_bytes()), Some(Match { start: 0, len: 2 }));
        assert_eq!(regex.match_idx("abc".as_bytes()), None);
        assert_eq!(regex.match_idx("abc".as_bytes()), None);
        assert_eq!(regex.match_idx("1213ff".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx("1213e12.ff".as_bytes()), Some(Match { start: 0, len: 7 }));
        assert_eq!(regex.match_idx("0.e1".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx(".0e1".as_bytes()), Some(Match { start: 0, len: 4 }));
        assert_eq!(regex.match_idx(".e1".as_bytes()), None);
        assert_eq!(regex.match_idx("1 2".as_bytes()), Some(Match { start: 0, len: 1 }));
    }


    #[test]
    fn test_string_to_float() {
        let mut conv = Converter::new();

        assert_eq!(conv.str_to_num("1".as_bytes()), Some(1.0));
        assert_eq!(conv.str_to_num("1.1".as_bytes()), Some(1.1));
        assert_eq!(conv.str_to_num("1123Z".as_bytes()), Some(1123.0));
        assert_eq!(conv.str_to_num("1123".as_bytes()), Some(1123.0));
        assert_eq!(conv.str_to_num("123312.".as_bytes()), Some(123312.0));
        assert_eq!(conv.str_to_num("333.3".as_bytes()), Some(333.3));
        assert_eq!(conv.str_to_num(".".as_bytes()), None);
        assert_eq!(conv.str_to_num(".33".as_bytes()), Some(0.33));
        assert_eq!(conv.str_to_num("0.0".as_bytes()), Some(0.0));
        assert_eq!(conv.str_to_num("0.".as_bytes()), Some(0.0));
        assert_eq!(conv.str_to_num(".0".as_bytes()), Some(0.0));
        assert_eq!(conv.str_to_num("abc".as_bytes()), None);
        assert_eq!(conv.str_to_num("abc1".as_bytes()), None);
        assert_eq!(conv.str_to_num("1213ff".as_bytes()), Some(1213.0));
        assert_eq!(conv.str_to_num("1213e2.ff".as_bytes()), Some(121300.0));
        assert_eq!(conv.str_to_num("1.e1".as_bytes()), Some(10.0));
        assert_eq!(conv.str_to_num(".1e1".as_bytes()), Some(1.0));
        assert_eq!(conv.str_to_num(".e1".as_bytes()), None);
    }

    #[test]
    fn test_rust_parse() {
        let flt: f64 = "1e2".parse().unwrap();
        assert_eq!(flt, 100.0);
    }
}


// // Exact match string to float
// // 1.1a => None
// // 1.1 => 1.1
// pub fn string_exactly_float(bytes: &[u8]) -> Option<f64> {
//     if bytes.len() == 0 {
//         None
//     } else {
//         let mut digits = 0;
//         let mut dot_seen = false;
//         for chr in bytes.iter() {
//             // [0..9]
//             if (ZERO..NINE + 1).contains(chr) {
//                 digits += 1;
//                 continue;
//             } else if *chr == DOT && !dot_seen {
//                 digits += 1;
//                 dot_seen = true;
//             } else {
//                 return None;
//             }
//         }
//         let number = std::str::from_utf8(bytes).expect("Compiler bug parsing float");
//         match number.parse() {
//             Ok(flt) => Some(flt),
//             Err(_err) => None,
//         }
//     }
// }
use lexical_core::write_float_options::Options;

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
    pub fn parse(&mut self, flt: f64) -> String {
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

        unsafe { String::from_utf8_unchecked(res.to_vec()) }
    }
}

pub fn string_to_float(string: &str) -> f64 {
    // TODO: Should never fail
    if string.len() == 0 {
        0.0
    } else {
        let mut digits = 0;
        let mut dot_seen = false;
        for chr in string.chars() {
            if chr.is_digit(10) {
                digits += 1;
                continue;
            } else if chr == '.' && !dot_seen {
                digits += 1;
                dot_seen = true;
            } else {
                break;
            }
        }
        string[0..digits]
            .parse()
            .expect(&format!("couldn't convert string to number {}", string))
    }
}

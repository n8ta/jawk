use lexical_core::write_float_options::Options;

pub struct FloatParser {
    buffer: [u8; 256],
    options: Options,
}

impl FloatParser {
    pub fn new() -> Self {
        let mut options = lexical_core::WriteFloatOptions::new();
        unsafe {
            options.set_trim_floats(true);
        }
        Self {
            buffer: [0; 256],
            options,
        } }
    pub fn parse(&mut self, flt: f64) -> String {
        const FORMAT: u128 = lexical_core::format::STANDARD;
        let res = unsafe {
            lexical_core::write_with_options_unchecked::<_, FORMAT>(flt, &mut self.buffer, &self.options)
        };
        String::from_utf8_lossy(res).to_string()
    }
}
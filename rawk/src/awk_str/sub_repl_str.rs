use crate::awk_str::{AwkStr};
use crate::compiler::compile;
use crate::util::memchr_libc;

// Struct for efficiently implementing the REPL
// arg of the sub function, its escaping rules
// and ampersand
pub struct SubReplStr {
    components: Vec<ReplComponent>,
}

#[derive(Debug)]
enum ReplComponent {
    EscapedBytes(Vec<u8>),
    Amp,
}

// Derive wasn't working, bizarre compiler error
#[cfg(test)]
impl PartialEq for ReplComponent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ReplComponent::Amp, ReplComponent::Amp) => true,
            (ReplComponent::EscapedBytes(_), ReplComponent::Amp) => false,
            (ReplComponent::Amp, ReplComponent::EscapedBytes(_)) => false,
            (ReplComponent::EscapedBytes(a), ReplComponent::EscapedBytes(b)) => a == b
        }
    }
}

impl SubReplStr {
    pub fn new(bytes: Vec<u8>) -> Self {
        // If there are no special characters skip the escaping routine as it's not super quick
        if memchr_libc(&bytes, '&' as u8) == None
            && memchr_libc(&bytes, '\\' as u8) == None {
            return Self {
                components: vec![ReplComponent::EscapedBytes(bytes)]
            };
        }

        Self { components: escape_reader(&bytes) }
    }
    pub fn push_replacement(&self, str: &mut AwkStr, matched_str: &[u8]) {
        for component in &self.components {
            match component {
                ReplComponent::EscapedBytes(bytes) => {
                    str.push_str(bytes);
                }
                ReplComponent::Amp => {
                    str.push_str(matched_str)
                }
            }
        }
    }
}

struct EscapeBuilder {
    current_str: Vec<u8>,
    components: Vec<ReplComponent>,
}

impl EscapeBuilder {
    pub fn new() -> Self {
        Self {
            current_str: vec![],
            components: vec![],
        }
    }
    pub fn add_byte(&mut self, byte: u8) {
        self.current_str.push(byte);
    }

    pub fn add_amp(&mut self) {
        if self.current_str.len() != 0 {
            let mut new_str = vec![];
            std::mem::swap(&mut new_str, &mut self.current_str);
            self.components.push(ReplComponent::EscapedBytes(new_str));
        }
        self.components.push(ReplComponent::Amp);
    }

    pub fn done(mut self) -> Vec<ReplComponent> {
        if self.current_str.len() != 0 {
            self.components.push(ReplComponent::EscapedBytes(self.current_str));
        }
        self.components
    }
}

fn escape_reader(str: &[u8]) -> Vec<ReplComponent> {
    let mut builder = EscapeBuilder::new();
    let mut iter = str.iter().peekable();
    while let Some(char) = iter.next() {
        let next = iter.peek();
        if *char == '\\' as u8 && next == Some(&&('\\' as u8)) {
            // Escaped \
            builder.add_byte('\\' as u8);
            iter.next();
        } else if *char == '\\' as u8 && next == Some(&&('&' as u8)) {
            // Escaped &
            builder.add_byte('&' as u8);
            iter.next();
        } else if *char == '\\' as u8 {
            // Just a \
            builder.add_byte('\\' as u8)
        } else if *char == '&' as u8 {
            // Logical &
            builder.add_amp();
        } else {
            // Regular character
            builder.add_byte(*char);
        }
    }
    builder.done()
}

#[cfg(test)]
mod test {
    use crate::awk_str::AwkStr;
    use crate::awk_str::sub_repl_str::{escape_reader, ReplComponent, SubReplStr};

    fn s(bytes: &str) -> ReplComponent {
        ReplComponent::EscapedBytes(bytes.as_bytes().to_vec())
    }

    #[test]
    fn test_escape_reader() {
        assert_eq!(escape_reader("abc".as_bytes()), vec![s("abc")]);
        assert_eq!(escape_reader("\nab c\t".as_bytes()), vec![s("\nab c\t")]);
        assert_eq!(escape_reader("&".as_bytes()), vec![ReplComponent::Amp]);
        assert_eq!(escape_reader("&aa".as_bytes()), vec![ReplComponent::Amp, s("aa")]);
        assert_eq!(escape_reader("&aa&&".as_bytes()), vec![ReplComponent::Amp, s("aa"), ReplComponent::Amp, ReplComponent::Amp]);
        assert_eq!(escape_reader("aa\\&&".as_bytes()), vec![s("aa&"), ReplComponent::Amp]);
        assert_eq!(escape_reader("aa\\\\&&".as_bytes()), vec![s("aa\\"), ReplComponent::Amp, ReplComponent::Amp]);
    }

    #[test]
    fn test_substrrepl_e2e_no_specials() {
        let repl_str = "abcdefghijklmnopqrstuvwxyz";
        let repl = SubReplStr::new(repl_str.as_bytes().to_vec());
        let mut str = AwkStr::new_empty();
        repl.push_replacement(&mut str, &[1,2,3]);
        assert_eq!(str.bytes(), repl_str.as_bytes())
    }

    #[test]
    fn test_substrrepl_e2e_amp() {
        let repl_str = "a&&&b&b";
        let repl = SubReplStr::new(repl_str.as_bytes().to_vec());
        let mut str = AwkStr::new_empty();
        repl.push_replacement(&mut str, "xyz".as_bytes());
        assert_eq!(str.bytes(), "axyzxyzxyzbxyzb".as_bytes())
    }

    #[test]
    fn test_substrrepl_e2e_amp_esc() {
        let repl_str = "a\\&&\\&b&b\\\\\\";
        let repl = SubReplStr::new(repl_str.as_bytes().to_vec());
        let mut str = AwkStr::new_empty();
        repl.push_replacement(&mut str, "xyz".as_bytes());
        assert_eq!(str.bytes(), "a&xyz&bxyzb\\\\".as_bytes())
    }
}
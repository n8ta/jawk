use mawk_regex::Regex;
use crate::util::index_in_slice;


// Used for the builtin split() function when splitting on an ERE
pub struct RegexSplit<'a> {
    string: &'a [u8],
    regex: &'a Regex,
    start: usize,
}

pub fn split_on_regex<'a>(regex: &'a Regex, string: &'a [u8], ) -> RegexSplit<'a> {
    RegexSplit {
        string,
        regex,
        start: 0,
    }
}

// Used for the builtin split() function when splitting on FS
pub struct StringSplit<'a> {
    string: &'a [u8],
    sep: &'a [u8],
    start: usize,

}

pub fn split_on_string<'a>(sep: &'a [u8], string: &'a [u8]) -> StringSplit<'a> {
    StringSplit {
        string,
        sep,
        start: 0
    }
}

impl<'a> Iterator for RegexSplit<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let slice = &self.string[self.start..];
        if slice.len() > 0 {
            if let Some(idx) = self.regex.match_idx(slice) {
                let res = &self.string[self.start..self.start+idx.start];
                self.start += idx.len + idx.start;
                Some(res)
            } else {
                let res = &self.string[self.start..];
                self.start = self.string.len();
                Some(res)
            }
        } else {
            None
        }

    }
}

impl<'a> Iterator for StringSplit<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let slice = &self.string[self.start..];
        if slice.len() > 0 {
            if let Some(idx) = index_in_slice(self.sep, slice) {
                let res = &self.string[self.start..self.start+idx];
                self.start += idx + self.sep.len();
                Some(res)
            } else {
                let res = &self.string[self.start..];
                self.start = self.string.len();
                Some(res)
            }
        } else {
            None
        }

    }
}

#[cfg(test)]
mod tests {
    use mawk_regex::Regex;
    use crate::runtime::arrays::{split_on_regex, split_on_string};

    #[test]
    fn test_split_on_regex() {
        let reg = Regex::new("B+".as_bytes());
        assert_eq!(split_on_regex( &reg, "aBBBcBBBBd".as_bytes()).collect::<Vec<&[u8]>>(), vec!["a".as_bytes(), "c".as_bytes(), "d".as_bytes()]);
    }

    #[test]
    fn test_split_nothing() {
        let reg = Regex::new("B+".as_bytes());
        let split = split_on_regex( &reg, "".as_bytes()).collect::<Vec<&[u8]>>();
        assert_eq!(split.len(), 0);
    }

    #[test]
    fn test_split_on_string() {
        let split = "BBB".as_bytes();
        assert_eq!(split_on_string(&split, "aBBBcBBBd".as_bytes()).collect::<Vec<&[u8]>>(), vec!["a".as_bytes(), "c".as_bytes(), "d".as_bytes()]);
    }

    #[test]
    fn test_split_nothing_str() {
        let split = "AKASDFJASLKDFJLA".as_bytes();
        let split = split_on_string(&split, "".as_bytes()).collect::<Vec<&[u8]>>();
        assert_eq!(split.len(), 0);
    }
}
use crate::columns::borrrowing_split::{borrowing_split, Split};

pub struct LazilySplitLine {
    contents: Vec<u8>,
    fs: Vec<u8>,
    splits: Vec<Split>,
}

impl LazilySplitLine {
    pub fn new() -> Self {
        Self {
            contents: vec![],
            fs: vec![32], // space
            splits: vec![],
        }
    }
    pub fn content_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.contents
    }
    pub fn calculate_columns(&mut self) {
        self.splits = borrowing_split(&self.contents, &*self.fs);
    }

    pub fn set_fs(&mut self, fs: Vec<u8>) {
        self.fs = fs;
    }
    pub fn get(&mut self, idx: usize) -> Option<&[u8]> {
        if idx == 0 {
            return Some(&self.contents)
        }
        match self.splits.get(idx-1) {
            None => None,
            Some(split) => Some(split.get_slice(&self.contents))
        }
    }
}
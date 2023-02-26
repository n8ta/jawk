#[cfg(test)]
mod test;

use quick_drop_deque::QuickDropDeque;
use crate::util::{index_in_dq, subslices};

const SPACE: u8 = 32;

pub struct LazilySplitLine {
    fs: Vec<u8>,
    next_fs: Option<Vec<u8>>,
}


impl LazilySplitLine {
    pub fn new() -> Self {
        Self {
            fs: vec![SPACE],
            next_fs: None,
        }
    }

    pub fn set_field_sep(&mut self, fs: &[u8]) {
        self.next_fs = Some(fs.to_vec());
    }

    pub fn next_record(&mut self) {
        if let Some(next_fs) = self.next_fs.take() {
            self.fs = next_fs;
        }
    }

    pub fn get(&mut self, dq: &QuickDropDeque, field_idx: usize, end_of_record_idx: usize) -> Vec<u8> {
        let mut vec = vec![];
        self.get_into(dq, field_idx, end_of_record_idx, &mut vec);
        vec
    }

    fn move_into_buf(dq: &QuickDropDeque, result: &mut Vec<u8>, start: usize, end: usize) {
        let (left, right) = subslices(dq, start, end);
        result.extend_from_slice(left);
        result.extend_from_slice(right);
        return
    }

    pub fn get_into(&mut self, dq: &QuickDropDeque, field_idx: usize, end_of_record_idx: usize, result: &mut Vec<u8>) {
        debug_assert!(field_idx != 0);
        let mut start_of_field = 0;
        let mut fields_found = 0;
        let fs_is_space = self.fs == &[SPACE];
        while let Some(found_at) = index_in_dq(&self.fs, dq, start_of_field, end_of_record_idx) {
            fields_found += 1;
            if fields_found == field_idx {
                return LazilySplitLine::move_into_buf(dq, result, start_of_field, found_at);
            }
            let mut spaces_after_record = 0;
            while fs_is_space && dq.get(found_at+spaces_after_record+1) == Some(&SPACE) {
                spaces_after_record += 1;
            }
            start_of_field = found_at + self.fs.len() + spaces_after_record;
        }
        if fields_found + 1 == field_idx {
            // Trailing record
            LazilySplitLine::move_into_buf(dq, result, start_of_field, end_of_record_idx);
        }
    }

    pub fn nf(&mut self, dq: &QuickDropDeque, end_of_record_idx: usize) -> usize {
        let mut start_of_record = 0;
        let mut records_found = 0;
        while let Some(found_at) = index_in_dq(&self.fs, dq, start_of_record, end_of_record_idx) {
            records_found += 1;
            start_of_record = found_at + self.fs.len();
        }
        // Final record may not have a FS after it
        if start_of_record != end_of_record_idx {
            records_found += 1;
        }
        records_found
    }

    pub fn get_field_sep(&self) -> &[u8] {
        if let Some(next) = &self.next_fs {
            next
        } else {
            &self.fs
        }
    }
}
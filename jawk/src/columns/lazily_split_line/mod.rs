#[cfg(test)]
mod test;

use quick_drop_deque::QuickDropDeque;
use crate::columns::borrowing_split::{borrowing_split, Split};
use crate::columns::index_of::{index_in_dq, subslices};

pub struct LazilySplitLine {
    // splits: Vec<Split>,
    fs: Vec<u8>,
    next_fs: Option<Vec<u8>>,
}

const EMPTY_SLICE: &[u8] = &[];

impl LazilySplitLine {
    pub fn new() -> Self {
        Self {
            fs: vec![32], // space
            // splits: vec![],
            next_fs: None,
        }
    }

    pub fn set_fs(&mut self, fs: Vec<u8>) {
        self.next_fs = Some(fs);
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

    pub fn get_into(&mut self, dq: &QuickDropDeque, field_idx: usize, end_of_record_idx: usize, result: &mut Vec<u8>) {
        result.clear();
        if field_idx == 0 {
            let (left, right) = subslices(dq, 0, end_of_record_idx);
            result.extend_from_slice(left);
            result.extend_from_slice(right);
            return
        }
        let mut start_of_record = 0;
        let mut records_found = 0;
        while let Some(found_at) = index_in_dq(&self.fs, dq, start_of_record, end_of_record_idx) {
            let found_at = found_at + start_of_record;
            records_found += 1;
            if records_found == field_idx {
                let (left, right) = subslices(dq, start_of_record, found_at);
                result.extend_from_slice(left);
                result.extend_from_slice(right);
                return
            }
            start_of_record = found_at + self.fs.len();
        }
        if records_found+1 == field_idx {
            // Trailing record
            let (left, right) = subslices(dq, start_of_record, end_of_record_idx);
            result.extend_from_slice(left);
            result.extend_from_slice(right);
        }
    }

    pub fn nf(&mut self, dq: &QuickDropDeque, end_of_record_idx: usize) -> usize {
        let mut start_of_record = 0;
        let mut records_found = 0;
        while let Some(found_at) = index_in_dq(&self.fs, dq, start_of_record, end_of_record_idx) {
            let found_at = found_at + start_of_record;
            records_found += 1;
            start_of_record = found_at + self.fs.len();
        }
        // Final record may not have a FS after it
        if start_of_record != end_of_record_idx {
            records_found += 1;
        }
        records_found
    }
}
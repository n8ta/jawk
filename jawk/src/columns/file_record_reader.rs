use std::cmp::min;
use std::fs::File;
use std::io::Read;
use std::ptr;
use crate::columns::index_of::index_in_dq;
use crate::printable_error::PrintableError;

use quick_drop_deque::QuickDropDeque;
use crate::columns::lazily_split_line::LazilySplitLine;

struct FileWithPath {
    path: String,
    file: File,
}

pub struct FileReader {
    file: Option<FileWithPath>,
    slop: QuickDropDeque,
    rs: Vec<u8>,
    end_of_current_record: usize,
    line: LazilySplitLine,
}

impl FileReader {
    pub fn new() -> Self {
        Self {
            slop: QuickDropDeque::with_io_size(16*1024, 8*1024),
            file: None,
            rs: vec![10], //space
            end_of_current_record: 0,
            line: LazilySplitLine::new(),
        }
    }

    pub fn next_file(&mut self, file: File, path: String) {
        self.file = Some(FileWithPath { file, path })
    }

    pub fn try_next_record(&mut self) -> Result<bool, PrintableError> {
        let file = if let Some(file) = &mut self.file {
            self.line.next_record();
            file
        } else {
            return Ok(false);
        };

        // Drop last record if any
        self.slop.drop_front(self.end_of_current_record);

        // Drop the record sep from the front if it's there. When the user changes RS read we want
        // to retain the RS from the prior record.
        let mut rs_idx = index_in_dq(&self.rs, &self.slop, 0, self.slop.len());
        if rs_idx == Some(0) {
            self.slop.drop_front(self.rs.len());
            rs_idx = None;
        }

        loop {
            // Check if our last read grabbed more than 1 record
            if let Some(idx) = rs_idx.or_else(|| index_in_dq(&self.rs, &self.slop, 0, self.slop.len())) {
                self.end_of_current_record = idx;
                return Ok(true);
            }
            // Nope, then read some bytes into buf then copy to slop
            let bytes_read = self.slop.read(&mut file.file).unwrap(); // TODO: unwrap

            if bytes_read == 0 {
                // No new data!
                self.end_of_current_record = self.slop.len();

                if self.slop.len() != 0 {
                    // Reached EOF but we have slop from last read without RS completing it
                    return Ok(true);
                } else {
                    // Reached EOF and nothing left in slop buffer we're out of records
                    return Ok(false);
                }
            }
        }
    }

    pub fn get_into_buf(&mut self, idx: usize, result: &mut Vec<u8>) {
        self.line.get_into(&self.slop, idx, self.end_of_current_record, result);
        // let slices = self.slop.as_slices();
        // let bytes_to_move = self.end_of_current_record;
        // let elements_from_left = min(slices.0.len(), bytes_to_move);
        // result.extend_from_slice(&slices.0[0..elements_from_left]);
        // if elements_from_left < bytes_to_move {
        //     let remaining = bytes_to_move - elements_from_left;
        //     result.extend_from_slice(&slices.1[0..remaining]);
        // }
    }

    pub fn get(&mut self, idx: usize) -> Option<Vec<u8>> {
        if self.end_of_current_record == 0 {
            return None
        }
        let mut result: Vec<u8> = Vec::with_capacity(self.end_of_current_record);
        self.get_into_buf(idx, &mut result);
        Some(result)
    }

    pub fn set_rs(&mut self, rs: Vec<u8>) {
        self.rs = rs;
    }
}

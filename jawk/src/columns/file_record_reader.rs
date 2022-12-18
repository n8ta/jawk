use std::cmp::min;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use lexical_core::BUFFER_SIZE;
use crate::columns::borrowing_split::{borrowing_split, Split};
use crate::columns::index_of::index_in_dq;
use crate::columns::lazily_split_line::LazilySplitLine;
use crate::printable_error::PrintableError;

use quick_drop_deque::QuickDropDeque;

struct FileWithPath {
    path: String,
    file: File,
}

const BUF_LEN: usize = 4096;

pub struct FileReader {
    file: Option<FileWithPath>,
    slop: QuickDropDeque,
    rs: Vec<u8>,
    read_buf: [u8; BUF_LEN],
    index_of_next_record: usize,
}

impl FileReader {
    fn read_into_buf(file: &mut FileWithPath, buf: &mut[u8; BUF_LEN]) -> Result<usize, PrintableError> {
        match file.file.read(buf) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(err) => Err(PrintableError::new(format!("Error reading from file {}\n{}", file.path, err)))
        }
    }


    pub fn new() -> Self {
        Self {
            slop: QuickDropDeque::with_capacity(BUF_LEN),
            file: None,
            rs: vec![10], //space
            read_buf: [0; BUF_LEN],
            index_of_next_record: 0,
        }
    }

    pub fn next_file(&mut self, file: File, path: String) {
        self.file = Some(FileWithPath { file, path })
    }

    pub fn try_next_record(&mut self) -> Result<bool, PrintableError> {
        // Drop last record if any
        self.slop.drop_front(self.index_of_next_record);

        let file = if let Some(file) = &mut self.file {
            file
        } else {
            return Ok(false);
        };

        loop {
            // Check if our last read grabbed more than 1 record
            if let Some(idx) = index_in_dq(&self.rs, &self.slop) {
                self.index_of_next_record = idx+self.rs.len();
                return Ok(true);
            }

            // Nope, then read some bytes into buf then copy to slop
            let bytes_read = FileReader::read_into_buf(file, &mut self.read_buf)?;

            if bytes_read == 0 {
                // No new data!
                self.index_of_next_record = self.slop.len();

                if self.slop.len() != 0 {
                    // Reached EOF but we have slop from last read without RS completing it
                    return Ok(true);
                } else {
                    // Reached EOF and nothing left in slop buffer we're out of records
                    return Ok(false);
                }
            }

            // Copy bytes we just read into slop, the loop continues
            self.slop.extend_from_slice(&self.read_buf[0..bytes_read]);
        }
    }



    pub fn get(&mut self, idx: usize) -> Option<Vec<u8>> {
        // TODO: Columns
        if self.index_of_next_record == 0 {
            return None
        }
        let mut result = Vec::with_capacity(self.index_of_next_record);

        let slices = self.slop.as_slices();
        let bytes_to_move = self.index_of_next_record - self.rs.len();
        let elements_from_left = min(slices.0.len(), bytes_to_move);
        result.extend_from_slice(&slices.0[0..elements_from_left]);
        if elements_from_left < bytes_to_move {
            let remaining = bytes_to_move - elements_from_left;
            result.extend_from_slice(&slices.1[0..remaining]);
        }
        Some(result)
    }

    pub fn set_rs(&mut self, rs: Vec<u8>) {
        self.rs = rs;
    }
}

use std::cmp::min;
use std::fs::File;
use std::io::Read;
use crate::columns::index_of::index_in_dq;
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
    end_of_current_record: usize,
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
            end_of_current_record: 0,
        }
    }

    pub fn next_file(&mut self, file: File, path: String) {
        self.file = Some(FileWithPath { file, path })
    }

    #[inline(never)]
    pub fn try_next_record(&mut self) -> Result<bool, PrintableError> {

        let file = if let Some(file) = &mut self.file {
            file
        } else {
            return Ok(false);
        };

        // Drop last record if any
        self.slop.drop_front(self.end_of_current_record);

        // Drop the record sep from the front if it's there. When the user changes RS read we want
        // to retain the RS from the prior record.
        let starts_with_rs = index_in_dq(&self.rs, &self.slop) == Some(0);
        if starts_with_rs {
            self.slop.drop_front(self.rs.len())
        }

        loop {
            // Check if our last read grabbed more than 1 record
            if let Some(idx) = index_in_dq(&self.rs, &self.slop) {
                self.end_of_current_record = idx;
                return Ok(true);
            }

            // Nope, then read some bytes into buf then copy to slop
            let bytes_read = FileReader::read_into_buf(file, &mut self.read_buf)?;

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

            // Copy bytes we just read into slop, the loop continues
            self.slop.extend_from_slice(&self.read_buf[0..bytes_read]);
        }
    }



    pub fn get(&mut self, idx: usize) -> Option<Vec<u8>> {
        if self.end_of_current_record == 0 {
            return None
        }
        let mut result = Vec::with_capacity(self.end_of_current_record);

        let slices = self.slop.as_slices();
        let bytes_to_move = self.end_of_current_record;
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

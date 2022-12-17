use std::cmp::min;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use crate::columns::index_of::{index_in_dq};
use crate::printable_error::PrintableError;

struct FileWithPath {
    path: String,
    file: File
}

const BUF_SIZE: usize = 4096;

pub struct FileReader {
    file: Option<FileWithPath>,
    rs: Vec<u8>,
    slop: VecDeque<u8>,
    read_buf: [u8; BUF_SIZE],
}

impl FileReader {
    pub fn new() -> Self {
        Self {
            rs: vec![10],
            slop: VecDeque::with_capacity(BUF_SIZE),
            read_buf: [0; BUF_SIZE],
            file: None,
        }
    }
    pub fn set_rs(&mut self, rs: Vec<u8>) {
        self.rs = rs;
    }

    pub fn next_file(&mut self, file: File, path: String) {
        self.file = Some(FileWithPath { file, path });
        self.slop.clear();
    }

    fn read_into_buf(file: &mut FileWithPath, buf: &mut [u8; BUF_SIZE]) -> Result<usize, PrintableError> {
        match file.file.read(buf) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(err) => Err(PrintableError::new(format!("Error reading from file {}\n{}", file.path, err)))
        }
    }

    // Efficiently move bytes between dequeue and vec using two memcpys.
    fn move_from_dq_to_vec(dq: &mut VecDeque<u8>, dest_buffer: &mut Vec<u8>, count: usize, split_len: usize) {
        let slices = dq.as_slices();
        let total = min(slices.0.len(), count);
        dest_buffer.extend_from_slice(&slices.0[0..total]);
        if total < count {
            dest_buffer.extend_from_slice(&slices.1[0..count - total]);
        }
        dq.drain(0..count+split_len);
    }

    pub fn try_read_record_into_buf(&mut self, dest_buffer: &mut Vec<u8>) -> Result<bool, PrintableError> {
        dest_buffer.clear();

        loop {
            let file = if let Some(file) = &mut self.file {
              file
            } else {
                return Ok(false)
            };

            // Check if our last read grabbed more than 1 record
            if let Some(idx) = index_in_dq(&self.rs, &self.slop) {
                FileReader::move_from_dq_to_vec(&mut self.slop, dest_buffer, idx, self.rs.len());
                return Ok(true);
            }

            // Nope, then read some bytes into buf then copy to slop
            let bytes_read = FileReader::read_into_buf(file, &mut self.read_buf)?;

            if bytes_read == 0 {
                // No new data!
                if self.slop.len() != 0 {
                    // Reached EOF but we have slop from last read without RS completing it
                    let total = self.slop.len();
                    FileReader::move_from_dq_to_vec(&mut self.slop, dest_buffer, total, 0);
                    return Ok(true);
                } else {
                    // Reached EOF and nothing left in slop buffer we're out of records
                    return Ok(false);
                }
            }
            // Copy bytes we just read into slop, the loop continues
            self.slop.extend(&self.read_buf[0..bytes_read]);
        }
    }
}
mod splitter;
mod file_reader;
#[cfg(test)]
mod tests;
mod record_state;

use std::fs::File;
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::runtime::columns::file_reader::FileReader;
use crate::printable_error::PrintableError;

pub use record_state::RecordState;
use crate::runtime::columns::record_state::RecordStateOutput;

pub struct Columns {
    files: Vec<String>,
    reader: FileReader,
}

impl Columns {
    pub fn new(mut files: Vec<String>) -> Self {
        files.reverse();
        Columns {
            files,
            reader: FileReader::new(),
        }
    }

    #[cfg(test)]
    pub fn get(&mut self, column: usize) -> Vec<u8> {
        self.reader.get(column)
    }

    pub fn set(&mut self, _column: usize, _bytes: &[u8]) -> Vec<u8> {
        todo!()
    }

    pub fn get_into_buf(&mut self, column: usize, buf: &mut Vec<u8>) {
        buf.clear();
        self.reader.get_into_buf(column, buf);
    }

    fn next_file(&mut self) -> Result<Option<RcAwkStr>, PrintableError> {
        if let Some(file_path) = self.files.pop() {
            let file = match File::open(&file_path) {
                Ok(f) => f,
                Err(err) => return Err(PrintableError::new(format!("Failed to open file {}\n{}", file_path, err))),
            };
            self.reader.next_file(file, file_path);
            // TODO: real name
            Ok(Some(RcAwkStr::new_str("")))
        } else {
            Ok(None)
        }
    }

    pub fn next_record(&mut self,
                       mut state: RecordState,)
                       -> Result<RecordStateOutput, PrintableError> {
        let mut FNR = state.FNR;
        let mut NR = state.NR;
        let mut next_file: Option<RcAwkStr> = None;
        loop {
            if self.reader.try_next_record()? {
                FNR += 1.0;
                NR += 1.0;
                return Ok(RecordStateOutput::new(NR, FNR, true, next_file));
            };
            if let Some(next_file_name) = self.next_file()? {
                FNR = 0.0;
                next_file = Some(next_file_name);
                continue;
            } else {
                // TODO: What should state be in the END block
                return Ok(RecordStateOutput::new(NR, FNR, false, next_file));
            }
        }
    }

    pub fn set_rs(&mut self, value: Vec<u8>) {
        self.reader.set_rs(value)
    }

    pub fn get_rs(&mut self) -> &[u8] {
        self.reader.get_rs()
    }

    pub fn set_fs(&mut self, value: Vec<u8>) {
        self.reader.set_fs(value);
    }

    pub fn get_fs(&mut self) -> &[u8] {
        self.reader.get_fs()
    }
}
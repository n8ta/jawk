mod splitter;
mod file_reader;
#[cfg(test)]
mod tests;

use std::fs::File;
use crate::awk_str::AwkStr;
use crate::runtime::columns::file_reader::FileReader;
use crate::printable_error::PrintableError;

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
        // TODO: UTF8 ?
        buf.clear();
        self.reader.get_into_buf(column, buf);
    }

    fn next_file(&mut self) -> Result<bool, PrintableError> {
        if let Some(file_path) = self.files.pop() {
            let file = match File::open(&file_path) {
                Ok(f) => f,
                Err(err) => return Err(PrintableError::new(format!("Failed to open file {}\n{}", file_path, err))),
            };
            self.reader.next_file(file, file_path);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn next_record(&mut self) -> Result<bool, PrintableError> {
        loop {
            if self.reader.try_next_record()? {
                return Ok(true);
            };
            if self.next_file()? {
                continue;
            } else {
                return Ok(false);
            }
        }
    }

    pub fn set_rs(&mut self, value: Vec<u8>) {
        self.reader.set_rs(value)
    }

    pub fn get_rs(&mut self) -> &[u8]{
        self.reader.get_rs()
    }

    pub fn set_fs(&mut self, value: Vec<u8>) {
        self.reader.set_fs(value);
    }

    pub fn get_fs(&mut self) -> &[u8] {
        self.reader.get_fs()
    }
}
mod lazily_split_line;
mod borrowing_split;
mod file_record_reader;
#[cfg(test)]
mod tests;

use std::fs::File;
use crate::awk_str::AwkStr;
use crate::columns::file_record_reader::FileReader;
use crate::printable_error::PrintableError;

pub struct Columns {
    files: Vec<String>,
    reader: FileReader,
    // overwritten_records: Vec<Vec<u8>>
}

impl Columns {
    pub fn new(mut files: Vec<String>) -> Self {
        files.reverse();
        Columns {
            files,
            reader: FileReader::new(),
        }
    }

    pub fn get(&mut self, column: usize) -> AwkStr {
        let bytes = self.reader.get(column);
        // TODO: check utf8
        AwkStr::new(bytes)
    }

    pub fn set(&mut self, _column: usize, _bytes: &[u8]) -> AwkStr {
        todo!()
    }

    pub fn get_into_buf(&mut self, column: usize, buf: &mut Vec<u8>) {
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

    pub fn next_line(&mut self) -> Result<bool, PrintableError> {
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

    pub fn set_record_sep(&mut self, value: String) {
        self.reader.set_record_sep(value.as_bytes().to_vec())
    }

    pub fn get_record_sep(&mut self, _value: String) -> &[u8]{
        self.reader.get_record_sep()
    }

    pub fn set_field_sep(&mut self, value: &[u8]) {
        self.reader.set_field_sep(value);
    }

    pub fn get_field_sep(&mut self) -> &[u8] {
        self.reader.get_field_sep()
    }
}
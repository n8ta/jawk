mod lazily_split_line;
mod index_of;
mod borrowing_split;
mod file_record_reader;

use std::fs::File;
use crate::awk_str::AwkStr;
use crate::columns::file_record_reader::FileReader;
use crate::printable_error::PrintableError;
pub use crate::columns::index_of::{index_in_slice};

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

    pub fn get(&mut self, column: usize) -> AwkStr {
        let bytes = self.reader.get(column);
        // TODO: check utf8
        AwkStr::new(bytes)
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
        self.reader.set_rs(value.as_bytes().to_vec())
    }

    pub fn set_field_sep(&mut self, value: String) {
        // let bytes = value.as_bytes().to_vec();
        todo!()
    }

    pub fn get_field_sep(&mut self) -> &[u8] {
        self.reader.get_field_sep()
    }
}


#[test]
fn test_files() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    let file_path_2 = temp_dir.path().join("file2.txt");
    std::fs::write(file_path_1.clone(), "a b c\nd e f\ng h i\n").unwrap();
    std::fs::write(file_path_2.clone(), "1 2 3\n4 5 6\n7 8 9\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
        file_path_2.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "a b c".into());
    assert_eq!(cols.get(1), "a".into());
    assert_eq!(cols.get(2), "b".into());
    assert_eq!(cols.get(3), "c".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "f".into());
    assert_eq!(cols.get(2), "e".into());
    assert_eq!(cols.get(1), "d".into());
    assert_eq!(cols.get(0), "d e f".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "i".into());
    assert_eq!(cols.get(2), "h".into());
    assert_eq!(cols.get(1), "g".into());
    assert_eq!(cols.get(0), "g h i".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "1 2 3".into());
    assert_eq!(cols.get(3), "3".into());
    assert_eq!(cols.get(2), "2".into());
    assert_eq!(cols.get(1), "1".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "6".into());
    assert_eq!(cols.get(2), "5".into());
    assert_eq!(cols.get(1), "4".into());
    assert_eq!(cols.get(0), "4 5 6".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "9".into());
    assert_eq!(cols.get(2), "8".into());
    assert_eq!(cols.get(1), "7".into());
    assert_eq!(cols.get(0), "7 8 9".into());
    assert_eq!(cols.next_line().unwrap(), false);
    assert_eq!(cols.next_line().unwrap(), false);
}

#[test]
fn test_files_set_rs() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "a b c\n-ZZZ1-ZZZ2").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "a b c".into());
    cols.set_record_sep("-".to_string());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "\n".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "ZZZ1".into());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "ZZZ2".into());
    assert_eq!(cols.next_line().unwrap(), false);
    assert_eq!(cols.next_line().unwrap(), false);
}


#[test]
fn test_simple_one_line() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "1 2 3\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "1 2 3".into());
    assert_eq!(cols.next_line().unwrap(), false);
}

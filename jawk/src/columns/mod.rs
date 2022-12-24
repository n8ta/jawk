mod lazily_split_line;
mod index_of;
mod borrowing_split;
mod file_record_reader;

use std::fs::File;
use crate::columns::file_record_reader::FileReader;
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

    pub fn get(&mut self, column: usize) -> String {
        let bytes = self.reader.get(column);
        // TODO: check utf8
        unsafe { String::from_utf8_unchecked(bytes) }
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
    assert_eq!(cols.get(0), "a b c");
    assert_eq!(cols.get(1), "a");
    assert_eq!(cols.get(2), "b");
    assert_eq!(cols.get(3), "c");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "f");
    assert_eq!(cols.get(2), "e");
    assert_eq!(cols.get(1), "d");
    assert_eq!(cols.get(0), "d e f");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "i");
    assert_eq!(cols.get(2), "h");
    assert_eq!(cols.get(1), "g");
    assert_eq!(cols.get(0), "g h i");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "1 2 3");
    assert_eq!(cols.get(3), "3");
    assert_eq!(cols.get(2), "2");
    assert_eq!(cols.get(1), "1");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "6");
    assert_eq!(cols.get(2), "5");
    assert_eq!(cols.get(1), "4");
    assert_eq!(cols.get(0), "4 5 6");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(3), "9");
    assert_eq!(cols.get(2), "8");
    assert_eq!(cols.get(1), "7");
    assert_eq!(cols.get(0), "7 8 9");
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
    assert_eq!(cols.get(0), "a b c");
    cols.set_record_sep("-".to_string());
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "\n");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "ZZZ1");
    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "ZZZ2");
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
    assert_eq!(cols.get(0), "1 2 3");
    assert_eq!(cols.next_line().unwrap(), false);
}

use std::iter::Rev;
use hashbrown::HashMap;
use std::path::PathBuf;
use std::vec::IntoIter;

type Line = HashMap<usize, String>;

pub struct Columns {
    rs: &'static str,
    fs: &'static str,
    files: Rev<IntoIter<String>>,
    current_path: Option<String>,
    lines: HashMap<usize, Line>,
    line_number: Option<usize>,
}

impl Columns {
    pub fn new(files: Vec<String>) -> Self {
        let c = Columns {
            rs: "\n",
            fs: " ",
            files: files.into_iter().rev(), // Collecting could be mean allocating. Instead just pull from the iterator
            line_number: None,
            lines: HashMap::new(),
            current_path: None,
        };
        c
    }

    fn get_line_number(&self) -> usize {
        if let Some(line) = self.line_number {
            line
        } else {
            0
        }
    }

    pub fn get(&mut self, column: usize) -> String {
        if let Some(line) = self.lines.get(&self.get_line_number()) {
            if let Some(field) = line.get(&column) {
                return field.to_string();
            }
        }
        "".to_string()
    }

    fn parse_input_file(fs: &str, rs: &str, contents: String) -> HashMap<usize, Line> {
        let mut lines = HashMap::new();
        for (line_idx, line) in contents.split(rs).enumerate() {
            if line.len() == 0 {
                continue;
            }

            let mut map = HashMap::new();
            map.insert(0, line.to_string());
            for (field_idx, field) in line.split(fs).enumerate() {
                map.insert(field_idx + 1, field.to_string());
            }
            lines.insert(line_idx, map);
        }
        lines
    }

    fn advance_file(&mut self) -> bool {
        if let Some(next_file) = self.files.next() {
            let contents = match std::fs::read_to_string(PathBuf::from(next_file.clone())) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("Unable to load file @ `{}`\nErr: {}", next_file, err);
                    std::process::exit(-1);
                }
            };
            self.current_path = Some(next_file);
            self.lines = Columns::parse_input_file(&self.fs, &self.rs, contents);
            true
        } else {
            false
        }
    }

    pub fn next_line(&mut self) -> bool {
        if self.current_path.is_none() && !self.advance_file() {
            return false;
        }
        loop {
            let line_number = if let Some(line_num) = self.line_number {
                line_num
            } else {
                self.line_number = Some(0);
                return self.lines.get(&0).is_some();
            };
            if let Some(_next_line) = self.lines.get(&(line_number + 1)) {
                self.line_number = Some(line_number + 1);
                return true;
            }
            if self.advance_file() {
                self.line_number = None;
            } else {
                return false;
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_record_sep(&mut self, value: String) {
        if self.current_path.is_some() {
            panic!("must set fs/rs before reading lines")
        }
        let boxed = Box::new(value);
        let leaked: &'static str = Box::leak(boxed);
        self.rs = leaked;
    }

    #[allow(dead_code)]
    pub fn set_field_sep(&mut self, value: String) {
        if self.current_path.is_some() {
            panic!("must set fs/rs before reading lines")
        }
        let boxed = Box::new(value);
        let leaked: &'static str = Box::leak(boxed);
        self.fs = leaked;
    }
}

#[test]
fn test_parse_input_file() {
    let actual = Columns::parse_input_file(" ", "\n", "a b c\nd e f\ng h i".to_string());
    let mut map: HashMap<usize, Line> = HashMap::new();
    let mut line1: Line = HashMap::new();
    line1.insert(0, "a b c".to_string());
    line1.insert(1, "a".to_string());
    line1.insert(2, "b".to_string());
    line1.insert(3, "c".to_string());
    let mut line2: Line = HashMap::new();
    line2.insert(0, "d e f".to_string());
    line2.insert(1, "d".to_string());
    line2.insert(2, "e".to_string());
    line2.insert(3, "f".to_string());
    let mut line3: Line = HashMap::new();
    line3.insert(0, "g h i".to_string());
    line3.insert(1, "g".to_string());
    line3.insert(2, "h".to_string());
    line3.insert(3, "i".to_string());

    map.insert(0, line1);
    map.insert(1, line2);
    map.insert(2, line3);
    assert_eq!(actual, map)
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

    assert!(cols.next_line());
    assert_eq!(cols.get(0), "a b c");
    assert_eq!(cols.get(1), "a");
    assert_eq!(cols.get(2), "b");
    assert_eq!(cols.get(3), "c");
    assert!(cols.next_line());
    assert_eq!(cols.get(3), "f");
    assert_eq!(cols.get(2), "e");
    assert_eq!(cols.get(1), "d");
    assert_eq!(cols.get(0), "d e f");
    assert!(cols.next_line());
    assert_eq!(cols.get(3), "i");
    assert_eq!(cols.get(2), "h");
    assert_eq!(cols.get(1), "g");
    assert_eq!(cols.get(0), "g h i");
    assert!(cols.next_line());
    assert_eq!(cols.get(0), "1 2 3");
    assert_eq!(cols.get(3), "3");
    assert_eq!(cols.get(2), "2");
    assert_eq!(cols.get(1), "1");
    assert!(cols.next_line());
    assert_eq!(cols.get(3), "6");
    assert_eq!(cols.get(2), "5");
    assert_eq!(cols.get(1), "4");
    assert_eq!(cols.get(0), "4 5 6");
    assert!(cols.next_line());
    assert_eq!(cols.get(3), "9");
    assert_eq!(cols.get(2), "8");
    assert_eq!(cols.get(1), "7");
    assert_eq!(cols.get(0), "7 8 9");
    assert_eq!(cols.next_line(), false);
    assert_eq!(cols.next_line(), false);
}

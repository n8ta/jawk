use crate::runtime::columns::Columns;

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

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "a b c".as_bytes());
    assert_eq!(cols.get(1), "a".as_bytes());
    assert_eq!(cols.get(2), "b".as_bytes());
    assert_eq!(cols.get(3), "c".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(3), "f".as_bytes());
    assert_eq!(cols.get(2), "e".as_bytes());
    assert_eq!(cols.get(1), "d".as_bytes());
    assert_eq!(cols.get(0), "d e f".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(3), "i".as_bytes());
    assert_eq!(cols.get(2), "h".as_bytes());
    assert_eq!(cols.get(1), "g".as_bytes());
    assert_eq!(cols.get(0), "g h i".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "1 2 3".as_bytes());
    assert_eq!(cols.get(3), "3".as_bytes());
    assert_eq!(cols.get(2), "2".as_bytes());
    assert_eq!(cols.get(1), "1".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(3), "6".as_bytes());
    assert_eq!(cols.get(2), "5".as_bytes());
    assert_eq!(cols.get(1), "4".as_bytes());
    assert_eq!(cols.get(0), "4 5 6".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(3), "9".as_bytes());
    assert_eq!(cols.get(2), "8".as_bytes());
    assert_eq!(cols.get(1), "7".as_bytes());
    assert_eq!(cols.get(0), "7 8 9".as_bytes());
    assert_eq!(cols.next_record().unwrap(), false);
    assert_eq!(cols.next_record().unwrap(), false);
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

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "a b c".as_bytes());
    cols.set_rs("-".as_bytes().to_vec());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "\n".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "ZZZ1".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "ZZZ2".as_bytes());
    assert_eq!(cols.next_record().unwrap(), false);
    assert_eq!(cols.next_record().unwrap(), false);
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

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "1 2 3".as_bytes());
    assert_eq!(cols.next_record().unwrap(), false);
}


#[test]
fn test_setting_fields() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "1 2 3\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(0), "1 2 3".as_bytes());
    let c = "c".as_bytes();
    cols.set(2, c);
    assert_eq!(cols.get(0), "1 c 3".as_bytes());
    assert_eq!(cols.get(1), "1".as_bytes());
    assert_eq!(cols.get(2), "c".as_bytes());
    assert_eq!(cols.get(3), "3".as_bytes());
}


#[test]
fn test_setting_0() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "1 2 3\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(1), "1".as_bytes());
    let c = "A B C".as_bytes();
    cols.set(0, c);
    assert_eq!(cols.get(1), "A".as_bytes());
    assert_eq!(cols.get(2), "B".as_bytes());
    assert_eq!(cols.get(3), "C".as_bytes());
    assert_eq!(cols.get(0), "A B C".as_bytes());
}

#[test]
fn test_setting_fs_0() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "A B C\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(2), "B".as_bytes());
    cols.set_fs("B".as_bytes().to_vec());
    cols.set(0, "A B C".as_bytes());
    assert_eq!(cols.get(2), " C".as_bytes());
    assert_eq!(cols.get(1), "A ".as_bytes());
    assert_eq!(cols.get(0), "A B C".as_bytes());
    cols.set(2, "Z".as_bytes());
    assert_eq!(cols.get(0), "A B Z".as_bytes());
}

#[test]
fn test_setting_fs_1() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "abc\nabc\nabc").unwrap();
    let mut cols = Columns::new(vec![file_path_1.to_str().unwrap().to_string(), ]);

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(2), "".as_bytes());
    cols.set_fs("b".as_bytes().to_vec());

    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(1), "a".as_bytes());
    assert_eq!(cols.get(2), "c".as_bytes());
    assert!(cols.next_record().unwrap());
    assert_eq!(cols.get(1), "a".as_bytes());
    assert_eq!(cols.get(2), "c".as_bytes());
}

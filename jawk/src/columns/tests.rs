use crate::columns::Columns;

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


#[test]
fn test_setting_fields() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("file1.txt");
    std::fs::write(file_path_1.clone(), "1 2 3\n").unwrap();

    let mut cols = Columns::new(vec![
        file_path_1.to_str().unwrap().to_string(),
    ]);

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(0), "1 2 3".into());
    let c = "c".as_bytes();
    cols.set(2, c);
    assert_eq!(cols.get(0), "1 c 3".into());
    assert_eq!(cols.get(1), "1".into());
    assert_eq!(cols.get(2), "c".into());
    assert_eq!(cols.get(3), "3".into());
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

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(1), "1".into());
    let c = "A B C".as_bytes();
    cols.set(0, c);
    assert_eq!(cols.get(1), "A".into());
    assert_eq!(cols.get(2), "B".into());
    assert_eq!(cols.get(3), "C".into());
    assert_eq!(cols.get(0), "A B C".into());
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

    assert!(cols.next_line().unwrap());
    assert_eq!(cols.get(2), "B".into());
    cols.set_field_sep("B".as_bytes());
    cols.set(0, "A B C".as_bytes());
    assert_eq!(cols.get(2), " C".into());
    assert_eq!(cols.get(1), "A ".into());
    assert_eq!(cols.get(0), "A B C".into());
    cols.set(2, "Z".as_bytes());
    assert_eq!(cols.get(0), "A B Z".into());
}

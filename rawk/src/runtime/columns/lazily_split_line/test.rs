#[cfg(test)]
mod test {
    use quick_drop_deque::QuickDropDeque;
    use crate::runtime::columns::lazily_split_line::LazilySplitLine;

    const A: u8 = 64;
    const B: u8 = 65;
    const C: u8 = 66;
    const SPACE: u8 = 32;
    const NL: u8 = 10;

    #[test]
    fn test_space_behavior() {
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, SPACE, A]);
        assert_eq!(LazilySplitLine::new().nf(&dq, dq.len()), 4);
    }

    #[test]
    fn test_non_space_behavior() {
        let dq = QuickDropDeque::from(vec![A, B, A, A, B, B, A]);
        let mut split = LazilySplitLine::new();
        split.set_field_sep(vec![B]);
        split.next_record();
        assert_eq!(split.nf(&dq, dq.len()), 4);
    }

    #[test]
    fn test_splits_are_correct_with_no_rs() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, A, A, A]);
        assert_eq!(line.get(&dq, 1, 8).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 8).to_vec(), vec![A, A]);
        assert_eq!(line.get(&dq, 3, 8).to_vec(), vec![A, A, A]);
    }

    #[test]
    fn test_splits_are_correct_space_rules() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, SPACE, A, A, A]);
        assert_eq!(line.get(&dq, 1, 9).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 9).to_vec(), vec![A, A]);
        assert_eq!(line.get(&dq, 3, 9).to_vec(), vec![A, A, A]);
    }

    #[test]
    fn test_splits_are_correct_with_rs_no_space_rules() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, A, A, A, NL]);
        assert_eq!(line.get(&dq, 1, 8).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 8).to_vec(), vec![A, A]);
        assert_eq!(line.get(&dq, 3, 8).to_vec(), vec![A, A, A]);
    }

    #[test]
    fn tests_splits_ez() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, NL]);
        assert_eq!(line.get(&dq, 1, 1).to_vec(), vec![A]);
    }


    #[test]
    fn test_changing_fs() {
        let mut line = LazilySplitLine::new();
        let mut dq = QuickDropDeque::from(vec![A, SPACE, B, NL, B, A, C]);
        line.set_field_sep(vec![A]);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 3, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 30000, 3).to_vec(), vec![]);
        line.next_record();
        dq.drop_front(4);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![C]);
        assert_eq!(line.get(&dq, 20000, 3).to_vec(), vec![]);
    }

    #[test]
    fn test_getting_in_rev_order() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, B, NL, B, A, C]);
        assert_eq!(line.get(&dq, 30000, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 3, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![A]);
    }
}

use quick_drop_deque::QuickDropDeque;
use crate::columns::borrowing_split::{borrowing_split, Split};
use crate::columns::index_of::{index_in_dq, index_in_dq_shifted, index_in_dq_truncated};

pub struct LazilySplitLine {
    // splits: Vec<Split>,
    fs: Vec<u8>,
    next_fs: Option<Vec<u8>>,
}

const EMPTY_SLICE: &[u8] = &[];

impl LazilySplitLine {
    pub fn new() -> Self {
        Self {
            fs: vec![32], // space
            // splits: vec![],
            next_fs: None,
        }
    }

    pub fn set_fs(&mut self, fs: Vec<u8>) {
        self.next_fs = Some(fs);
    }

    pub fn next_record(&mut self) {
        if let Some(next_fs) = self.next_fs.take() {
            self.fs = next_fs;
        }
    }

    pub fn get(&mut self, dq: &QuickDropDeque, idx: usize, ends_at: usize) -> Vec<u8> {
        // dest_buf.clear();
        todo!();
        // debug_assert!(idx != 0); // $0 should be handled by RecordReader
        // let mut start_of_record = 0;
        // let mut records_found = 0;
        // while let Some(found_at) = index_in_dq_shifted(&self.fs, dq, start_of_record) {
        //     records_found += 1;
        //     if records_found == idx {
        //
        //
        //
        //     }
        //     start_of_record = found_at+self.fs.len();
        //
        // }
        // EMPTY_SLICE
    }

    pub fn nf(&mut self, dq: &QuickDropDeque) -> usize {
        todo!()
    }
}


#[cfg(test)]
mod test {
    use quick_drop_deque::QuickDropDeque;
    use crate::columns::lazily_split_line::LazilySplitLine;

    const A: u8 = 64;
    const B: u8 = 65;
    const C: u8 = 66;
    const SPACE: u8 = 32;
    const NL: u8 = 10;

    #[test]
    fn test_space_behavior() {
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, SPACE, A]);
        assert_eq!(LazilySplitLine::new().nf(&dq), 3);
    }

    #[test]
    fn test_non_space_behavior() {
        let dq = QuickDropDeque::from(vec![A, B, A, A, B, B, A]);
        let mut split = LazilySplitLine::new();
        split.set_fs(vec![B]);
        split.next_record();
        assert_eq!(LazilySplitLine::new().nf(&dq), 4);
    }

    #[test]
    fn test_splits_are_correct_with_no_rs() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, SPACE, A, A, A]);
        assert_eq!(line.get(&dq, 0, 10).to_vec(), vec![A, SPACE, A, A, SPACE, SPACE, A, A, A]);
        assert_eq!(line.get(&dq, 1, 10).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 10).to_vec(), vec![A, A]);
        assert_eq!(line.get(&dq, 3, 10).to_vec(), vec![A, A, A]);
    }

    #[test]
    fn test_splits_are_correct_with_rs() {
        let mut line = LazilySplitLine::new();
        let dq = QuickDropDeque::from(vec![A, SPACE, A, A, SPACE, SPACE, A, A, A, NL]);
        assert_eq!(line.get(&dq, 0, 10).to_vec(), vec![A, SPACE, A, A, SPACE, SPACE, A, A, A]);
        assert_eq!(line.get(&dq, 1, 10).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 10).to_vec(), vec![A, A]);
        assert_eq!(line.get(&dq, 3, 10).to_vec(), vec![A, A, A]);
    }

    #[test]
    fn test_changing_fs() {
        let mut line = LazilySplitLine::new();
        let mut dq = QuickDropDeque::from(vec![A, SPACE, B, NL, B, A, C]);
        line.set_fs(vec![A]);
        assert_eq!(line.get(&dq, 0, 3).to_vec(), vec![A, SPACE, A]);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 3, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 30000, 3).to_vec(), vec![]);
        line.next_record();
        dq.drop_front(4);
        assert_eq!(line.get(&dq, 0, 3).to_vec(), vec![B, A, C]);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![C]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 2, 3000).to_vec(), vec![]);
    }

    #[test]
    fn test_getting_in_rev_order() {
        let mut line = LazilySplitLine::new();
        let mut dq = QuickDropDeque::from(vec![A, SPACE, B, NL, B, A, C]);
        line.set_fs(vec![A]);
        assert_eq!(line.get(&dq, 30000, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 3, 3).to_vec(), vec![]);
        assert_eq!(line.get(&dq, 2, 3).to_vec(), vec![B]);
        assert_eq!(line.get(&dq, 1, 3).to_vec(), vec![A]);
        assert_eq!(line.get(&dq, 0, 3).to_vec(), vec![A, SPACE, B]);
    }
}

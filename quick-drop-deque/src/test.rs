#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::env::temp_dir;
    use std::fs::File;
    use crate::QuickDropDeque;
    use super::*;

    fn to_vec(dq: &QuickDropDeque) -> Vec<u8> {
        let length = dq.len();
        let (left, right) = dq.as_slices();
        let mut res = Vec::with_capacity(left.len() + right.len());
        for x in left {
            res.push(*x);
        }
        for x in right {
            res.push(*x);
        }
        res
    }

    #[test]
    fn test_from_small_file() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path_1 = temp_dir.path().join("file1.txt");
        let mut bytes = vec![];
        for i in 0..10 {
            bytes.push(i);
        }
        std::fs::write(file_path_1.clone(), bytes).unwrap();

        let mut file = File::open(file_path_1).unwrap();

        let mut dq = QuickDropDeque::new();
        assert_eq!(dq.read(&mut file).unwrap(), 10);
        assert_eq!(to_vec(&dq), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_from_large_file() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path_1 = temp_dir.path().join("file1.txt");
        let mut bytes = vec![];
        for i in 0..10_000 {
            bytes.push((i % 255) as u8);
        }
        std::fs::write(file_path_1.clone(), &bytes).unwrap();

        let mut file = File::open(file_path_1).unwrap();

        let mut dq = QuickDropDeque::new();

        while let bytes = dq.read(&mut file).unwrap() {
            println!("Read {}", bytes);
            if bytes == 0 { break; }
        }
        assert_eq!(to_vec(&dq), bytes);
    }

    #[test]
    fn test_disjoint_reads() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path_1 = temp_dir.path().join("file1.txt");
        let mut bytes = vec![];
        for i in 0..10_000 {
            bytes.push((i % 255) as u8);
        }
        std::fs::write(file_path_1.clone(), &bytes).unwrap();

        let mut file = File::open(file_path_1).unwrap();

        // 8192
        let mut dq = QuickDropDeque::with_io_size(8, 4);

        dq.extend_from_slice(&[0, 1, 2, 3, 4, 5]);
        // [0, 1, 2, 3, 4, 5, _, _ ]

        dq.drop_front(3);
        // [_, _, _, 3, 4, 5, _, _ ]

        assert_eq!(4, dq.read(&mut file).unwrap());
        assert_eq!(to_vec(&dq), vec![3, 4, 5, 0, 1, 2, 3]);
    }

    #[test]
    fn test_regular_dq() {
        let mut dq = VecDeque::with_capacity(4);
        dq.extend(vec![0, 1, 2, 3]);
        assert_eq!(dq.len(), 4)
    }

    #[test]
    fn test_len_4() {
        let mut dq = QuickDropDeque::with_capacity(4);
        dq.extend_from_slice(&[0, 1, 2, 3]);
        assert_eq!(dq.len(), 4)
    }


    #[test]
    fn test_len_8() {
        let mut dq = QuickDropDeque::with_capacity(4);
        dq.extend_from_slice(&[0, 1, 2, 3]);
        dq.extend_from_slice(&[0, 1, 2, 3]);
        assert_eq!(dq.len(), 8)
    }

    #[test]
    fn it_works() {
        let mut deque = QuickDropDeque::new();
        deque.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(deque.len(), 4);
        deque.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(deque.len(), 8);
        deque.extend_from_slice(&[3, 3, 3, 3]);
        assert_eq!(deque.len(), 12);
        let slices = deque.as_slices();
        assert_eq!(slices.0.len() + slices.1.len(), 12);
        assert_eq!(vec![1, 2, 3, 4, 1, 2, 3, 4, 3, 3, 3, 3], to_vec(&deque))
    }

    #[test]
    fn many_pushes() {
        let mut std_dq: VecDeque<u8> = VecDeque::new();
        let mut deque = QuickDropDeque::new();
        deque.extend_from_slice(&[1, 2, 3, 4]);
        deque.extend_from_slice(&[1, 2, 3, 4]);
        deque.extend_from_slice(&[3, 3, 3, 3]);
        std_dq.extend(&[1, 2, 3, 4]);
        std_dq.extend(&[1, 2, 3, 4]);
        std_dq.extend(&[3, 3, 3, 3]);

        for _ in 0..5000 {
            deque.extend_from_slice(&[3, 3, 3, 3, 4, 4, 4, 4]);
            std_dq.extend(&[3, 3, 3, 3, 4, 4, 4, 4]);
        }
        assert_eq!(std_dq.into_iter().collect::<Vec<u8>>(), to_vec(&deque))
    }

    #[test]
    fn many_pushes_and_drop() {
        let mut std_dq: VecDeque<u8> = VecDeque::new();
        let mut deque = QuickDropDeque::new();

        for _ in 0..50 {
            let slice = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            deque.extend_from_slice(&slice);
            std_dq.extend(&slice);
            std_dq.drain(0..5);
            deque.drop_front(5);
            println!("{:?}", deque.as_slices());
        }
        println!("{:?}", std_dq.as_slices());
        assert_eq!(std_dq.into_iter().collect::<Vec<u8>>(), to_vec(&deque))
    }

    #[test]
    fn from_vec_len_pow2() {
        let mut d = QuickDropDeque::from(vec![0, 1, 2, 3]);
        assert_eq!(d.len(), 4);
    }

    #[test]
    fn from_len_extend_exactly_to_pow2() {
        let mut d = QuickDropDeque::from(vec![0, 1, 2, 3]);
        d.extend_from_slice(&[4, 5, 6, 7]);
        assert_eq!(d.len(), 8);
    }

    #[test]
    fn from_vec_len_non_pow2() {
        let mut d = QuickDropDeque::from(vec![0, 1, 2, 3, 4]);
        assert_eq!(d.len(), 5)
    }
}


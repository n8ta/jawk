use quick_drop_deque::QuickDropDeque;
pub fn index_of<T: PartialEq>(needle: &[T], haystack: &[T]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|subslice| subslice == needle)
}


pub fn index_in_dq(needle: &[u8], haystack: &QuickDropDeque) -> Option<usize> {
    if needle.len() == 1 {
        let needle = needle[0];
        let slices = haystack.as_slices();
        let left_len = slices.0.len();
        for (idx, elem) in slices.0.iter().enumerate() {
            if *elem == needle { return Some(idx) }
        }
        for elem in slices.1.iter().enumerate() {
            if *elem.1 == needle { return Some(elem.0 + left_len) }
        }
        return None;
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let hlen = haystack.len();
    let nlen = needle.len();
    for idx in 0..(hlen-nlen)+1 {
        let mut matches = true;
        for needle_idx in 0..needle.len() {
            if needle[needle_idx] != haystack[idx+needle_idx] {
                matches = false;
                break
            }
        }
        if matches {
            return Some(idx);
        }

    }
    None
}

#[cfg(test)]
mod index_of_tests {
    use quick_drop_deque::QuickDropDeque;
    use crate::columns::index_of::{index_of, index_in_dq};

    #[test]
    fn test_index_of() {
        assert_eq!(index_of(&[1, 2, 3], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[2, 3], &[1, 2, 3]), Some(1));
        assert_eq!(index_of(&[1, 2], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[1], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[1, 2, 3], &[]), None);
        assert_eq!(index_of(&[1, 2, 3], &[0, 1, 2, 3]), Some(1));
        assert_eq!(index_of(&[1, 2, 3,4,5,6,7,8], &[0, 1, 2, 3]), None);
    }

    #[test]
    fn test_index_of_dq_0() {
        // assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![1, 2, 3])), Some(0));
        // assert_eq!(index_in_dq(&[2, 3], &QuickDropDeque::from(vec![1, 2, 3])), Some(1));
        // assert_eq!(index_in_dq(&[1, 2], &QuickDropDeque::from(vec![1, 2, 3])), Some(0));
        // assert_eq!(index_in_dq(&[1], &QuickDropDeque::from(vec![1, 2, 3])), Some(0));
        // assert_eq!(index_in_dq(&[], &QuickDropDeque::from(vec![1, 2, 3])), Some(0));
        // assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![])), None);
        assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![0, 1, 2, 3])), Some(1));
        // assert_eq!(index_in_dq(&[1, 2, 3,4,5,6,7,8], &QuickDropDeque::from(vec![0, 1, 2, 3])), None);
    }

    #[test]
    fn test_index_of_dq_with_shifting() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1,2,3,4]);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.drop_front(2);
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.extend_from_slice(&[5, 6]);
        // head:2 buf: [5,6,3,4]
        assert_eq!(index_in_dq(&[3,4,5], &shifted_dequeue), Some(0));
    }

    #[test]
    fn test_index_of_dq_not_at_0() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1,2,3,4]);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.drop_front(2);
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.extend_from_slice(&[5]);
        assert_eq!(index_in_dq(&[4,5], &shifted_dequeue), Some(1));
    }
}
use std::collections::VecDeque;

pub fn index_of<T: PartialEq>(needle: &[T], haystack: &[T]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|subslice| subslice == needle)
}


pub fn index_in_dq<T: PartialEq + Copy>(needle: &[T], haystack: &VecDeque<T>) -> Option<usize> {
    if needle.len() == 1 {
        let zero = needle[0];
        return match haystack.into_iter().enumerate().find(|elem| *(elem.1) == zero) {
            None => None,
            Some(idx) => Some(idx.0),
        }
    }
    if needle.len() > haystack.len() {
        return None;
    }
    for idx in 0..(haystack.len() - needle.len())+1 {
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
    use std::collections::VecDeque;
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
    fn test_index_of_dq() {
        assert_eq!(index_in_dq(&[1, 2, 3], &VecDeque::from(vec![1, 2, 3])), Some(0));
        assert_eq!(index_in_dq(&[2, 3], &VecDeque::from(vec![1, 2, 3])), Some(1));
        assert_eq!(index_in_dq(&[1, 2], &VecDeque::from(vec![1, 2, 3])), Some(0));
        assert_eq!(index_in_dq(&[1], &VecDeque::from(vec![1, 2, 3])), Some(0));
        assert_eq!(index_in_dq(&[], &VecDeque::from(vec![1, 2, 3])), Some(0));
        assert_eq!(index_in_dq(&[1, 2, 3], &VecDeque::from(vec![])), None);
        assert_eq!(index_in_dq(&[1, 2, 3], &VecDeque::from(vec![0, 1, 2, 3])), Some(1));
        assert_eq!(index_in_dq(&[1, 2, 3,4,5,6,7,8], &VecDeque::from(vec![0, 1, 2, 3])), None);
    }

    #[test]
    fn test_index_of_dq_with_shifting() {
        let mut shifted_dequeue = VecDeque::with_capacity(4);
        shifted_dequeue.push_back(1);
        shifted_dequeue.push_back(2);
        shifted_dequeue.push_back(3);
        shifted_dequeue.push_back(4);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.pop_front();
        shifted_dequeue.pop_front();
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.push_back(5);
        shifted_dequeue.push_back(6);
        // head:2 buf: [5,6,3,4]
        assert_eq!(index_in_dq(&[3,4,5], &shifted_dequeue), Some(0));
    }

    #[test]
    fn test_index_of_dq_not_at_0() {
        let mut shifted_dequeue = VecDeque::with_capacity(4);
        shifted_dequeue.push_back(1);
        shifted_dequeue.push_back(2);
        shifted_dequeue.push_back(3);
        shifted_dequeue.push_back(4);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.pop_front();
        shifted_dequeue.pop_front();
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.push_back(5);
        shifted_dequeue.push_front(0);
        // head:1 buf: [5,0,3,4]
        assert_eq!(index_in_dq(&[3,4,5], &shifted_dequeue), Some(1));
    }
}
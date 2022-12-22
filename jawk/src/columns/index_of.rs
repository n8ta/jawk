use std::cmp::min;
use libc::EMPTY;
use quick_drop_deque::QuickDropDeque;

pub fn index_of<T: PartialEq>(needle: &[T], haystack: &[T]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|subslice| subslice == needle)
}

fn memchr_libc(buf: &[u8], needle: u8) -> Option<usize> {
    let res = unsafe {
        libc::memchr(
            buf.as_ptr() as *const std::os::raw::c_void,
            needle as i32,
            buf.len())
    };
    if res == 0 as *mut std::os::raw::c_void {
        return None;
    }
    let mut base = buf.as_ptr();
    let res = res as *const u8;
    Some((unsafe { res.offset_from(base) }) as usize)
}

const EMPTY_SLICE: &[u8] = &[];

fn subslices_inner<'a, 'b>(left: &'a[u8], right: &'b[u8], start: usize, length: usize) -> (&'a[u8], &'b[u8]) {
    let llen = left.len();

    let left_start = min(llen, start);
    let left_end = min(llen, length + start);
    let elem_from_left = left_end - left_start;

    let right_start = start - left_start;
    let right_end = right_start + (length - elem_from_left);

    (&left[left_start..left_end], &right[right_start..right_end])
}

// Skip `start` elements and take `length` elements from the logical buffer made up
// up of `dq`s two slices
fn subslices(dq: &QuickDropDeque, start: usize, length: usize) -> (&[u8], &[u8]) {
    let dlen = dq.len();
    debug_assert!(dlen >= start + length);
    let (left, right) = dq.as_slices();
    subslices_inner(left, right, start, length)
}

#[inline(never)]
fn index_in_slices_multibyte(needle: &[u8], left: &[u8], right: &[u8]) -> Option<usize> {
    // Slow path
    let nlen = needle.len();
    let llen = left.len();
    let hlen = llen + right.len();
    for idx in 0..(hlen - nlen) + 1 {
        let mut matches = true;
        for needle_idx in 0..nlen {
            let sub_idx = idx + needle_idx;
            let haystack_at_idx = if sub_idx > llen { right[sub_idx - llen] } else { left[sub_idx] };
            if needle[needle_idx] != haystack_at_idx {
                matches = false;
                break;
            }
        }
        if matches {
            return Some(idx);
        }
    }
    None
}

fn index_in_slices(needle: &[u8], left: &[u8], right: &[u8]) -> Option<usize> {
    let hlen = left.len() + right.len();
    let nlen = needle.len();
    if nlen > hlen {
        return None;
    }
    if nlen == 1 {
        let needle = needle[0];
        if let Some(idx) = memchr_libc(left, needle) {
            return Some(idx);
        }
        if let Some(idx) = memchr_libc(right, needle) {
            return Some(idx + left.len());
        }
        return None;
    }
    return index_in_slices_multibyte(needle, left, right);
}

pub fn index_in_dq(needle: &[u8], haystack: &QuickDropDeque, start: usize, length: usize) -> Option<usize> {
    let (left, right) = subslices(haystack, start, length);
    return index_in_slices(needle, left, right);
}

// Only look at first `length` elements of the deque
pub fn index_in_dq_truncated(needle: &[u8], haystack: &QuickDropDeque, length: usize) -> Option<usize> {
    index_in_dq(needle, haystack, 0, length)
}

// Skip the first `num_skipped` elements when searching
pub fn index_in_dq_shifted(needle: &[u8], haystack: &QuickDropDeque, num_skipped: usize) -> Option<usize> {
    index_in_dq(needle, haystack, num_skipped, haystack.len() - num_skipped)
}

#[cfg(test)]
mod index_of_tests {
    use libc::EMPTY;
    use quick_drop_deque::QuickDropDeque;
    use crate::columns::index_of::{index_of, index_in_dq, index_in_dq_truncated, index_in_dq_shifted, subslices_inner, EMPTY_SLICE};

    #[test]
    fn test_index_of() {
        assert_eq!(index_of(&[1, 2, 3], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[2, 3], &[1, 2, 3]), Some(1));
        assert_eq!(index_of(&[1, 2], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[1], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[], &[1, 2, 3]), Some(0));
        assert_eq!(index_of(&[1, 2, 3], &[]), None);
        assert_eq!(index_of(&[1, 2, 3], &[0, 1, 2, 3]), Some(1));
        assert_eq!(index_of(&[1, 2, 3, 4, 5, 6, 7, 8], &[0, 1, 2, 3]), None);
    }

    #[test]
    fn test_index_of_dq_0() {
        assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![1, 2, 3]), 0, 3), Some(0));
        assert_eq!(index_in_dq(&[2, 3], &QuickDropDeque::from(vec![1, 2, 3]), 0, 3), Some(1));
        assert_eq!(index_in_dq(&[1, 2], &QuickDropDeque::from(vec![1, 2, 3]), 0, 3), Some(0));
        assert_eq!(index_in_dq(&[1], &QuickDropDeque::from(vec![1, 2, 3]), 0, 3), Some(0));
        assert_eq!(index_in_dq(&[], &QuickDropDeque::from(vec![1, 2, 3]), 0, 3), Some(0));
        assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![]), 0, 0), None);
        assert_eq!(index_in_dq(&[1, 2, 3], &QuickDropDeque::from(vec![0, 1, 2, 3]), 0, 4), Some(1));
        assert_eq!(index_in_dq(&[1, 2, 3, 4, 5, 6, 7, 8], &QuickDropDeque::from(vec![0, 1, 2, 3]), 0, 4), None);
    }

    #[test]
    fn test_index_of_dq_up_to() {
        assert_eq!(index_in_dq_truncated(&[1, 2, 3], &QuickDropDeque::from(vec![1, 2, 3]), 3), Some(0));
        assert_eq!(index_in_dq_truncated(&[2, 3], &QuickDropDeque::from(vec![1, 2, 3]), 3), Some(1));
        assert_eq!(index_in_dq_truncated(&[1, 2], &QuickDropDeque::from(vec![1, 2, 3]), 3), Some(0));
        assert_eq!(index_in_dq_truncated(&[1], &QuickDropDeque::from(vec![1, 2, 3]), 3), Some(0));
        assert_eq!(index_in_dq_truncated(&[], &QuickDropDeque::from(vec![1, 2, 3]), 3), Some(0));
        assert_eq!(index_in_dq_truncated(&[1, 2, 3], &QuickDropDeque::from(vec![]), 0), None);
        assert_eq!(index_in_dq_truncated(&[1, 2, 3], &QuickDropDeque::from(vec![0, 1, 2, 3]), 4), Some(1));
        assert_eq!(index_in_dq_truncated(&[1, 2, 3, 4, 5, 6, 7, 8], &QuickDropDeque::from(vec![0, 1, 2, 3]), 4), None);
        assert_eq!(index_in_dq_truncated(&[2, 3], &QuickDropDeque::from(vec![1, 2, 3]), 0), None);
    }

    #[test]
    fn test_index_of_dq_with_shifting() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1, 2, 3, 4]);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.drop_front(2);
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.extend_from_slice(&[5, 6]);
        // head:2 buf: [5,6,3,4]
        assert_eq!(index_in_dq(&[3, 4, 5], &shifted_dequeue, 0, 3), Some(0));
    }

    #[test]
    fn test_index_of_dq_trunc_with_shifting() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1, 2, 3, 4]);
        shifted_dequeue.drop_front(2);
        shifted_dequeue.extend_from_slice(&[5, 6]);
        assert_eq!(index_in_dq_truncated(&[3, 4, 5], &shifted_dequeue, 4), Some(0));
        assert_eq!(index_in_dq_truncated(&[3, 4, 5], &shifted_dequeue, 3), Some(0));
        assert_eq!(index_in_dq_truncated(&[3, 4, 5], &shifted_dequeue, 1), None);
        assert_eq!(index_in_dq_truncated(&[3, 4, 5], &shifted_dequeue, 2), None);
    }

    #[test]
    fn test_index_of_dq_shifted() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1, 2, 3, 4]);
        shifted_dequeue.drop_front(2);
        shifted_dequeue.extend_from_slice(&[5, 6]);
        // 3 4 5 6
        assert_eq!(index_in_dq_shifted(&[3, 4, 5], &shifted_dequeue, 0), Some(0));
        assert_eq!(index_in_dq_shifted(&[4, 5], &shifted_dequeue, 0), Some(1));
        assert_eq!(index_in_dq_shifted(&[5], &shifted_dequeue, 0), Some(2));
        assert_eq!(index_in_dq_shifted(&[6], &shifted_dequeue, 0), Some(3));

        assert_eq!(index_in_dq_shifted(&[3, 4, 5], &shifted_dequeue, 1), None);
        assert_eq!(index_in_dq_shifted(&[4, 5], &shifted_dequeue, 1), Some(0));
        assert_eq!(index_in_dq_shifted(&[5], &shifted_dequeue, 1), Some(1));
        assert_eq!(index_in_dq_shifted(&[6], &shifted_dequeue, 1), Some(2));
        assert_eq!(index_in_dq_shifted(&[5], &shifted_dequeue, 2), Some(0));
        assert_eq!(index_in_dq_shifted(&[6], &shifted_dequeue, 2), Some(1));
        assert_eq!(index_in_dq_shifted(&[6], &shifted_dequeue, 3), Some(0));
        assert_eq!(index_in_dq_shifted(&[6], &shifted_dequeue, 4), None);
    }

    #[test]
    fn test_index_of_dq_not_at_0() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1, 2, 3, 4]);
        // head:0 buf: [1,2,3,4]
        shifted_dequeue.drop_front(2);
        // head:2 buf: [x,x,3,4]
        shifted_dequeue.extend_from_slice(&[5]);
        assert_eq!(index_in_dq(&[4, 5], &shifted_dequeue, 0, 3), Some(1));
    }

    #[test]
    fn test_index_of_dq_trunc_not_at_0() {
        let mut shifted_dequeue = QuickDropDeque::with_capacity(4);
        shifted_dequeue.extend_from_slice(&[1, 2, 3, 4]);
        shifted_dequeue.drop_front(2);
        shifted_dequeue.extend_from_slice(&[5]);
        assert_eq!(index_in_dq_truncated(&[4, 5], &shifted_dequeue, 3), Some(1));
        assert_eq!(index_in_dq_truncated(&[4, 5], &shifted_dequeue, 2), None);
        assert_eq!(index_in_dq_truncated(&[4, 5], &shifted_dequeue, 1), None);
    }

    #[test]
    fn test_subslices() {
        let a: &[u8] = &[1,2,3];
        let b: &[u8] = &[4,5,6];
        let c: &[u8] = &[2,3];
        let d: &[u8] = &[4,5];
        assert_eq!(subslices_inner(a,b, 0, 6), (a,b));
        assert_eq!(subslices_inner(a,b, 3, 3), (EMPTY_SLICE,b));
        assert_eq!(subslices_inner(a,b, 0, 3), (a,EMPTY_SLICE));
        assert_eq!(subslices_inner(a,b, 1, 4), (c,d));
        assert_eq!(subslices_inner(a,b, 1, 2), (c,EMPTY_SLICE));
    }
}
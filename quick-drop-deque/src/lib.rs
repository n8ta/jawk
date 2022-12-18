use std::{cmp, ptr, slice};
use std::fs::File;
use std::mem::MaybeUninit;

pub struct QuickDropDeque {
    head: usize,
    tail: usize,
    buf: Vec<u8>,
}

impl QuickDropDeque {
    pub fn new() -> Self {
        let cap = 4;
        QuickDropDeque { tail: 0, head: 0, buf: Vec::with_capacity(cap) }
    }

    pub fn len(&self) -> usize {
        count(self.tail, self.head, self.cap())
    }

    fn is_full(&self) -> bool {
        self.cap() - self.len() == 1
    }

    fn cap(&self) -> usize {
        self.buf.capacity()
    }

    fn drop_front(&mut self, num: usize) {
        if self.len() < num {
            panic!("Cannot drop more elements than exist in deque");
        }
        let tail = self.tail;
        self.tail = self.wrap_add(self.tail, num);
    }

    fn grow(&mut self) {
        // Extend or possibly remove this assertion when valid use-cases for growing the
        // buffer without it being full emerge
        debug_assert!(self.is_full());
        let old_cap = self.cap();
        self.buf.reserve_exact(old_cap);
        assert!(self.cap() == old_cap * 2);
        unsafe {
            self.handle_capacity_increase(old_cap);
        }
        debug_assert!(!self.is_full());
    }

    unsafe fn handle_capacity_increase(&mut self, old_capacity: usize) {
        let new_capacity = self.cap();

        if self.tail <= self.head {
            // A
            // Nop
        } else if self.head < old_capacity - self.tail {
            // B
            unsafe {
                self.copy_nonoverlapping(old_capacity, 0, self.head);
            }
            self.head += old_capacity;
            debug_assert!(self.head > self.tail);
        } else {
            // C
            let new_tail = new_capacity - (old_capacity - self.tail);
            unsafe {
                self.copy_nonoverlapping(new_tail, self.tail, old_capacity - self.tail);
            }
            self.tail = new_tail;
            debug_assert!(self.head < self.tail);
        }
        debug_assert!(self.head < self.cap());
        debug_assert!(self.tail < self.cap());
        debug_assert!(self.cap().count_ones() == 1);
    }

    unsafe fn copy_nonoverlapping(&mut self, dst: usize, src: usize, len: usize) {
        debug_assert!(
            dst + len <= self.cap(),
            "cno dst={} src={} len={} cap={}",
            dst,
            src,
            len,
            self.cap()
        );
        debug_assert!(
            src + len <= self.cap(),
            "cno dst={} src={} len={} cap={}",
            dst,
            src,
            len,
            self.cap()
        );
        unsafe {
            ptr::copy_nonoverlapping(unsafe { self.ptr() }.add(src), self.ptr().add(dst), len);
        }
    }
    fn ptr(&self) -> *mut u8 {
        self.buf.as_ptr() as *mut u8
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.reserve(additional);
    }

    pub fn reserve(&mut self, additional: usize) {
        let old_cap = self.cap();
        let used_cap = self.len() + 1;
        let new_cap = used_cap
            .checked_add(additional)
            .and_then(|needed_cap| needed_cap.checked_next_power_of_two())
            .expect("capacity overflow");

        if new_cap > old_cap {
            self.buf.reserve_exact(new_cap);
            unsafe {
                self.handle_capacity_increase(old_cap);
            }
        }
    }
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let mut iter = slice.iter();
        while let Some(element) = iter.next() {
            if self.len() == self.capacity() {
                let (lower, _) = iter.size_hint();
                self.reserve(lower.saturating_add(1));
            }

            let head = self.head;
            self.head = self.wrap_add(self.head, 1);
            unsafe {
                self.buffer_write(head, *element);
            }
        }
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    unsafe fn buffer_write(&mut self, off: usize, value: u8) {
        unsafe {
            ptr::write(self.ptr().add(off), value);
        }
    }

    #[inline]
    fn wrap_index(&self, idx: usize) -> usize {
        wrap_index(idx, self.cap())
    }

    /// Returns the index in the underlying buffer for a given logical element
    /// index + addend.
    #[inline]
    fn wrap_add(&self, idx: usize, addend: usize) -> usize {
        wrap_index(idx.wrapping_add(addend), self.cap())
    }

    /// Returns the index in the underlying buffer for a given logical element
    /// index - subtrahend.
    #[inline]
    fn wrap_sub(&self, idx: usize, subtrahend: usize) -> usize {
        wrap_index(idx.wrapping_sub(subtrahend), self.cap())
    }
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap() - 1
    }


    unsafe fn buffer_as_slice(&self) -> &[MaybeUninit<u8>] {
        unsafe { slice::from_raw_parts(self.ptr() as *mut MaybeUninit<u8>, self.cap()) }
    }

    #[inline]
    pub fn as_slices(&self) -> (&[u8], &[u8]) {
        // Safety:
        // - `self.head` and `self.tail` in a ring buffer are always valid indices.
        // - `RingSlices::ring_slices` guarantees that the slices split according to `self.head` and `self.tail` are initialized.
        unsafe {
            let buf = self.buffer_as_slice();
            let (front, back) = ring_slices(buf, self.head, self.tail);

            (slice_assume_init_ref(front), slice_assume_init_ref(back))
        }
    }
}

fn ring_slices(buf: &[MaybeUninit<u8>], head: usize, tail: usize) -> (&[MaybeUninit<u8>], &[MaybeUninit<u8>]) {
    let contiguous = tail <= head;
    if contiguous {
        let (empty, buf) = buf.split_at(0);
        (&buf[tail..head], empty)
    } else {
        let (mid, right) = buf.split_at(tail);
        let (left, _) = mid.split_at(head);
        (right, left)
    }
}

fn count(tail: usize, head: usize, size: usize) -> usize {
    // size is always a power of 2
    (head.wrapping_sub(tail)) & (size - 1)
}

pub const unsafe fn slice_assume_init_ref(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { &*(slice as *const [MaybeUninit<u8>] as *const [u8]) }
}


#[inline]
fn wrap_index(index: usize, size: usize) -> usize {
    // size is always a power of 2
    debug_assert!(size.is_power_of_two());
    index & (size - 1)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
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
    fn it_works() {
        let mut deque = QuickDropDeque::new();
        deque.extend_from_slice(&[1, 2, 3, 4]);
        deque.extend_from_slice(&[1, 2, 3, 4]);
        deque.extend_from_slice(&[3, 3, 3, 3]);
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
            let slice = [1,2,3,4,5,6,7,8,9,10];
            deque.extend_from_slice(&slice);
            std_dq.extend(&slice);
            std_dq.drain(0..5);
            deque.drop_front(5);
            println!("{:?}", deque.as_slices());

        }
        println!("{:?}", std_dq.as_slices());
        assert_eq!(std_dq.into_iter().collect::<Vec<u8>>(), to_vec(&deque))
    }
}


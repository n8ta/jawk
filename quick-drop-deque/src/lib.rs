use std::{cmp, io, ptr, slice};
use std::fs::File;
use std::io::Read;
use std::mem::MaybeUninit;
use std::ops::Index;
use crate::raw_vec::RawVec;

mod raw_vec;
#[cfg(test)]
mod test;

pub struct QuickDropDeque {
    head: usize,
    tail: usize,
    buf: RawVec,
    io_size: usize,
}

impl From<Vec<u8>> for QuickDropDeque {
    fn from(vec: Vec<u8>) -> Self {
        // TODO: consume the vec instead of copying but this is test only atm so not a prio
        let mut dq = QuickDropDeque::with_capacity(vec.len());
        dq.extend_from_slice(&vec);
        dq
    }
}

impl Index<usize> for QuickDropDeque {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &u8 {
        self.get(index).expect("Out of bounds access")
    }
}

const DEFAULT_READ_SIZE: usize = 1024 * 8;

impl QuickDropDeque {
    pub fn new() -> Self {
        let cap = 4;
        let buf = RawVec::with_capacity(cap);
        QuickDropDeque { tail: 0, head: 0, buf, io_size: DEFAULT_READ_SIZE }
    }
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cmp::max(cap, 2).next_power_of_two();
        let buf = RawVec::with_capacity(cap);
        QuickDropDeque { tail: 0, head: 0, buf, io_size: DEFAULT_READ_SIZE }
    }
    pub fn with_io_size(cap: usize, io_size: usize) -> Self {
        let cap = cmp::max(cap, 2).next_power_of_two();
        let buf = RawVec::with_capacity(cap);
        QuickDropDeque { tail: 0, head: 0, buf, io_size }
    }
    pub fn get(&self, index: usize) -> Option<&u8> {
        if index < self.len() {
            let idx = self.wrap_add(self.tail, index);
            unsafe { Some(&*self.ptr().add(idx)) }
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        count(self.tail, self.head, self.cap())
    }

    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        self.cap() - self.len() == 1
    }

    fn cap(&self) -> usize {
        self.buf.capacity()
    }

    pub fn drop_front(&mut self, num: usize) {
        debug_assert!(self.len() >= num);
        self.tail = self.wrap_add(self.tail, num);
    }

    #[inline(never)]
    pub fn read(&mut self, file: &mut File) -> io::Result<usize> {
        let free_bytes = self.cap() - self.len();
        if free_bytes <= self.io_size {
            self.reserve(self.io_size);
        }
        debug_assert!(self.capacity() - self.len() >= self.io_size);
        let head_room = self.cap() - self.head;

        let bytes_read = if self.io_size <= head_room {
            let mut dest_slice = unsafe { std::slice::from_raw_parts_mut(self.ptr().add(self.head), self.io_size) };
            file.read(dest_slice)?
        } else {
            let mut slice_one = unsafe { std::slice::from_raw_parts_mut(self.ptr().add(self.head), head_room) };
            let mut bytes_read = file.read(&mut slice_one)?;
            if bytes_read == head_room {
                let mut slice_two = unsafe { std::slice::from_raw_parts_mut(self.ptr(), self.io_size - head_room) };
                bytes_read += file.read(&mut slice_two)?;
            }
            bytes_read
        };
        self.head = self.wrap_add(self.head, bytes_read);
        Ok(bytes_read)
    }

    #[allow(dead_code)]
    fn grow(&mut self) {
        // Extend or possibly remove this assertion when valid use-cases for growing the
        // buffer without it being full emerge
        debug_assert!(self.is_full());
        let old_cap = self.cap();
        self.buf.reserve_exact(old_cap, old_cap);
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
            ptr::copy_nonoverlapping(self.ptr().add(src), self.ptr().add(dst), len);
        }
    }
    fn ptr(&self) -> *mut u8 {
        self.buf.ptr()
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
            self.buf.reserve_exact(used_cap, new_cap - used_cap);
            unsafe {
                self.handle_capacity_increase(old_cap);
            }
        }
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let free_bytes = self.cap() - self.len();
        if slice.len() >= free_bytes {
            self.reserve(self.len() + slice.len() + 1);
        }
        unsafe {
            self.copy_slice(self.head, slice);
        }
        self.head = self.wrap_add(self.head, slice.len());
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    #[allow(dead_code)]
    unsafe fn buffer_write(&mut self, off: usize, value: u8) {
        unsafe {
            ptr::write(self.ptr().add(off), value);
        }
    }

    #[inline]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    unsafe fn copy_slice(&mut self, dst: usize, src: &[u8]) {
        debug_assert!(src.len() <= self.cap());
        let head_room = self.cap() - dst;
        if src.len() <= head_room {
            unsafe {
                ptr::copy_nonoverlapping(src.as_ptr(), self.ptr().add(dst), src.len());
            }
        } else {
            let (left, right) = src.split_at(head_room);
            unsafe {
                ptr::copy_nonoverlapping(left.as_ptr(), self.ptr().add(dst), left.len());
                ptr::copy_nonoverlapping(right.as_ptr(), self.ptr(), right.len());
            }
        }
    }

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
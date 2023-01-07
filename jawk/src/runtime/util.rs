pub fn clamp_to_slice_index(zero_indexed: f64, slice_len: usize) -> usize {
    if zero_indexed >= slice_len as f64 {
        slice_len
    } else if zero_indexed < 0.0 {
        0
    } else {
        // zero_indexed >= 0
        // zero_indexed < slice_len
        // [0, slice_len)
        zero_indexed.trunc() as usize
    }
}

pub fn clamp_to_max_len(max_chars: f64, start_idx: usize, slice_len: usize) -> usize {
    debug_assert!(slice_len >= start_idx);
    let max_chars = max_chars.trunc();
    if max_chars < 1.0 {
        0
        // >= 1
    } else if max_chars > (slice_len-start_idx) as f64 {
        slice_len - start_idx
    } else {
        max_chars as usize
    }
}
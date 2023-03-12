
use quick_drop_deque::QuickDropDeque;
use crate::util::{index_in_dq, subslices};

const SPACE: u8 = 32;

pub fn get(fs: &[u8], dq: &QuickDropDeque, field_idx: usize, end_of_record_idx: usize) -> Vec<u8> {
    let mut vec = vec![];
    get_into(fs, dq, field_idx, end_of_record_idx, &mut vec);
    vec
}

fn move_into_buf(dq: &QuickDropDeque, result: &mut Vec<u8>, start: usize, end: usize) {
    let (left, right) = subslices(dq, start, end);
    result.extend_from_slice(left);
    result.extend_from_slice(right);
    return;
}

pub fn get_into(fs: &[u8], dq: &QuickDropDeque, field_idx: usize, end_of_record_idx: usize, result: &mut Vec<u8>) {
    debug_assert!(field_idx != 0);
    let mut start_of_field = 0;
    let mut fields_found = 0;
    let fs_is_space = fs == &[SPACE]; // space
    while let Some(found_at) = index_in_dq(fs, dq, start_of_field, end_of_record_idx) {
        fields_found += 1;
        if fields_found == field_idx {
            move_into_buf(dq, result, start_of_field, found_at);
        }
        let mut spaces_after_record = 0;
        while fs_is_space && dq.get(found_at + spaces_after_record + 1) == Some(&SPACE) {
            spaces_after_record += 1;
        }
        start_of_field = found_at + fs.len() + spaces_after_record;
    }
    if fields_found + 1 == field_idx {
        // Trailing record
        move_into_buf(dq, result, start_of_field, end_of_record_idx);
    }
}
use crate::awk_str::RcAwkStr;

#[derive(Clone, Copy)]
pub struct RecordState {
    pub NR: f64,
    pub FNR: f64,
}

impl RecordState {
    pub fn new(NR: f64, FNR: f64) -> Self {
        Self { NR, FNR }
    }
}

pub struct RecordStateOutput {
    pub NR: f64,
    pub FNR: f64,
    pub next_record: bool,
    pub new_file: Option<RcAwkStr>,
}

impl RecordStateOutput {
    pub fn new(NR: f64, FNR: f64, next_record: bool, new_file: Option<RcAwkStr>) -> Self {
        Self { NR, FNR, next_record, new_file }
    }
}
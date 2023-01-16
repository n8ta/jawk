use crate::typing::AnalysisResults;
use crate::vm::vm_func::VmFunc;

pub struct VmProgram {
    pub functions: Vec<VmFunc>,
    pub analysis: AnalysisResults,
}

impl VmProgram {
    pub fn new(functions: Vec<VmFunc>, analysis: AnalysisResults) -> Self {
        Self { functions, analysis }
    }
    pub fn main(&self) -> &VmFunc {
        self.functions.iter().find(|f| f.is_main()).unwrap()
    }
}
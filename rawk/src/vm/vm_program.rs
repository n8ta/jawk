use crate::typing::{AnalysisResults, FunctionMap};
use crate::vm::vm_func::VmFunc;

pub struct VmProgram {
    pub functions: Vec<VmFunc>,
    pub analysis: AnalysisResults,

    // Test only
    pub func_map: FunctionMap,
}

impl VmProgram {
    pub fn new(functions: Vec<VmFunc>, analysis: AnalysisResults, func_map: FunctionMap) -> Self {
        Self { functions, analysis, func_map}
    }
    pub fn main(&self) -> &VmFunc {
        self.functions.iter().find(|f| f.is_main()).unwrap()
    }

    #[cfg(test)]
    pub fn pretty_print(&self) -> Vec<u8> {
        let mut s = vec![];
        for func in &self.functions {
            s.extend_from_slice("\n\n=-=-=-=-=-=-=-=-=-=-=-=-\nfn ".as_bytes());
            s.extend_from_slice(func.name().to_str().as_bytes());
            let id = format!(" {}", func.id());
            s.extend_from_slice(&id.as_bytes());
            s.extend_from_slice("\n=-=-=-=-=-=-=-=-=-=-=-=-\n".as_bytes());
            func.pretty_print(func, &self, &mut s)
        }
        s
    }

}

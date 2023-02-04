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
    pub fn pretty_print(&self) -> String {
        let mut s = String::new();
        for func in &self.functions {
            s.push_str("\n=-=-=-=-=-=-=-=-=-=-=-=-\nfn ");
            s.push_str(func.name().to_str());
            s.push_str("\n=-=-=-=-=-=-=-=-=-=-=-=-\n");
            func.pretty_print(func, &self, &mut s)
        }
        s
    }

}

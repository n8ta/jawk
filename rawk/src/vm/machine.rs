use std::rc::Rc;
use crate::arrays::Arrays;
use crate::awk_str::AwkStr;
use crate::columns::Columns;
use crate::typing::GlobalArrayId;
use crate::vm::{Code, VmFunc, VmProgram};
use crate::vm::converter::Converter;

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Str(Rc<AwkStr>),
    StrNum(Rc<AwkStr>),
    Num(f64),
}

enum ScalarOrBarrier {
    Barrier,
    Value(RuntimeValue),
}

enum ArrayOrBarrier {
    Barrier,
    Value(GlobalArrayId),
}

struct FunctionScope {
    scalar_base_offset: usize,
    // Scalar args start here on scalar_stack
    array_base_offset: usize,
    // Array args start here on array_stack
    ip: usize,
}

pub struct VirtualMachine {
    global_scalars: Vec<RuntimeValue>,

    //array stack
    ars: Vec<ArrayOrBarrier>,

    //scalar stack
    ss: Vec<ScalarOrBarrier>,

    scopes: Vec<FunctionScope>,
    arrays: Arrays,
    columns: Columns,
    converter: Converter,
}

impl VirtualMachine {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            ars: vec![],
            ss: vec![],
            scopes: vec![],
            columns: Columns::new(files),
            arrays: Arrays::new(),
            converter: Converter::new(),
            global_scalars: vec![],
        }
    }
    pub fn run(&mut self, program: &VmProgram) {
        self.arrays.allocate(program.analysis.global_arrays.len() as u16); // TODO u16max
        self.run_function(program.main(), program)
    }
    pub fn push(&mut self, scalar: RuntimeValue) {
        self.ss.push(ScalarOrBarrier::Value(scalar))
    }
    pub fn push_arr(&mut self, array_id: GlobalArrayId) {
        self.ars.push(ArrayOrBarrier::Value(GlobalArrayId))
    }
    pub fn array_barrier(&mut self) {
        self.ars.push(ArrayOrBarrier::Barrier)
    }
    pub fn scalar_barrier(&mut self) {
        self.ss.push(ScalarOrBarrier::Barrier)
    }

    pub fn pop_scalar(&mut self) -> RuntimeValue {
        if let Some(popped) = self.ss.pop() {
            match popped {
                ScalarOrBarrier::Barrier => panic!("Compiler bug unexpected barrier"),
                ScalarOrBarrier::Value(v) => v,
            }
        } else {
            panic!("Compiler but missing stack value")
        }
    }
    fn run_function(&mut self, function: &VmFunc, program: &VmProgram) {
        let mut ip = 0;
        loop {
            let code = &function[ip];
            match code {
                Code::FloatZero => self.push(RuntimeValue::Num(0.0)),
                Code::FloatOne => self.push(RuntimeValue::Num(1.0)),
                Code::ScalarBarrier => self.push(RuntimeValue::Num(0.0)),
                Code::ArrayBarrier => self.array_barrier(),
                Code::Pop => { self.ss.pop().unwrap(); }
                Code::Column => {
                    let index = self.pop_scalar();
                    let idx = match index {
                        RuntimeValue::Str(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
                        RuntimeValue::StrNum(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
                        RuntimeValue::Num(n) => n,
                    };
                    let idx = idx.round() as usize;
                    let field = self.columns.get(idx);
                    self.push(RuntimeValue::StrNum(Rc::new(field)));
                }
                Code::NextLine => {
                    let more_lines = self.columns.next_line().unwrap(); // TODO: unwrap
                    let num = if more_lines { 1.0 } else { 0.0 };
                    self.push(RuntimeValue::Num(num));
                }
                Code::GlobalScalarAssign(idx) => {
                    let scalar = self.pop_scalar();
                    self.global_scalars[idx.id as usize] = scalar;
                }
                Code::GlobalScalar(idx) => {
                    self.push(self.global_scalars[idx.id as usize].clone())
                }
                Code::ArgScalarAssign { arg_idx } => {
                    
                }
                Code::ArgScalar { .. } => {}
                Code::Exp => {}
                Code::UnaryPlus => {}
                Code::UnaryMinus => {}
                Code::Mult => {}
                Code::Div => {}
                Code::Mod => {}
                Code::Add => {}
                Code::Sub => {}
                Code::Lt => {}
                Code::Gt => {}
                Code::LtEq => {}
                Code::GtEq => {}
                Code::EqEq => {}
                Code::Neq => {}
                Code::Matches => {}
                Code::NotMatches => {}
                Code::Concat { .. } => {}
                Code::GlobalArray(_) => {}
                Code::ArgArray { .. } => {}
                Code::ArrayMember { .. } => {}
                Code::ArrayAssign { .. } => {}
                Code::ArrayIndex { .. } => {}
                Code::Call { .. } => {}
                Code::Print => {}
                Code::Printf { .. } => {}
                Code::NoOp => {}
                Code::Ret => {}
                Code::ConstantLookup { .. } => {}
                Code::JumpIfFalseLbl(_) => {}
                Code::JumpLbl(_) => {}
                Code::JumpIfTrueLbl(_) => {}
                Code::Label(_) => {}
                Code::RelJumpIfFalse { .. } => {}
                Code::RelJumpIfTrue { .. } => {}
                Code::RelJump { .. } => {}
            }
            ip += 1;
        }
    }
}
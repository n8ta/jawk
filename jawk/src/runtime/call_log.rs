// Used by the test runtime for debugging

#[derive(Clone, Debug)]
pub enum Call {
    NextLine,
    Column(f64),
    FreeString,
    StringToNumber,
    CopyString,
    NumberToString,
    Concat,
    PrintString,
    EmptyString,
    PrintFloat,
    BinOp,
    ArrayAssign,
    ArrayAccess,
    InArray,
    ConcatArrayIndices,
    ToLower,
    ToUpper,
    Rand,
    Srand,
    Length,
    Split,
    SplitEre,
    // Printf,
}

pub struct CallLog {
    pub log: Vec<Call>,
}

impl CallLog {
    pub fn new() -> Self {
        CallLog { log: vec![] }
    }
    pub fn log(&mut self, call: Call) {
        println!("call: {:?}", call);
        self.log.push(call)
    }
}

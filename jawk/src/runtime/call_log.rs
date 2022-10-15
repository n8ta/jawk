// Used by the test runtime for debugging

#[derive(Clone, Debug)]
pub enum Call {
    NextLine,
    Column(f64, String),
    FreeString,
    StringToNumber,
    CopyString,
    NumberToString,
    Concat,
    PrintString,
    EmptyString,
    PrintFloat,
    BinOp,
    // Malloc,
    // Realloc,
    Free,
    ArrayAssign,
    ArrayAccess,
    InArray,
    ConcatArrayIndices,
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

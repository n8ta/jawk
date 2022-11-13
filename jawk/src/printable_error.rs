use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct PrintableError {
    pub msg: String,
}

impl Display for PrintableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl PrintableError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        PrintableError { msg: msg.into() }
    }
}

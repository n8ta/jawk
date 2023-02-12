use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

#[derive(Clone)]
pub struct IoCapture {
    pub buf: Rc<RefCell<Vec<u8>>>,
}

impl IoCapture {
    pub fn new() -> Self { Self { buf: Rc::new(RefCell::new(vec![])) } }
    pub fn collect(&self) -> Vec<u8> {
        let buf = self.buf.borrow();
        let buf: Vec<u8> = buf.clone();
        buf
    }
}

impl Write for IoCapture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut io_buf = self.buf.borrow_mut();
        io_buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

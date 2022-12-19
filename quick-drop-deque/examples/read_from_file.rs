use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use quick_drop_deque::QuickDropDeque;

pub fn main() {
    let start = Instant::now();
    let mut file = File::open(Path::new("/Users/n8ta/Desktop/short.csv")).unwrap();
    let mut dq = QuickDropDeque::with_capacity(4);
    let mut iters = 0;
    while let bytes = dq.read(&mut file).unwrap() {
        iters += 1;
        if bytes == 0 {
            break;
        }
        if iters % 4 == 0 {
            dq.drop_front(dq.len());
        }
    }
    println!("{} - {}ms", dq.len(), start.elapsed().as_millis());
}
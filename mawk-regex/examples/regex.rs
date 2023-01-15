use std::rc::Rc;
use mawk_regex::Regex;

fn main() {
    let regex = Regex::new("c+".as_bytes());
    println!("{}", regex.matches("dddd".as_bytes()));
    println!("{}", regex.matches("cccc".as_bytes()));

    let rc_str = Rc::new(String::from("c+"));
    let regex2 = Regex::new(&*rc_str.as_bytes());
    println!("{}", regex2.matches("dddd".as_bytes()));
    println!("{}", regex2.matches("cccc".as_bytes()));
}
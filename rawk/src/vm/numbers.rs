fn string_to_number(ptr: &Rc<AwkStr>) -> f64 {
    let res = data.converter.str_to_num(&*string).unwrap_or(0.0);
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

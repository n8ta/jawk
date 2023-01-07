use gnu_libjit::Value;

pub struct CodegenConsts {
    // Used to init the pointer section of the value struct when it's undefined. Should never be dereferenced.
    pub zero_ptr: Value,
    // Used to init the float section of value. Safe to use but using it is a bug.
    pub zero_f: Value,
    // To avoid creating tons of constants just reuse the tags here
    pub float_tag: Value,
    pub string_tag: Value,
    pub strnum_tag: Value,
    pub sentinel_float: Value,
}

impl CodegenConsts {
    pub fn new(
        zero_ptr: Value,
        zero_f: Value,
        float_tag: Value,
        string_tag: Value,
        strnum_tag: Value,
        sentinel_float: Value,
    ) -> Self {
        Self {
            zero_f,
            zero_ptr,
            float_tag,
            string_tag,
            strnum_tag,
            sentinel_float,
        }
    }
}

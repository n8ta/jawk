use crate::codgen::ValueT;
use crate::runtime::Runtime;
use gnu_libjit::{Abi, Context, Function, JitType, Label};
use std::os::raw::c_long;

#[allow(dead_code)]
// Lazily constructs subroutines for use by the codegen module.
pub struct Subroutines {
    string_truthy: Option<Function>,
    to_float: Option<Function>,
    get_byte_from_string: Option<Function>,
    // Takes a *mut String
    get_byte_from_vec: Option<Function>,
}

impl Subroutines {
    pub fn new<RuntimeT: Runtime>(_ctx: &mut Context, _runtime: &mut RuntimeT) -> Subroutines {
        Subroutines {
            string_truthy: None,
            to_float: None,
            get_byte_from_string: None,
            get_byte_from_vec: None,
        }
    }

    pub fn string_truthy<RuntimeT: Runtime>(
        &mut self,
        ctx: &mut Context,
        runtime: &mut RuntimeT,
    ) -> &Function {
        self.string_truthy
            .get_or_insert_with(|| Subroutines::gen_string_truthy(ctx, runtime))
    }
    pub fn to_float<RuntimeT: Runtime>(
        &mut self,
        ctx: &mut Context,
        runtime: &mut RuntimeT,
    ) -> &Function {
        self.to_float
            .get_or_insert_with(|| Subroutines::gen_to_float(ctx, runtime))
    }

    // Involves two pointer de-refs so not good for inner-loops. This accepts a *mut String
    #[allow(dead_code)]
    pub fn get_byte_from_string<RuntimeT: Runtime>(
        &mut self,
        ctx: &mut Context,
        runtime: &mut RuntimeT,
    ) -> &Function {
        self.get_byte_from_string
            .get_or_insert_with(|| Subroutines::gen_get_byte_from_string(ctx, runtime))
    }

    // One pointer de-ref. This accepts a *mut u8 (like you find inside a *mut String)
    #[allow(dead_code)]
    pub fn get_byte_from_vec<RuntimeT: Runtime>(
        &mut self,
        ctx: &mut Context,
        runtime: &mut RuntimeT,
    ) -> &Function {
        self.get_byte_from_vec
            .get_or_insert_with(|| Subroutines::gen_get_byte_from_vec(ctx, runtime))
    }

    // ValueT type as a vec of JitTypes
    fn value() -> Vec<JitType> {
        return vec![
            Context::sbyte_type(),
            Context::float64_type(),
            Context::void_ptr_type(),
        ];
    }

    fn gen_string_truthy<RuntimeT: Runtime>(
        ctx: &mut Context,
        _runtime: &mut RuntimeT,
    ) -> Function {
        // Rust string layout: [ptr, cap, len]
        // To get the length just add sizeof(ptr) and sizeof(usize) to the base ptr and de-reference.
        let mut func = ctx
            .function(
                Abi::Cdecl,
                &Context::int_type(),
                vec![Context::void_ptr_type()],
            )
            .expect("unable to create a new sub routine");
        let string_len_offset = std::mem::size_of::<usize>() + std::mem::size_of::<*const u8>();
        let arg0 = func.arg(0).unwrap();
        let zero_ulong = func.create_ulong_constant(0);
        let string_len =
            func.insn_load_relative(&arg0, string_len_offset as c_long, &Context::long_type());
        let ret = func.insn_ne(&zero_ulong, &string_len);
        func.insn_return(&ret);
        func.compile();
        func
    }
    fn gen_to_float<RuntimeT: Runtime>(ctx: &mut Context, runtime: &mut RuntimeT) -> Function {
        let mut func = ctx
            .function(Abi::Cdecl, &Context::float64_type(), Subroutines::value())
            .expect("unable to create a new sub routine");
        let zero = func.create_sbyte_constant(0);
        let result = func.create_value_float64();
        let mut done_lbl = Label::new();
        let arg0 = ValueT::new(
            func.arg(0).unwrap(),
            func.arg(1).unwrap(),
            func.arg(2).unwrap(),
        );
        func.insn_store(&result, &arg0.float);
        let is_float = func.insn_eq(&arg0.tag, &zero);
        func.insn_branch_if(&is_float, &mut done_lbl);

        let res = runtime.string_to_number(&mut func, arg0.pointer.clone());
        func.insn_store(&result, &res);

        func.insn_label(&mut done_lbl);
        let ret = func.insn_load(&result);
        func.insn_return(&ret);
        func.compile();
        func
    }
    fn gen_get_byte_from_string<RuntimeT: Runtime>(
        ctx: &mut Context,
        _runtime: &mut RuntimeT,
    ) -> Function {
        let mut func = ctx
            .function(
                Abi::Cdecl,
                &Context::ubyte_type(),
                vec![Context::void_ptr_type(), Context::ulong_type()],
            )
            .unwrap();
        let ptr = func.arg(0).unwrap();
        let idx = func.arg(1).unwrap();

        let char_0_ptr = func.insn_load_relative(&ptr, 0 as c_long, &Context::void_ptr_type());
        let char_n_ptr = func.insn_load_elem_address(&char_0_ptr, &idx, &Context::ubyte_type());
        let char_0 = func.insn_load_relative(&char_n_ptr, 0, &Context::ubyte_type());

        func.insn_return(&char_0);
        func.compile();
        func
    }
    fn gen_get_byte_from_vec<RuntimeT: Runtime>(
        ctx: &mut Context,
        _runtime: &mut RuntimeT,
    ) -> Function {
        let mut func = ctx
            .function(
                Abi::Cdecl,
                &Context::ubyte_type(),
                vec![Context::void_ptr_type(), Context::ulong_type()],
            )
            .unwrap();
        let ptr = func.arg(0).unwrap();
        let idx = func.arg(1).unwrap();

        let char_n_ptr = func.insn_load_elem_address(&ptr, &idx, &Context::ubyte_type());
        let char_0 = func.insn_load_relative(&char_n_ptr, 0, &Context::ubyte_type());

        func.insn_return(&char_0);
        func.compile();
        func
    }
}

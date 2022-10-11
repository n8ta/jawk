#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum Abi {
    Cdecl,
    VarArg,
    Stdcall,
    Fastcall,
}
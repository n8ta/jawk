pub use value::{ValuePtrT, ValueT};

mod value;
mod globals;
mod codegen_consts;
mod function_codegen;
mod function_scope;
mod callable_function;

use hashbrown::HashMap;
use crate::printable_error::PrintableError;
use crate::runtime::{ReleaseRuntime, Runtime, DebugRuntime};
use crate::{Symbolizer};
use gnu_libjit::{Abi, Context, Function, Value};
use crate::codegen::callable_function::CallableFunction;
use crate::codegen::function_codegen::{FunctionCodegen};
use crate::codegen::globals::Globals;
use crate::symbolizer::Symbol;
use crate::typing::{FunctionMap, ITypedFunction, TypedProgram};

/// ValueT is the jit values that make up a struct. It's not a tagged union
/// just a struct with only one other field being valid to read at a time based on the tag field.
///
/// ValueT {
///     tag: u8
///     float: f64
///     string: *mut c_void
/// }


pub const FLOAT_TAG: i8 = 0;
pub const STRING_TAG: i8 = 1;

// Entry point to run a program
pub fn compile_and_run(prog: TypedProgram, files: &[String], symbolizer: &mut Symbolizer) -> Result<(), PrintableError> {
    let context = Context::new();
    let mut runtime = ReleaseRuntime::new(&context, files.to_vec());
    let mut codegen = CodeGen::compile(&context, &mut runtime, symbolizer, prog, false, false)?;
    codegen.run();
    Ok(())
}

// Entry point to run and debug/test a program. Use the test runtime.
pub fn compile_and_capture(prog: TypedProgram, files: &[String], symbolizer: &mut Symbolizer, dump: bool) -> Result<DebugRuntime, PrintableError> {
    let context = Context::new();
    let mut test_runtime = DebugRuntime::new(&context, files.to_vec());
    {
        let mut codegen = CodeGen::compile(&context, &mut test_runtime, symbolizer, prog, true, dump)?;
        codegen.run();
    }
    assert_eq!(test_runtime.strings_in(), test_runtime.strings_out(), "LEFT strings in does not match RIGHT strings out. This program caused a memory leak.");
    Ok(test_runtime)
}

struct CodeGen<'a, RuntimeT: Runtime> {
    main: Function,
    context: &'a Context,
    runtime: &'a mut RuntimeT,
    symbolizer: &'a mut Symbolizer,
    globals: Globals,
    var_arg_scratch: Value,
    function_map: HashMap<Symbol, CallableFunction>,
}

impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    fn compile(
        context: &'a Context,
        runtime: &'a mut RuntimeT,
        symbolizer: &'a mut Symbolizer,
        prog: TypedProgram,
        debug_asserts: bool,
        dump: bool,
    ) -> Result<Self, PrintableError> {

        // Main gets created apart from normal function_codegen since it needs
        // to do some runtime setup.
        let mut main_function = context
            .function(Abi::Cdecl, &Context::int_type(), vec![])
            .expect("to create function");

        // Allocate global arrays
        let num_arrays = prog.global_analysis.global_arrays.len();
        runtime.allocate_arrays(num_arrays);

        // Setup heap space for global scalars
        let globals = Globals::new(prog.global_analysis, runtime, &mut main_function, symbolizer);

        // printf is a variadic function. Allocate a bunch of heap space for it's args
        // right now it could overflow. TODO: Fix overflow
        let var_arg_scratch = unsafe { libc::malloc(100 * 8) };
        let var_arg_scratch = main_function.create_void_ptr_constant(var_arg_scratch);

        let main_sym = symbolizer.get("main function");
        let mut function_map = HashMap::with_capacity(1);
        let main_function_typed = prog.functions.get_user_function(&main_sym).unwrap();
        function_map.insert(main_sym.clone(),
                            CallableFunction::main(
                                main_function.clone(),
                                main_function_typed));

        let mut codegen = CodeGen {
            main: main_function,
            context,
            runtime,
            symbolizer,
            globals,
            var_arg_scratch,
            function_map,
        };
        codegen.compile_inner(prog.functions, debug_asserts, dump, main_sym)?;
        Ok(codegen)
    }

    fn run(&mut self) {
        let function: extern "C" fn() -> i32 = self.main.to_closure();
        function();
    }

    fn compile_inner(&mut self,
                     functions: FunctionMap,
                     debug_asserts: bool,
                     dump: bool,
                     main_sym: Symbol) -> Result<(), PrintableError> {

        // Gen stubs for each function, main already created
        // We need to create a stub for each function before we compile the others to allow
        // functions call any other function regardless of order they are compiled in
        for (name, function) in functions.user_functions().iter() {
            if *name == main_sym { continue; };
            let callable = CallableFunction::new(&self.context, function.clone());
            self.function_map.insert(name.clone(), callable);
        }

        // Compile bodies of each function (including main)
        for (name, parser_func) in functions.user_functions().iter() {
            let callable_function = self.function_map.get(name).expect("func to exist");
            FunctionCodegen::build_function(callable_function.jit_function().clone(),
                                            parser_func,
                                            self.runtime,
                                            &self.function_map,
                                            &mut self.context,
                                            &mut self.globals,
                                            self.symbolizer,
                                            &self.var_arg_scratch,
                                            *name == main_sym,
                                            debug_asserts,
                                            dump,
            )?;
        }

        self.context.build_end();
        Ok(())
    }
}

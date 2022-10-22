pub use value::{ValuePtrT, ValueT};

mod value;
mod globals;
mod codegen_consts;
mod function_codegen;

use hashbrown::HashMap;
use crate::parser::{Program};
use crate::printable_error::PrintableError;
use crate::runtime::{LiveRuntime, Runtime, TestRuntime};
use crate::{AnalysisResults, Symbolizer};
use gnu_libjit::{Abi, Context, Function, Value};
use crate::codegen::function_codegen::FunctionCodegen;
use crate::codegen::globals::Globals;
use crate::symbolizer::Symbol;

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
pub fn compile_and_run(prog: Program, files: &[String], symbolizer: &mut Symbolizer) -> Result<(), PrintableError> {
    let mut runtime = LiveRuntime::new(files.to_vec());
    let mut codegen = CodeGen::compile(&mut runtime, symbolizer, prog, false, false)?;
    codegen.run();
    Ok(())
}

// Entry point to run and debug/test a program. Use the test runtime.
pub fn compile_and_capture(prog: Program, files: &[String], symbolizer: &mut Symbolizer, dump: bool) -> Result<TestRuntime, PrintableError> {
    let mut test_runtime = TestRuntime::new(files.to_vec());
    {
        let mut codegen = CodeGen::compile(&mut test_runtime, symbolizer, prog, true, dump)?;
        codegen.run();
    }
    assert_eq!(test_runtime.strings_in(), test_runtime.strings_out(), "LEFT strings in does not match RIGHT strings out. This program caused a memory leak.");
    Ok(test_runtime)
}

struct CodeGen<'a, RuntimeT: Runtime> {
    main: Function,
    context: Context,
    runtime: &'a mut RuntimeT,
    symbolizer: &'a mut Symbolizer,
    globals: Globals,
    var_arg_scratch: Value,
    function_map: HashMap<Symbol, Function>,
}

impl<'a, RuntimeT: Runtime> CodeGen<'a, RuntimeT> {
    fn compile(runtime: &'a mut RuntimeT,
               symbolizer: &'a mut Symbolizer,
               prog: Program,
               debug_asserts: bool,
               dump: bool

    ) -> Result<Self,PrintableError> {
        let mut context = Context::new();

        let mut main_function = context
            .function(Abi::Cdecl, &Context::int_type(), vec![])
            .expect("to create function");

        let globals = Globals::new(AnalysisResults::new(), runtime, &mut main_function, symbolizer);

        let var_arg_scratch = unsafe { libc::malloc(100 * 8) };
        let var_arg_scratch = main_function.create_void_ptr_constant(var_arg_scratch);

        let main_sym = symbolizer.get("main function");
        let mut function_map = HashMap::with_capacity(1);
        function_map.insert(main_sym.clone(), main_function.clone());

        let mut codegen = CodeGen {
            main: main_function,
            context,
            runtime,
            symbolizer,
            globals,
            var_arg_scratch,
            function_map,
        };
        codegen.compile_inner(prog, debug_asserts, dump, main_sym)?;
        Ok(codegen)
    }

    fn run(&mut self) {
        let function: extern "C" fn() -> i32 = self.main.to_closure();
        function();
    }


    fn compile_inner(&mut self, mut prog: Program, debug_asserts: bool, dump: bool, main_sym: Symbol) -> Result<(), PrintableError> {
        let num_arrays = prog.global_analysis.global_arrays.len();
        let mut global_analysis = AnalysisResults::new();
        std::mem::swap(&mut global_analysis, &mut prog.global_analysis);

        self.runtime.allocate_arrays(num_arrays);

        // Gen stubs for each function, main already created
        for (name, _func) in &prog.functions {
            if *name == main_sym { continue };
            let func = self.context.function(Abi::Cdecl, &Context::int_type(), vec![]).unwrap();
            self.function_map.insert(name.clone(), func);
        }

        {
            // Init globals in main function
            self.globals = Globals::new(global_analysis, self.runtime, &mut self.main, self.symbolizer);
        }

        for (name, func) in &prog.functions {
            let jit_func = self.function_map.get(name).expect("func to exist").clone();
            FunctionCodegen::build_function(jit_func,
                                            &func,
                                            self.runtime,
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

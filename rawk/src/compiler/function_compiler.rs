use std::rc::Rc;
use crate::awk_str::RcAwkStr;
use crate::lexer::{BinOp, LogicalOp, MathOp};
use crate::parser::{ArgT, Expr, LValue, ScalarType, Stmt, TypedExpr};
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::{AnalysisResults, BuiltinFunc, FunctionMap, ITypedFunction, TypedProgram, TypedUserFunction};
use crate::vm::{Code, Label, VmFunc};
use crate::compiler::chunk::Chunk;
use crate::stackt::StackT;


pub struct FunctionCompiler<'a> {
    chunk: Chunk,
    label_count: usize,
    typed_program: &'a mut TypedProgram,
    break_labels: Vec<Label>,
    parser_func: Rc<TypedUserFunction>,
}

impl<'a> FunctionCompiler<'a> {
    pub fn new(typed_program: &'a mut TypedProgram, parser_func: Rc<TypedUserFunction>) -> Self {
        Self {
            typed_program,
            chunk: Chunk::new(),
            label_count: 0,
            break_labels: vec![],
            parser_func,
        }
    }

    pub fn compile(mut self) -> Result<VmFunc, PrintableError> {
        let name = self.parser_func.name();
        let id = self.typed_program.functions.get_id(&name).unwrap();

        let cpy = self.parser_func.clone();
        let func = cpy.function();
        self.stmt(&func.body)?;

        // If function doesn't end with a user provided return return the empty string
        if !self.chunk.ends_with(&[Code::Ret]) {
            let strnum = RcAwkStr::new_bytes("".as_bytes().to_vec());
            self.add(Code::ConstStrNum { strnum });
            self.add(Code::StrToVar);
            self.add(Code::Ret);
        }

        self.chunk.optimize();
        self.chunk.resolve_labels();
        Ok(VmFunc::new(self.chunk, id, self.parser_func.clone()))
    }

    fn add(&mut self, code: Code) {
        self.chunk.push(code)
    }

    fn create_and_insert_lbl(&mut self) -> Label {
        let lbl = self.create_lbl();
        self.insert_lbl(lbl);
        lbl
    }

    fn create_lbl(&mut self) -> Label {
        let id = self.label_count;
        self.label_count += 1;
        Label::new(id)
    }

    fn insert_lbl(&mut self, label: Label) {
        self.chunk.push(Code::Label(label));
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Expr(expr) => {
                self.expr_opt(expr, None)?;
            }
            Stmt::Print(expr) => {
                self.expr(expr, StackT::Str)?;
                self.add(Code::Print);
            }
            Stmt::Group(grp) => {
                for elem in grp {
                    self.stmt(elem)?;
                }
            }
            Stmt::If(test, if_so, if_not) => {
                /*
                    Expression test
                    jmp_false :if_not
                    Stmt: if-body
                    jmp :done
                :if_not
                    Stmt: if-not-body
                :done
                */
                if let Some(if_not) = if_not {
                    let if_not_lbl = self.create_lbl();
                    let done_lbl = self.create_lbl();

                    self.expr(test, test.typ.into())?;
                    self.add(Code::jump_if_false(test.typ, &if_not_lbl));
                    self.stmt(if_so)?;
                    self.add(Code::JumpLbl(done_lbl));
                    self.insert_lbl(if_not_lbl);
                    self.stmt(if_not)?;
                    self.insert_lbl(done_lbl);
                } else {
                    // test
                    // JmpIfFalse :if_not
                    // [IfSo]
                    // Jmp :done
                    // :if_not
                    // [IfNot]
                    // :done

                    let if_not_lbl = self.create_lbl();
                    let done_lbl = self.create_lbl();
                    self.expr(test, test.typ.into())?;
                    self.add(Code::jump_if_false(test.typ, &if_not_lbl));
                    self.stmt(if_so)?;
                    self.add(Code::JumpLbl(done_lbl));
                    self.insert_lbl(if_not_lbl);
                    self.insert_lbl(done_lbl);
                }
            }
            Stmt::While(test, body) => {

                if test.expr == Expr::NextLine {
                    /*
                        JumpIfFalseNextLine
                        :body
                        [Body]
                        JumpIfTrueNextLine :body
                        :done
                    */
                    let body_lbl = self.create_lbl();
                    let done_lbl = self.create_lbl();

                    self.add(Code::JumpIfFalseNextLineLbl(done_lbl));
                    self.break_labels.push(done_lbl);
                    self.insert_lbl(body_lbl);
                    self.stmt(body)?;
                    self.add(Code::JumpIfTrueNextLineLbl(body_lbl));
                    self.insert_lbl(done_lbl);
                    self.break_labels.pop().unwrap();

                } else {
                    /*
                    [Test]
                    JumpIfFalse :done
                    :body
                    [Body]
                    [Test]
                    JumpIfTrue :body
                    :done

                    :test
                    [Test]
                    JumpIfFalse :done
                    :body
                    [Body]
                    jump :test
                    :done

                     */
                    let body_lbl = self.create_lbl();
                    let done_lbl = self.create_lbl();

                    let test_typ = test.typ;
                    self.expr(test, test_typ.into())?;
                    self.add(Code::jump_if_false(test_typ, &done_lbl));
                    self.break_labels.push(done_lbl);
                    self.insert_lbl(body_lbl);
                    self.stmt(body)?;
                    self.expr(test, test_typ.into())?;
                    self.add(Code::jump_if_true(test_typ, &body_lbl));
                    self.insert_lbl(done_lbl);
                    self.break_labels.pop().unwrap();
                }
            }
            Stmt::Printf { args, fstring } => {
                for arg in args {
                    self.expr(arg, StackT::Str)?;
                }
                self.expr(fstring, StackT::Str)?;
                self.add(Code::Printf { num_args: args.len() }); // TODO u16max
            }
            Stmt::Break => {
                if let Some(break_lbl) = self.break_labels.last() {
                    self.add(Code::JumpLbl(*break_lbl))
                } else {
                    return Err(PrintableError::new("Tried to break outside a loop".to_string()));
                }
            }
            Stmt::Return(ret) => {
                if let Some(ret) = ret {
                    self.expr(ret, StackT::Var)?;
                } else {
                    self.add(Code::ConstNum { num: 0.0 });
                    self.add(Code::NumToVar);
                }
                self.add(Code::Ret);
            }
        }
        Ok(())
    }

    fn expr(&mut self, expr: &TypedExpr, desired_stack: StackT) -> Result<Option<StackT>, PrintableError> {
        self.expr_opt(expr, Some(desired_stack))
    }

    // expr: AST Node
    // dest_stack: which stack the result will be left on
    // side_effect_only: skip pushing it onto the stack we will not use it
    fn expr_opt(&mut self, expr: &TypedExpr, desired_stack: Option<StackT>) -> Result<Option<StackT>, PrintableError> {
        let stack: Option<StackT> = match &expr.expr {
            Expr::ScalarAssign(scalar_name, value) => {
                self.expr(value, value.typ.into())?;
                let side_effect_only = desired_stack == None;
                self.assign_to_scalar(scalar_name, value.typ, side_effect_only);
                if side_effect_only { None } else { Some(value.typ.into()) }
            }
            Expr::NumberF64(num) => {
                self.add(Code::ConstNum { num: *num });
                Some(StackT::Num)
            }
            Expr::String(str) => {
                self.add(Code::ConstStr { str: str.clone() });
                Some(StackT::Str)
            }
            Expr::Regex(reg) => {
                self.add(Code::ConstStr { str: reg.clone() });
                Some(StackT::Str)
            }
            Expr::Concatenation(exprs) => {
                for expr in exprs.iter().rev() {
                    self.expr(expr, StackT::Str)?;
                }
                self.add(Code::Concat { count: exprs.len() });
                Some(StackT::Str)
            }
            Expr::BinOp(lhs, op, rhs) => {
                if lhs.typ == ScalarType::Num && rhs.typ == ScalarType::Num
                    && *op != BinOp::NotMatchedBy
                    && *op != BinOp::MatchedBy {
                    self.expr(lhs, StackT::Num)?;
                    self.expr(rhs, StackT::Num)?;
                    match op {
                        BinOp::Greater => self.add(Code::GtNum),
                        BinOp::GreaterEq => self.add(Code::GtEqNum),
                        BinOp::Less => self.add(Code::LtNum),
                        BinOp::LessEq => self.add(Code::LtEqNum),
                        BinOp::BangEq => self.add(Code::NeqNum),
                        BinOp::EqEq => self.add(Code::EqEqNum),
                        _ => panic!("not possible")
                    };
                } else {
                    let desired_stack = if *op == BinOp::NotMatchedBy || *op == BinOp::MatchedBy
                    { StackT::Str } else { StackT::Var };
                    self.expr(lhs, desired_stack)?;
                    self.expr(rhs, desired_stack)?;
                    match op {
                        BinOp::Greater => self.add(Code::Gt),
                        BinOp::GreaterEq => self.add(Code::GtEq),
                        BinOp::Less => self.add(Code::Lt),
                        BinOp::LessEq => self.add(Code::LtEq),
                        BinOp::BangEq => self.add(Code::Neq),
                        BinOp::EqEq => self.add(Code::EqEq),
                        BinOp::MatchedBy => self.add(Code::Matches),
                        BinOp::NotMatchedBy => self.add(Code::NMatches),
                    };
                }
                Some(StackT::Num)
            }
            Expr::MathOp(lhs, op, rhs) => {
                self.expr(lhs, StackT::Num)?;
                self.expr(rhs, StackT::Num)?;
                match op {
                    MathOp::Minus => self.add(Code::Minus),
                    MathOp::Plus => self.add(Code::Add),
                    MathOp::Slash => self.add(Code::Div),
                    MathOp::Star => self.add(Code::Mult),
                    MathOp::Modulus => self.add(Code::Mod),
                    MathOp::Exponent => self.add(Code::Exp),
                };
                Some(StackT::Num)
            }
            Expr::LogicalOp(lhs, op, rhs) => {
                match op {
                    LogicalOp::And => {
                        /*
                        [LHS]
                        JumpIfFalse :is_false
                        [RHS]
                        JumpIfFalse :is_false
                        One
                        Jump :done
                        :is_false
                        Zero
                        :done
                        */
                        let is_false = self.create_lbl();
                        let done = self.create_lbl();
                        self.expr(lhs, lhs.typ.into())?;
                        self.add(Code::jump_if_false(lhs.typ, &is_false));
                        self.expr(rhs, rhs.typ.into())?;
                        self.add(Code::jump_if_false(rhs.typ, &is_false));
                        self.add(Code::ConstNum { num: 1.0 });
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_false);
                        self.add(Code::ConstNum { num: 0.0 });
                        self.insert_lbl(done);
                        Some(StackT::Num)
                    }
                    LogicalOp::Or => {
                        /*
                        [LHS]
                        JumpIfTrue :is_true
                        [RHS]
                        JumpIfTrue :is_true
                        Zero
                        Jump :done
                        :is_true
                        FloatOne
                        :done
                        */
                        let done = self.create_lbl();
                        let is_true = self.create_lbl();

                        self.expr(lhs, lhs.typ.into())?;
                        self.add(Code::jump_if_true(lhs.typ, &is_true));
                        self.expr(rhs, rhs.typ.into())?;
                        self.add(Code::jump_if_true(rhs.typ, &is_true));
                        self.add(Code::ConstNum { num: 0.0 });
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_true);
                        self.add(Code::ConstNum { num: 1.0 });
                        self.insert_lbl(done);
                        Some(StackT::Num)
                    }
                }
            }
            Expr::Variable(scalar) => {
                if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar) {
                    // TODO: function args should be on non-unknown stacks
                    self.add(Code::arg_scl(ScalarType::Var, arg_idx));
                    Some(StackT::Var)
                } else if let Some(id) = self.typed_program.global_analysis.global_scalars.get(scalar) {
                    self.add(Code::gscl(*id, expr.typ));
                    Some(expr.typ.into())
                } else if let Some(arg_idx) = self.parser_func.array_arg_idx(scalar) {
                    self.add(Code::ArgArray { arg_idx: arg_idx }); // TODO: u16max
                    Some(StackT::Array)
                } else {
                    let id = self.typed_program.global_analysis.global_arrays.get(scalar).expect("compiler bug in typing pass can't find global array");
                    self.add(Code::GlobalArr(*id));
                    Some(StackT::Array)
                }
            }
            Expr::Column(col) => {
                self.expr(col, StackT::Num)?;
                self.add(Code::Column);
                Some(StackT::Str)
            }
            Expr::NextLine => {
                panic!("compiler bug: checking for next line should be handled within while");
            }
            Expr::Ternary(test, if_so, if_not) => {
                /*
                [Test]
                JumpIfFalse :is_false
                [IF_SO]
                Jump :done
                :is_false
                Pop
                [IF_NOT]
                :done
                 */
                let is_false = self.create_lbl();
                let done = self.create_lbl();

                self.expr(test, test.typ.into())?;
                self.add(Code::jump_if_false(test.typ, &is_false));
                self.expr(if_so, expr.typ.into())?;
                self.add(Code::JumpLbl(done));
                self.insert_lbl(is_false);
                self.expr(if_not, expr.typ.into())?;
                self.insert_lbl(done);
                Some(expr.typ.into())
            }
            Expr::ArrayAssign { name, indices, value } => {
                self.expr(value, value.typ.into())?;
                let side_effect_only = desired_stack == None;
                self.assign_to_array(name, indices, value.typ, side_effect_only)?;
                if side_effect_only { None } else { Some(value.typ.into()) }
            }
            Expr::ArrayIndex { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx, StackT::Str)?;
                };
                self.add(Code::ArrayIndex { indices: indices.len() }); // TODO: u16max
                Some(StackT::Var)
            }
            Expr::InArray { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx, StackT::Str)?;
                };
                self.add(Code::ArrayMember { indices: indices.len() }); // TODO: u16max
                Some(StackT::Num)
            }
            Expr::Call { target, args } => {
                // TODO: Arg # mismatch and implicit array creation

                if let Some(builtin) = BuiltinFunc::get(target.to_str()) {
                    let t = self.builtin(builtin, args)?;
                    Some(t.into())
                } else if let Some(target_func) = self.typed_program.functions.get(target) {
                    let id = self.typed_program.functions.get_id(&target_func.name()).unwrap();
                    let target_name = target_func.name();
                    for (idx, (function_arg, call_arg)) in target_func.args().iter().zip(args).enumerate() {
                        match function_arg.typ {
                            ArgT::Scalar => {
                                self.expr(call_arg, StackT::Var)?;
                            }
                            ArgT::Array => {
                                if let Expr::Variable(sym) = &call_arg.expr {
                                    self.push_array(sym);
                                } else {
                                    return Err(PrintableError::new(format!("Tried to use scalar as arg #{} to function {} which accepts an array", idx + 1, &target_name)));
                                }
                            }
                            ArgT::Unknown => {
                                self.expr(call_arg, StackT::Var)?; // Compile for side effects only
                                self.add(Code::Pop); // And then pop result
                            }
                        }
                    }
                    self.add(Code::Call { target: id });
                    Some(StackT::Var)
                } else {
                    return Err(PrintableError::new(format!("Attempted to call unknown function: `{}`", target)));
                }
            }
            Expr::CallSub {
                ere,
                replacement,
                string,
                global
            } => {
                self.expr(ere, StackT::Str)?;
                self.expr(replacement, StackT::Str)?;

                let string_expr: Expr = string.clone().into(); // TODO: No clone
                let typed_str_expr = TypedExpr::new(string_expr);
                self.expr(&typed_str_expr, StackT::Str)?;

                // Stack: [ere, repl, string]
                self.add(Code::Sub3 { global: if *global { true } else { false } });
                // Pushes String with subs and the number of subs

                // Stack: [result]
                match string {
                    LValue::Variable(name) => {
                        self.assign_to_scalar(name, ScalarType::Str, true);
                    }
                    LValue::ArrayIndex { name, indices } => {
                        self.assign_to_array(name, indices, ScalarType::Str, true)?;
                    }
                    LValue::Column(_col) => todo!("column assignment"),
                }
                Some(StackT::Num)
            }
        };

        match (stack, desired_stack) {
            (Some(stack), Some(desired_stack)) => {
                if desired_stack == stack {
                    return Ok(Some(desired_stack));
                }
                let stack = if let Ok(scalar_src) = stack.try_into() { scalar_src } else { panic!("cannot convert array to scalar") };
                let desired = if let Ok(desired) = desired_stack.try_into() { desired } else { panic!("cannot convert array to scalar") };
                self.add(Code::move_stack_to_stack(stack, desired));
            }
            (None, Some(_desired_stack)) => {
                panic!("compiler bug")
            }
            (Some(stack), None) => {
                if let Ok(scalar_src) = stack.try_into() {
                    self.add(Code::pop(scalar_src));
                } else {
                    panic!("cannot have extra array to pop")
                }
            }
            (None, None) => {}
        }

        Ok(desired_stack)
    }

    // Value to assign should be top of the stack unless side_effect_only==true
    fn assign_to_scalar(&mut self, scalar_name: &Symbol, typ: ScalarType, side_effect_only: bool) {
        // TODO: u16max
        let code = if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar_name) {
            Code::arg_scl_assign(side_effect_only, typ, arg_idx) // todo u16
        } else {
            let id = self.typed_program.global_analysis.global_scalars.get(scalar_name).expect("compiler bug in typing pass global scalar not found");
            Code::gscl_assign(side_effect_only, typ, *id)
        };
        self.add(code);
    }

    // Value to assign should be top of the stack
    fn assign_to_array(&mut self,
                       name: &Symbol,
                       indices: &[TypedExpr],
                       result_type: ScalarType,
                       side_effect_only: bool) -> Result<(), PrintableError> {
        self.push_array(name);
        for idx in indices {
            self.expr(idx, StackT::Str)?;
        };
        self.add(Code::array_assign(indices.len(), result_type, side_effect_only));
        Ok(())
    }

    fn builtin(&mut self, builtin: BuiltinFunc, args: &Vec<TypedExpr>) -> Result<ScalarType, PrintableError> {
        let code = match builtin {
            BuiltinFunc::Atan2 => Code::BuiltinAtan2,
            BuiltinFunc::Cos => Code::BuiltinCos,
            BuiltinFunc::Exp => Code::BuiltinExp,
            BuiltinFunc::Substr => {
                if args.len() == 2 {
                    Code::BuiltinSubstr2
                } else {
                    Code::BuiltinSubstr3
                }
            }
            BuiltinFunc::Index => Code::BuiltinIndex,
            BuiltinFunc::Int => Code::BuiltinInt,
            BuiltinFunc::Length => {
                if args.len() == 0 {
                    Code::BuiltinLength0
                } else {
                    Code::BuiltinLength1
                }
            }
            BuiltinFunc::Log => Code::BuiltinLog,
            BuiltinFunc::Rand => Code::BuiltinRand,
            BuiltinFunc::Sin => Code::BuiltinSin,
            BuiltinFunc::Split => {
                if args.len() == 2 {
                    Code::BuiltinSplit2
                } else {
                    Code::BuiltinSplit3
                }
            }
            BuiltinFunc::Sqrt => Code::BuiltinSqrt,
            BuiltinFunc::Srand => {
                if args.len() == 0 {
                    Code::BuiltinSrand0
                } else {
                    Code::BuiltinSrand1
                }
            }
            BuiltinFunc::Tolower => Code::BuiltinTolower,
            BuiltinFunc::Toupper => Code::BuiltinToupper,
            BuiltinFunc::System => todo!("builtin System"),
            BuiltinFunc::Sprintf => todo!("builtin Sprintf"),
            BuiltinFunc::Close => todo!("builtin Close"),
            BuiltinFunc::Matches => todo!("builtin Matches"),
        };
        let meta = code.meta(&self.typed_program.functions);
        for (idx, arg) in meta.args().iter().enumerate() {
            self.expr(&args[idx], *arg)?;
        }
        self.add(code);
        Ok(meta.returns().single_scalar_return_value())
    }

    fn push_array(&mut self, name: &Symbol) {
        if let Some(arg_idx) = self.parser_func.array_arg_idx(name) {
            self.add(Code::ArgArray { arg_idx }); // TODO: u16max
        } else {
            let id = self.typed_program.global_analysis.global_arrays.get(name).expect("compiler bug in typing pass global array not found");
            self.add(Code::GlobalArr(*id));
        }
    }
}
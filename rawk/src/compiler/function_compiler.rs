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
    label_count: u16,
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
            if !self.parser_func.is_main() {
                let idx = self.chunk.add_const_strnum(RcAwkStr::new_bytes("".as_bytes().to_vec()));
                self.add(Code::ConstLkpNum { idx });
            }
            self.add(Code::Ret);
        }

        self.chunk.resolve_labels();
        Ok(VmFunc::new(self.chunk, id as u16, self.parser_func.clone()))
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
                self.expr(expr, StackT::Var, true)?;
            },
            Stmt::Print(expr) => {
                self.expr(expr, StackT::Str, false)?;
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

                    self.expr(test, StackT::Num, false)?;
                    self.add(Code::JumpIfFalseLbl(if_not_lbl));

                    self.stmt(if_so)?;
                    self.add(Code::JumpLbl(done_lbl));
                    self.insert_lbl(if_not_lbl);
                    self.stmt(if_not)?;
                    self.insert_lbl(done_lbl);
                } else {
                    self.expr(test, StackT::Num, false)?;
                    let if_not_lbl = self.create_lbl();
                    self.add(Code::JumpIfFalseLbl(if_not_lbl));
                    self.stmt(if_so)?;
                    self.insert_lbl(if_not_lbl)
                }
            }
            Stmt::While(test, body) => {
                /*
                :Test
                [Test]
                JumpIfFalse :Done
                Pop
                [Body]
                Jump :Test
                :Done
                Pop
                :BreakToHere
                 */
                let test_lbl = self.create_and_insert_lbl();
                let done_lbl = self.create_lbl();
                let break_lbl = self.create_lbl();
                self.expr(test, StackT::Num, false)?;

                self.break_labels.push(break_lbl);
                self.add(Code::JumpIfFalseLbl(done_lbl));
                self.add(Code::PopNum);
                self.stmt(body)?;
                self.add(Code::JumpLbl(test_lbl));
                self.insert_lbl(done_lbl);
                self.add(Code::PopNum);
                self.insert_lbl(break_lbl);
                self.break_labels.pop().unwrap();
            }
            Stmt::Printf { args, fstring } => {
                for arg in args {
                    self.expr(arg, StackT::Str, false)?;
                }
                self.expr(fstring, StackT::Str, false)?;
                self.add(Code::Printf { num_args: args.len() as u16 }); // TODO u16max
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
                    self.expr(ret, StackT::Var, false)?;
                } else {
                    self.add(Code::FloatZero);
                    self.add(Code::NumToVar);
                }
                self.add(Code::Ret);
            }
        }
        Ok(())
    }

    // expr: AST Node
    // dest_stack: which stack the result will be left on
    // side_effect_only: skip pushing it onto the stack we will not use it
    fn expr(&mut self, expr: &TypedExpr, desired_stack: StackT, side_effect_only: bool) -> Result<StackT, PrintableError> {
        let stack: StackT = match &expr.expr {
            Expr::ScalarAssign(scalar_name, value) => {
                self.expr(value, value.typ.into(), false)?;
                self.assign_to_scalar(scalar_name, value.typ, side_effect_only);
                value.typ.into()
            }
            Expr::NumberF64(num) => {
                let idx = self.chunk.add_const_float(*num);
                self.add(Code::ConstLkpNum { idx });
                StackT::Num
            }
            Expr::String(str) => {
                let idx = self.chunk.add_const_str(str.clone());
                self.add(Code::ConstLkpStr { idx });
                StackT::Str
            }
            Expr::Regex(reg) => {
                let idx = self.chunk.add_const_str(reg.clone());
                self.add(Code::ConstLkpStr { idx });
                StackT::Str
            }
            Expr::Concatenation(exprs) => {
                for expr in exprs.iter().rev() {
                    self.expr(expr, StackT::Str, false)?;
                }
                self.add(Code::Concat { count: exprs.len() as u16 });
                StackT::Str
            }
            Expr::BinOp(lhs, op, rhs) => {
                self.expr(lhs, StackT::Var, false)?;
                self.expr(rhs, StackT::Var, false)?;
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
                StackT::Num
            }
            Expr::MathOp(lhs, op, rhs) => {
                self.expr(lhs, StackT::Num, false)?;
                self.expr(rhs, StackT::Num, false)?;
                match op {
                    MathOp::Minus => self.add(Code::Minus),
                    MathOp::Plus => self.add(Code::Add),
                    MathOp::Slash => self.add(Code::Div),
                    MathOp::Star => self.add(Code::Mult),
                    MathOp::Modulus => self.add(Code::Mod),
                    MathOp::Exponent => self.add(Code::Exp),
                };
                StackT::Num
            }
            Expr::LogicalOp(lhs, op, rhs) => {
                self.expr(lhs, StackT::Var, false)?;
                match op {
                    LogicalOp::And => {
                        /*
                        [LHS]
                        JumpIfFalse :is_false
                        Pop
                        [RHS]
                        JumpIfFalse :is_false
                        Pop
                        One
                        Jump :done
                        :is_false
                        Pop
                        Zero
                        :done
                        */
                        let is_false = self.create_lbl();
                        let done = self.create_lbl();

                        self.add(Code::JumpIfFalseLbl(is_false));
                        self.add(Code::Pop); // Pop lhs
                        self.expr(rhs, StackT::Var, false)?;
                        self.add(Code::JumpIfFalseLbl(is_false));
                        self.add(Code::Pop); // Pop rhs
                        self.add(Code::FloatOne);
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_false);
                        self.add(Code::Pop);
                        self.add(Code::FloatZero);
                        self.insert_lbl(done);
                        StackT::Num
                    }
                    LogicalOp::Or => {
                        /*
                        [LHS]
                        JumpIfTrue :is_true
                        Pop
                        [RHS]
                        JumpIfTrue :is_true
                        Pop
                        Zero
                        Jump :done
                        :is_true
                        Pop
                        FloatOne
                        :done

                        */
                        let done = self.create_lbl();
                        let is_true = self.create_lbl();

                        self.expr(lhs, StackT::Var, false)?;
                        self.add(Code::JumpIfTrueLbl(is_true));
                        self.add(Code::Pop);
                        self.expr(rhs, StackT::Var, false)?;
                        self.add(Code::JumpIfTrueLbl(is_true));
                        self.add(Code::Pop);
                        self.add(Code::FloatZero);
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_true);
                        self.add(Code::Pop);
                        self.add(Code::FloatOne);
                        self.insert_lbl(done);
                        StackT::Num
                    }
                }
            }
            Expr::Variable(scalar) => {
                if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar) {
                    // TODO: function args should be on non-unknown stacks
                    self.add(Code::arg_scl(ScalarType::Var, arg_idx as u16));
                    StackT::Var
                } else if let Some(id) = self.typed_program.global_analysis.global_scalars.get(scalar) {
                    self.add(Code::gscl(*id, expr.typ));
                    expr.typ.into()
                } else if let Some(arg_idx) = self.parser_func.array_arg_idx(scalar) {
                    self.add(Code::ArgArray { arg_idx: arg_idx as u16 }); // TODO: u16max
                    StackT::Array
                } else {
                    let id = self.typed_program.global_analysis.global_arrays.get(scalar).expect("compiler bug in typing pass can't find global array");
                    self.add(Code::GlobalArr(*id));
                    StackT::Array
                }
            }
            Expr::Column(col) => {
                self.expr(col, StackT::Num, false)?;
                self.add(Code::Column);
                StackT::Str
            }
            Expr::NextLine => {
                self.add(Code::NextLine);
                StackT::Num
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

                self.expr(test, StackT::Num, false)?;
                self.add(Code::JumpIfFalseLbl(is_false));
                self.add(Code::PopNum);
                self.expr(if_so, expr.typ.into(), false)?;
                self.add(Code::JumpLbl(done));
                self.insert_lbl(is_false);
                self.add(Code::PopNum);
                self.expr(if_not, expr.typ.into(), false)?;
                self.insert_lbl(done);
                expr.typ.into()
            }
            Expr::ArrayAssign { name, indices, value } => {
                self.expr(value, value.typ.into(), false)?;
                self.assign_to_array(name, indices, value.typ, side_effect_only)?;
                value.typ.into()
            }
            Expr::ArrayIndex { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx, StackT::Str, false)?;
                };
                self.add(Code::ArrayIndex { indices: indices.len() as u16 }); // TODO: u16max
                StackT::Var
            }
            Expr::InArray { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx, StackT::Str, false)?;
                };
                self.add(Code::ArrayMember { indices: indices.len() as u16 }); // TODO: u16max
                StackT::Num
            }
            Expr::Call { target, args } => {
                // TODO: Arg # mismatch and implicit array creation

                if let Some(builtin) = BuiltinFunc::get(target.to_str()) {
                    let t = self.builtin(builtin, args)?;
                    t.into()
                } else if let Some(target_func) = self.typed_program.functions.get(target) {
                    let id = self.typed_program.functions.get_id(&target_func.name()).unwrap();
                    let target_name = target_func.name();
                    for (idx, (function_arg, call_arg)) in target_func.args().iter().zip(args).enumerate() {
                        match function_arg.typ {
                            ArgT::Scalar => {
                                self.expr(call_arg, StackT::Var, false)?;
                            }
                            ArgT::Array => {
                                if let Expr::Variable(sym) = &call_arg.expr {
                                    self.push_array(sym);
                                } else {
                                    return Err(PrintableError::new(format!("Tried to use scalar as arg #{} to function {} which accepts an array", idx + 1, &target_name)));
                                }
                            }
                            ArgT::Unknown => {
                                self.expr(call_arg, StackT::Var, false)?; // Compile for side effects only
                                self.add(Code::Pop); // And then pop result
                            }
                        }
                    }
                    self.add(Code::Call { target: id as u16 });
                    StackT::Var
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
                self.expr(ere, StackT::Str,false)?;
                self.expr(replacement, StackT::Str, false)?;

                let string_expr: Expr = string.clone().into(); // TODO: No clone
                let typed_str_expr = TypedExpr::new(string_expr);
                self.expr(&typed_str_expr, StackT::Str, false)?;

                // Stack: [ere, repl, string]
                self.add(Code::Sub3 { global: if *global { true } else { false } });

                // Stack: [result]
                match string {
                    LValue::Variable(name) => {
                        self.assign_to_scalar(name, ScalarType::Num, true);
                    }
                    LValue::ArrayIndex { name, indices } => {
                        self.assign_to_array(name, indices, ScalarType::Num, true)?;
                    }
                    LValue::Column(_col) => todo!("column assignment"),
                }
                StackT::Num
            }
        };

        if stack != desired_stack && !side_effect_only {
            if let Ok(scalar_src) = stack.try_into() {
                if let Ok(scalar_dest) = desired_stack.try_into() {
                    self.add(Code::move_stack_to_stack(scalar_src, scalar_dest));
                    return Ok(desired_stack.into())
                }
            }
            panic!("Cannot convert array into other types ")
        }
        Ok(stack)
    }

    // Value to assign should be top of the stack unless side_effect_only==true
    fn assign_to_scalar(&mut self, scalar_name: &Symbol, typ: ScalarType, side_effect_only: bool) -> ScalarType {
        // TODO: u16max
        let code = if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar_name) {
            Code::arg_scl_assign(side_effect_only, typ, arg_idx as u16) // todo u16
        } else {
            let id = self.typed_program.global_analysis.global_scalars.get(scalar_name).expect("compiler bug in typing pass global scalar not found");
            Code::gscl_assign(side_effect_only, typ, *id)
        };
        self.add(code);
        typ
    }

    // Value to assign should be top of the stack
    fn assign_to_array(&mut self,
                       name: &Symbol,
                       indices: &[TypedExpr],
                       result_type: ScalarType,
                       side_effect_only: bool) -> Result<(), PrintableError> {
        self.push_array(name);
        for idx in indices {
            self.expr(idx, StackT::Str, false)?;
        };
        self.add(Code::array_assign(indices.len() as u16, result_type, side_effect_only));
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
        Ok(meta.returns().single_scalar_return_value())
    }

    fn push_array(&mut self, name: &Symbol) {
        if let Some(arg_idx) = self.parser_func.array_arg_idx(name) {
            self.add(Code::ArgArray { arg_idx: arg_idx as u16 }); // TODO: u16max
        } else {
            let id = self.typed_program.global_analysis.global_arrays.get(name).expect("compiler bug in typing pass global array not found");
            self.add(Code::GlobalArr(*id));
        }
    }
}
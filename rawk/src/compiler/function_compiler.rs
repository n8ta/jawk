use std::rc::Rc;
use crate::awk_str::RcAwkStr;
use crate::compiler::FunctionIdMap;
use crate::lexer::{BinOp, LogicalOp, MathOp};
use crate::parser::{ArgT, Expr, LValue, Stmt, TypedExpr};
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::{AnalysisResults, BuiltinFunc, ITypedFunction, TypedUserFunction};
use crate::vm::{Code, Label, VmFunc};
use crate::compiler::chunk::Chunk;


// Jump offsets are often calculated after bytecode is emitted. This value is used temporarily
// and then overwritten with the actual offset
const JUMP_SENTINEL: i16 = 0917;

pub struct FunctionCompiler<'a> {
    chunk: Chunk,
    label_count: u16,
    mapping: &'a FunctionIdMap,
    type_analysis: &'a AnalysisResults,
    break_labels: Vec<Label>,
    parser_func: Rc<TypedUserFunction>,
}

impl<'a> FunctionCompiler<'a> {
    pub fn new(mapping: &'a FunctionIdMap, type_analysis: &'a AnalysisResults, parser_func: Rc<TypedUserFunction>) -> Self {
        Self {
            mapping,
            type_analysis,
            chunk: Chunk::new(),
            label_count: 0,
            break_labels: vec![],
            parser_func,
        }
    }

    pub fn compile(mut self) -> Result<VmFunc, PrintableError> {
        let name = self.parser_func.name();
        let id = self.mapping.get(&name).unwrap().0;

        let cpy = self.parser_func.clone();
        let func = cpy.function();
        self.stmt(&func.body)?;

        // If function doesn't end with a user provided return return the empty string
        if !self.chunk.ends_with(&[Code::Ret]) {
            let idx = self.chunk.add_const_strnum(RcAwkStr::new_bytes("".as_bytes().to_vec()));
            self.add(Code::ConstLkp { idx });
            self.add(Code::Ret);
        }

        self.chunk.resolve_labels();
        Ok(VmFunc::new(self.chunk, id, self.parser_func.clone()))
    }

    fn add(&mut self, code: Code) {
        self.chunk.push(code);
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
            Stmt::Expr(expr) => self.expr(expr)?,
            Stmt::Print(expr) => {
                self.expr(expr)?;
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

                    self.expr(test)?;
                    self.add(Code::JumpIfFalseLbl(if_not_lbl));

                    self.stmt(if_so)?;
                    self.add(Code::JumpLbl(done_lbl));
                    self.insert_lbl(if_not_lbl);
                    self.stmt(if_not)?;
                    self.insert_lbl(done_lbl);
                } else {
                    self.expr(test)?;
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
                self.expr(test)?;

                self.break_labels.push(break_lbl);
                self.add(Code::JumpIfFalseLbl(done_lbl));
                self.add(Code::Pop);
                self.stmt(body)?;
                self.add(Code::JumpLbl(test_lbl));
                self.insert_lbl(done_lbl);
                self.add(Code::Pop);
                self.insert_lbl(break_lbl);
                self.break_labels.pop().unwrap();
            }
            Stmt::Printf { args, fstring } => {
                for arg in args {
                    self.expr(arg)?;
                }
                self.expr(fstring)?;
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
                    self.expr(ret)?;
                } else {
                    self.add(Code::FloatZero)
                }
                self.add(Code::Ret);
            }
        }
        Ok(())
    }

    // Value to assign should be top of the stack
    fn assign_to_scalar(&mut self, scalar_name: &Symbol) {
        if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar_name) {
            self.add(Code::ArgSclAsgn { arg_idx: arg_idx as u16 }); // TODO: u16max
        } else {
            let id = self.type_analysis.global_scalars.get(scalar_name).expect("compiler bug in typing pass global scalar not found");
            self.add(Code::GSclAssign(*id));
        };
    }

    // Value to assign should be top of the stack
    fn assign_to_array(&mut self, name: &Symbol, indices: &[TypedExpr]) -> Result<(), PrintableError>{
        self.push_array(name);
        for idx in indices {
            self.expr(idx)?;
        };
        // TODO: u16max
        self.add(Code::ArrayAssign { indices: indices.len() as u16 });
        Ok(())
    }

    pub fn expr(&mut self, expr: &TypedExpr) -> Result<(), PrintableError> {
        match &expr.expr {
            Expr::ScalarAssign(scalar_name, value) => {
                self.expr(value)?;
                self.assign_to_scalar(scalar_name);
            }
            Expr::NumberF64(num) => {
                let idx = self.chunk.add_const_float(*num);
                self.add(Code::ConstLkp { idx });
            }
            Expr::String(str) => {
                let idx = self.chunk.add_const_str(str.clone());
                self.add(Code::ConstLkp { idx });
            }
            Expr::Regex(reg) => {
                let idx = self.chunk.add_const_str(reg.clone());
                self.add(Code::ConstLkp { idx });
            }
            Expr::Concatenation(exprs) => {
                for expr in exprs.iter().rev() {
                    self.expr(expr)?;
                }
                self.add(Code::Concat { count: exprs.len() as u16 });
            }
            Expr::BinOp(lhs, op, rhs) => {
                self.expr(lhs)?;
                self.expr(rhs)?;
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
            Expr::MathOp(lhs, op, rhs) => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                match op {
                    MathOp::Minus => self.add(Code::Minus),
                    MathOp::Plus => self.add(Code::Add),
                    MathOp::Slash => self.add(Code::Div),
                    MathOp::Star => self.add(Code::Mult),
                    MathOp::Modulus => self.add(Code::Mod),
                    MathOp::Exponent => self.add(Code::Exp),
                };
            }
            Expr::LogicalOp(lhs, op, rhs) => {
                self.expr(lhs)?;
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
                        self.expr(rhs)?;
                        self.add(Code::JumpIfFalseLbl(is_false));
                        self.add(Code::Pop); // Pop rhs
                        self.add(Code::FloatOne);
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_false);
                        self.add(Code::Pop);
                        self.add(Code::FloatZero);
                        self.insert_lbl(done);
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

                        self.add(Code::JumpIfTrueLbl(is_true));
                        self.add(Code::Pop);
                        self.expr(rhs)?;
                        self.add(Code::JumpIfTrueLbl(is_true));
                        self.add(Code::Pop);
                        self.add(Code::FloatZero);
                        self.add(Code::JumpLbl(done));
                        self.insert_lbl(is_true);
                        self.add(Code::Pop);
                        self.add(Code::FloatOne);
                        self.insert_lbl(done);
                    }
                }
            }
            Expr::Variable(scalar) => {
                if let Some(arg_idx) = self.parser_func.scalar_arg_idx(scalar) {
                    self.add(Code::ArgScl { arg_idx: arg_idx as u16 }); // TODO: u16max
                } else if let Some(id) = self.type_analysis.global_scalars.get(scalar) {
                    self.add(Code::GScl(*id));
                } else if let Some(arg_idx) = self.parser_func.array_arg_idx(scalar) {
                    self.add(Code::ArgArray { arg_idx: arg_idx as u16 }); // TODO: u16max
                } else {
                    let id = self.type_analysis.global_arrays.get(scalar).expect("compiler bug in typing pass can't find global array");
                    self.add(Code::GlobalArr(*id));
                }
            }
            Expr::Column(col) => {
                self.expr(col)?;
                self.add(Code::Column);
            }
            Expr::NextLine => {
                self.add(Code::NextLine);
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

                self.expr(test)?;
                self.add(Code::JumpIfFalseLbl(is_false));
                self.expr(if_so)?;
                self.add(Code::JumpLbl(done));
                self.insert_lbl(is_false);
                self.add(Code::Pop);
                self.expr(if_not)?;
                self.insert_lbl(done);
            }
            Expr::ArrayAssign { name, indices, value } => {
                self.expr(value)?;
                self.assign_to_array(name, indices)?;
            }
            Expr::ArrayIndex { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx)?;
                };
                self.add(Code::ArrayIndex { indices: indices.len() as u16 }); // TODO: u16max
            }
            Expr::InArray { name, indices } => {
                self.push_array(name);
                for idx in indices {
                    self.expr(idx)?;
                };
                self.add(Code::ArrayMember { indices: indices.len() as u16 }); // TODO: u16max
            }
            Expr::Call { target, args } => {
                // TODO: Arg # mismatch and implicit array creation

                if let Some(builtin) = BuiltinFunc::get(target.to_str()) {
                    return self.builtin(builtin, args);
                } else if let Some((id, target_func)) = self.mapping.get(target) {
                    let target_name = target_func.name();
                    for (idx, (function_arg, call_arg)) in target_func.args().iter().zip(args).enumerate() {
                        match function_arg.typ {
                            ArgT::Scalar => {
                                self.expr(call_arg)?;
                            }
                            ArgT::Array => {
                                if let Expr::Variable(sym) = &call_arg.expr {
                                    self.push_array(sym);
                                } else {
                                    return Err(PrintableError::new(format!("Tried to use scalar as arg #{} to function {} which accepts an array", idx + 1, &target_name)));
                                }
                            }
                            ArgT::Unknown => {
                                self.expr(call_arg)?; // Compile for side effects only
                                self.add(Code::Pop); // And then pop result
                            }
                        }
                    }
                    self.add(Code::Call { target: *id });
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
                self.expr(ere)?;
                self.expr(replacement)?;

                let string_expr: Expr = string.clone().into(); // TODO: No clone
                let typed_str_expr = TypedExpr::new(string_expr);
                self.expr(&typed_str_expr)?;

                // Stack: [ere, repl, string]
                self.add(Code::Sub { global: if *global { true } else { false } });

                // Stack: [result]
                match string {
                    LValue::Variable(name) => {
                        self.assign_to_scalar(name);
                    },
                    LValue::ArrayIndex { name, indices } => self.assign_to_array(name, indices)?,
                    LValue::Column(_col) => todo!("column assignment"),
                }
                // Assignment pushes the old value to the stack remove it.
                self.add(Code::Pop);
            }
        };
        Ok(())
    }

    fn builtin(&mut self, builtin: BuiltinFunc, args: &Vec<TypedExpr>) -> Result<(), PrintableError> {
        // TODO: Handle when there are too many args
        for arg in args {
            self.expr(arg)?;
        }
        match builtin {
            BuiltinFunc::Atan2 => self.add(Code::BuiltinAtan2),
            BuiltinFunc::Cos => self.add(Code::BuiltinCos),
            BuiltinFunc::Exp => self.add(Code::BuiltinExp),
            BuiltinFunc::Substr => {
                if args.len() == 2 {
                    self.add(Code::BuiltinSubstr2)
                } else {
                    self.add(Code::BuiltinSubstr3)
                }
            }
            BuiltinFunc::Index => self.add(Code::BuiltinIndex),
            BuiltinFunc::Int => self.add(Code::BuiltinInt),
            BuiltinFunc::Length => {
                if args.len() == 0 {
                    self.add(Code::BuiltinLength0)
                } else {
                    self.add(Code::BuiltinLength1)
                };
            }
            BuiltinFunc::Log => self.add(Code::BuiltinLog),
            BuiltinFunc::Rand => self.add(Code::BuiltinRand),
            BuiltinFunc::Sin => self.add(Code::BuiltinSin),
            BuiltinFunc::Split => {
                if args.len() == 2 {
                    self.add(Code::BuiltinSplit2);
                } else {
                    self.add(Code::BuiltinSplit3);
                }
            }
            BuiltinFunc::Sqrt => self.add(Code::BuiltinSqrt),
            BuiltinFunc::Srand => {
                if args.len() == 0 {
                    self.add(Code::BuiltinSrand0)
                } else {
                    self.add(Code::BuiltinSrand1)
                }
            }
            BuiltinFunc::Tolower => self.add(Code::BuiltinTolower),
            BuiltinFunc::Toupper => self.add(Code::BuiltinToupper),

            BuiltinFunc::System => todo!("builtin System"),
            BuiltinFunc::Sprintf => todo!("builtin Sprintf"),
            BuiltinFunc::Close => todo!("builtin Close"),
            BuiltinFunc::Matches => todo!("builtin Matches"),
        }
        Ok(())
    }

    fn push_array(&mut self, name: &Symbol) {
        if let Some(arg_idx) = self.parser_func.array_arg_idx(name) {
            self.add(Code::ArgArray { arg_idx: arg_idx as u16 }); // TODO: u16max
        } else {
            let id = self.type_analysis.global_arrays.get(name).expect("compiler bug in typing pass global array not found");
            self.add(Code::GlobalArr(*id));
        }
    }
}
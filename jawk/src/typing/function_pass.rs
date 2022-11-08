use hashbrown::{HashMap, HashSet};
use crate::parser::{Arg, ArgT, Program, ScalarType, Stmt, TypedExpr};
use crate::{Expr, PrintableError};
use crate::global_scalars::SymbolMapping;
use crate::symbolizer::Symbol;
use crate::typing::types::{AnalysisResults, Call, CallArg, MapT, TypedFunc, TypedProgram};

pub struct FunctionAnalysis {
    global_scalars: MapT,
    global_arrays: SymbolMapping,
    func_names: HashSet<Symbol>,
    str_consts: HashSet<Symbol>,
}

impl FunctionAnalysis {
    pub fn new() -> Self {
        Self {
            global_scalars: MapT::new(),
            global_arrays: SymbolMapping::new(),
            func_names: Default::default(),
            str_consts: Default::default(),
        }
    }
}

fn get_arg<'a>(func_state: &'a mut FuncState, name: &Symbol) -> Option<&'a mut Arg> {
    if let Some(arg) = func_state.args.iter_mut().find(|a| a.name == *name) {
        Some(arg)
    } else {
        None
    }
}

struct FuncState<'a> {
    args: &'a mut [Arg],
    calls: Vec<Call>,
    globals_used: &'a mut HashSet<Symbol>,
}

impl FunctionAnalysis {
    pub fn analyze_program(mut self, prog: Program) -> Result<TypedProgram, PrintableError> {
        for (name, _) in &prog.functions {
            self.func_names.insert(name.clone());
        }

        let mut functions = HashMap::new();
        for (name, mut func) in prog.functions {
            let calls = {
                let mut fstate = FuncState { args: &mut func.args, calls: vec![], globals_used: &mut func.globals_used };
                self.analyze_stmt(&mut func.body, &mut fstate)?;
                fstate.calls
            };
            functions.insert(name, TypedFunc::new(func, calls));
        }

        let mut global_scalars = SymbolMapping::new();
        for (scalar, _) in self.global_scalars.into_iter() {
            global_scalars.insert(scalar)
        }
        let results = AnalysisResults {
            global_scalars,
            str_consts: self.str_consts,
            global_arrays: self.global_arrays,
        };

        Ok(TypedProgram::new(functions, results))
    }
    fn use_as_scalar(&mut self, var: &Symbol, typ: ScalarType, func_state: &mut FuncState) -> Result<(), PrintableError> {
        if let Some(arg) = get_arg(func_state, var) {
            if let Some(arg_typ) = arg.typ {
                match arg_typ {
                    ArgT::Scalar => {} // scalar arg used as scalar, lgtm
                    ArgT::Array => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var)))
                }
            } else {
                arg.typ = Some(ArgT::Scalar)
            }
            return Ok(());
        }
        if self.func_names.contains(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use function `{}` in a scalar context", var)));
        }
        if self.global_arrays.contains_key(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var)));
        }
        func_state.globals_used.insert(var.clone());
        self.global_scalars = self.global_scalars.insert(var.clone(), typ).0;
        Ok(())
    }
    fn use_as_array(&mut self, var: &Symbol, func_state: &mut FuncState) -> Result<(), PrintableError> {
        if let Some(arg) = get_arg(func_state, var) {
            if let Some(arg_typ) = arg.typ {
                match arg_typ {
                    ArgT::Scalar => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context", var))),
                    ArgT::Array => {}
                }
            } else {
                arg.typ = Some(ArgT::Array)
            }
            return Ok(());
        }
        if self.func_names.contains(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use function `{}` in a scalar context", var)));
        }
        if let Some(_type) = self.global_scalars.get(var) {
            return Err(PrintableError::new(format!("fatal: attempt to scalar `{}` in an array context", var)));
        }
        self.global_arrays.insert(&var);
        Ok(())
    }

    fn analyze_stmt(&mut self, stmt: &mut Stmt, func_state: &mut FuncState) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                if let Some(ret_value) = ret {
                    self.analyze_expr(ret_value, func_state, true)?;
                }
            }
            Stmt::Printf { args: printf_args, fstring } => {
                for arg in printf_args {
                    self.analyze_expr(arg, func_state, false)?;
                }
                self.analyze_expr(fstring, func_state, false)?;
            }
            Stmt::Break => {}
            Stmt::Expr(expr) => self.analyze_expr(expr, func_state, false)?,
            Stmt::Print(expr) => self.analyze_expr(expr, func_state, false)?,
            Stmt::Group(grouping) => {
                for stmt in grouping {
                    self.analyze_stmt(stmt, func_state)?;
                }
            }
            Stmt::If(test, if_so, if_not) => {
                self.analyze_expr(test, func_state, false)?;
                let mut if_so_map = self.global_scalars.clone();
                let mut if_not_map = self.global_scalars.clone();
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);

                self.analyze_stmt(if_so, func_state)?;
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                if let Some(else_case) = if_not {
                    self.analyze_stmt(else_case, func_state)?
                }
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.global_scalars = FunctionAnalysis::merge_maps(&[&if_so_map, &if_not_map]);
            }
            Stmt::While(test, body) => {
                let pre_map = self.global_scalars.clone();
                self.analyze_expr(test, func_state, false)?;

                let after_test_map = self.global_scalars.clone();

                self.analyze_stmt(body, func_state)?;


                let after_body_map = self.global_scalars.clone();

                self.global_scalars = FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);

                self.analyze_expr(test, func_state, false)?;

                let after_test_map = self.global_scalars.clone();
                self.analyze_stmt(body, func_state)?;
                let after_body_map = self.global_scalars.clone();

                // Pass in an empty map to show that it's possible body branch never taken
                self.global_scalars = FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);
            }
        }
        Ok(())
    }

    fn analyze_expr(&mut self, expr: &mut TypedExpr, func_state: &mut FuncState, is_returned: bool) -> Result<(), PrintableError> {
        match &mut expr.expr {
            Expr::Call { args, target } => {
                for arg in args.iter_mut() {
                    if let Expr::Variable(_str) = &arg.expr {} else {
                        self.analyze_expr(arg, func_state, false)?;
                    }
                }
                let call_args = args.iter_mut().map(|arg| {
                    if let Expr::Variable(var_name) = &arg.expr {
                        if let Some(arg_t) = get_arg(func_state, var_name) {
                            CallArg::new(arg_t.name.clone())
                        } else if self.global_arrays.contains_key(var_name) {
                            CallArg::new(var_name.clone())
                        } else if self.global_scalars.get(var_name).is_some() {
                            CallArg::new(var_name.clone())
                        } else {
                            CallArg::new(var_name.clone())
                        }
                    } else {
                        CallArg::new_scalar()
                    }
                });
                let call = Call::new(target.clone(), call_args.collect());
                func_state.calls.push(call)
            }
            Expr::NumberF64(_) => {
                expr.typ = ScalarType::Float;
            }
            Expr::String(str) => {
                self.str_consts.insert(str.clone());
                expr.typ = ScalarType::String;
            }
            Expr::BinOp(left, _op, right) => {
                self.analyze_expr(left, func_state, false)?;
                self.analyze_expr(right, func_state, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::MathOp(left, _op, right) => {
                self.analyze_expr(left, func_state, false)?;
                self.analyze_expr(right, func_state, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::LogicalOp(left, _op, right) => {
                self.analyze_expr(left, func_state, false)?;
                self.analyze_expr(right, func_state, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::ScalarAssign(var, value) => {
                self.analyze_expr(value, func_state, false)?;
                self.use_as_scalar(var, value.typ, func_state)?;
                expr.typ = value.typ;
            }
            Expr::Regex(sym) => {
                self.str_consts.insert(sym.clone());
                expr.typ = ScalarType::String;
            }
            Expr::Ternary(cond, expr1, expr2) => {
                self.analyze_expr(cond, func_state, false)?;
                let mut if_so_map = self.global_scalars.clone();
                let mut if_not_map = self.global_scalars.clone();
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);

                self.analyze_expr(expr1, func_state, false)?;
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.analyze_expr(expr2, func_state, false)?;
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.global_scalars = FunctionAnalysis::merge_maps(&[&if_so_map, &if_not_map]);
                expr.typ = Self::merge_types(&expr1.typ, &expr2.typ);
            }
            Expr::Variable(var) => {
                if let Some(arg) = func_state.args.iter().find(|arg| arg.name == *var) {
                    if arg.typ == Some(ArgT::Array) && is_returned {
                        return Err(PrintableError::new(format!("fatal: attempted to use array {} in scalar context", var)));
                    }
                    expr.typ = ScalarType::Variable;
                } else if self.global_arrays.contains_key(var) && is_returned {
                    return Err(PrintableError::new(format!("fatal: attempted to use array {} in scalar context", var)));
                } else if let Some(typ) = self.global_scalars.get(var) {
                    expr.typ = *typ;
                } else {
                    expr.typ = ScalarType::Variable;
                    self.use_as_scalar(var, ScalarType::Variable, func_state)?;
                }
            }
            Expr::Column(col) => {
                expr.typ = ScalarType::String;
                self.analyze_expr(col, func_state, false)?;
            }
            Expr::NextLine => expr.typ = ScalarType::Float,
            Expr::Concatenation(vals) => {
                expr.typ = ScalarType::String;
                for val in vals {
                    self.analyze_expr(val, func_state, false)?;
                }
            }
            Expr::ArrayIndex { indices, name } => {
                self.use_as_array(name, func_state)?;
                for idx in indices {
                    self.analyze_expr(idx, func_state, false)?;
                }
            }
            Expr::InArray { indices, name } => {
                self.use_as_array(name, func_state)?;
                for idx in indices {
                    self.analyze_expr(idx, func_state, false)?;
                }
            }
            Expr::ArrayAssign { indices, name, value } => {
                self.use_as_array(name, func_state)?;
                for idx in indices {
                    self.analyze_expr(idx, func_state, false)?;
                }
                self.analyze_expr(value, func_state, false)?;
                expr.typ = value.typ;
            }
        };
        Ok(())
    }

    fn merge_maps(children: &[&MapT]) -> MapT {
        let mut merged = MapT::new();
        let mut all_vars = HashSet::new();
        for map in children {
            for (name, _typ) in map.into_iter() {
                all_vars.insert(name.clone());
            }
        }
        for var in &all_vars {
            let mut typ = None;
            for map in children {
                if let Some(typ_in_map) = map.get(var) {
                    if let Some(prior_type) = typ {
                        typ = Some(FunctionAnalysis::merge_types(&prior_type, typ_in_map));
                    } else {
                        typ = Some(*typ_in_map);
                    }
                } else {
                    typ = Some(ScalarType::Variable);
                }
            }
            let typ = typ.unwrap();
            merged = merged.insert(var.clone(), typ).0;
        }
        merged
    }
    fn merge_types(a: &ScalarType, b: &ScalarType) -> ScalarType {
        match (a, b) {
            (ScalarType::Float, ScalarType::Float) => ScalarType::Float,
            (ScalarType::String, ScalarType::String) => ScalarType::String,
            _ => ScalarType::Variable,
        }
    }
}
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use crate::parser::{ArgT, Program, ScalarType, Stmt, TypedExpr};
use crate::{Expr, PrintableError};
use crate::global_scalars::SymbolMapping;
use crate::symbolizer::Symbol;
use crate::typing::ITypedFunction;
use crate::typing::structs::{TypedUserFunction, FunctionMap, CallArg, Call};
use crate::typing::types::{AnalysisResults, MapT, TypedProgram};

pub struct FunctionAnalysis {
    global_scalars: MapT,
    global_arrays: SymbolMapping,
    func_names: HashSet<Symbol>,
    str_consts: HashSet<Symbol>,
    functions: FunctionMap,
}

pub fn function_pass(prog: Program) -> Result<TypedProgram, PrintableError> {
    let mut func_names: HashSet<Symbol> = Default::default();
    let mut functions = HashMap::new();
    for (name, function) in prog.functions {
        func_names.insert(name.clone());
        functions.insert(name, Rc::new(TypedUserFunction::new(function)));
    }

    let analysis = FunctionAnalysis {
        global_scalars: MapT::new(),
        global_arrays: SymbolMapping::new(),
        func_names,
        str_consts: Default::default(),
        functions: FunctionMap::new(functions),
    };
    analysis.analyze_program()
}

impl FunctionAnalysis {
    pub fn analyze_program(mut self) -> Result<TypedProgram, PrintableError> {
        let user_functions: Vec<Rc<TypedUserFunction>> = self.functions.user_functions()
            .iter()
            .map(|(k, v)| v.clone())
            .collect();
        for func in user_functions {
            let mut parser_func = func.function();
            self.analyze_stmt(&mut parser_func.body, &func)?;
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

        Ok(TypedProgram::new(self.functions, results))
    }
    fn use_as_scalar(&mut self, var: &Symbol, typ: ScalarType, function: &Rc<TypedUserFunction>) -> Result<(), PrintableError> {
        if let Some((_idx, arg_t)) = function.get_arg_idx_and_type(var) {
            match arg_t {
                ArgT::Scalar => {} // scalar arg used as scalar, lgtm
                ArgT::Array => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var))),
                ArgT::Unknown => { function.set_arg_type(var, ArgT::Scalar)?; }
            }
            return Ok(());
        }
        if self.func_names.contains(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use function `{}` in a scalar context", var)));
        }
        if self.global_arrays.contains_key(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var)));
        }
        function.use_global(var);
        self.global_scalars = self.global_scalars.insert(var.clone(), typ).0;
        Ok(())
    }
    fn use_as_array(&mut self, var: &Symbol, function: &Rc<TypedUserFunction>) -> Result<(), PrintableError> {
        if let Some((_idx, arg_t)) = function.get_arg_idx_and_type(var) {
            match arg_t {
                ArgT::Scalar => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context", var))),
                ArgT::Array => {}
                ArgT::Unknown => { function.set_arg_type(var, ArgT::Array)?; }
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

    fn analyze_stmt(&mut self, stmt: &mut Stmt, function: &Rc<TypedUserFunction>) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                if let Some(ret_value) = ret {
                    self.analyze_expr(ret_value, function, true)?;
                }
            }
            Stmt::Printf { args: printf_args, fstring } => {
                for arg in printf_args {
                    self.analyze_expr(arg, function, false)?;
                }
                self.analyze_expr(fstring, function, false)?;
            }
            Stmt::Break => {}
            Stmt::Expr(expr) => self.analyze_expr(expr, function, false)?,
            Stmt::Print(expr) => self.analyze_expr(expr, function, false)?,
            Stmt::Group(grouping) => {
                for stmt in grouping {
                    self.analyze_stmt(stmt, function)?;
                }
            }
            Stmt::If(test, if_so, if_not) => {
                self.analyze_expr(test, function, false)?;
                let mut if_so_map = self.global_scalars.clone();
                let mut if_not_map = self.global_scalars.clone();
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);

                self.analyze_stmt(if_so, function)?;
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                if let Some(else_case) = if_not {
                    self.analyze_stmt(else_case, function)?
                }
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.global_scalars = FunctionAnalysis::merge_maps(&[&if_so_map, &if_not_map]);
            }
            Stmt::While(test, body) => {
                let pre_map = self.global_scalars.clone();
                self.analyze_expr(test, function, false)?;

                let after_test_map = self.global_scalars.clone();

                self.analyze_stmt(body, function)?;


                let after_body_map = self.global_scalars.clone();

                self.global_scalars = FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);

                self.analyze_expr(test, function, false)?;

                let after_test_map = self.global_scalars.clone();
                self.analyze_stmt(body, function)?;
                let after_body_map = self.global_scalars.clone();

                // Pass in an empty map to show that it's possible body branch never taken
                self.global_scalars = FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);
            }
        }
        Ok(())
    }

    fn analyze_expr(&mut self, expr: &mut TypedExpr, function: &Rc<TypedUserFunction>, is_returned: bool) -> Result<(), PrintableError> {
        match &mut expr.expr {
            Expr::Call { args, target } => {
                for arg in args.iter_mut() {
                    if let Expr::Variable(_str) = &arg.expr {} else {
                        self.analyze_expr(arg, function, false)?;
                    }
                }
                let call_args = args.iter_mut().map(|arg| {
                    if let Expr::Variable(var_name) = &arg.expr {
                        CallArg::new(var_name.clone())
                    } else {
                        CallArg::new_scalar()
                    }
                });
                let target_func = match self.functions.get(target) {
                    None => return Err(PrintableError::new(format!("Function `{}` does not exist. Called from function `{}`", target, function.name()))),
                    Some(f) => f.clone(),
                };
                let call = Call::new(target_func.clone(), call_args.collect());
                function.add_call(call);
                target_func.add_caller(function.clone())
            }
            Expr::NumberF64(_) => {
                expr.typ = ScalarType::Float;
            }
            Expr::String(str) => {
                self.str_consts.insert(str.clone());
                expr.typ = ScalarType::String;
            }
            Expr::BinOp(left, _op, right) => {
                self.analyze_expr(left, function, false)?;
                self.analyze_expr(right, function, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::MathOp(left, _op, right) => {
                self.analyze_expr(left, function, false)?;
                self.analyze_expr(right, function, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::LogicalOp(left, _op, right) => {
                self.analyze_expr(left, function, false)?;
                self.analyze_expr(right, function, false)?;
                expr.typ = ScalarType::Float;
            }
            Expr::ScalarAssign(var, value) => {
                self.analyze_expr(value, function, false)?;
                self.use_as_scalar(var, value.typ, function)?;
                expr.typ = value.typ;
            }
            Expr::Regex(sym) => {
                self.str_consts.insert(sym.clone());
                expr.typ = ScalarType::String;
            }
            Expr::Ternary(cond, expr1, expr2) => {
                self.analyze_expr(cond, function, false)?;
                let mut if_so_map = self.global_scalars.clone();
                let mut if_not_map = self.global_scalars.clone();
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);

                self.analyze_expr(expr1, function, false)?;
                std::mem::swap(&mut if_so_map, &mut self.global_scalars);
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.analyze_expr(expr2, function, false)?;
                std::mem::swap(&mut if_not_map, &mut self.global_scalars);
                self.global_scalars = FunctionAnalysis::merge_maps(&[&if_so_map, &if_not_map]);
                expr.typ = Self::merge_types(&expr1.typ, &expr2.typ);
            }
            Expr::Variable(var) => {
                if let Some((_idx, arg_t)) = function.get_arg_idx_and_type(var) {
                    if arg_t == ArgT::Array && is_returned {
                        return Err(PrintableError::new(format!("fatal: attempted to use array {} in scalar context", var)));
                    }
                } else if self.global_arrays.contains_key(var) && is_returned {
                    return Err(PrintableError::new(format!("fatal: attempted to use array {} in scalar context", var)));
                } else if let Some(typ) = self.global_scalars.get(var) {
                    expr.typ = *typ;
                } else {
                    expr.typ = ScalarType::Variable;
                    self.use_as_scalar(var, ScalarType::Variable, function)?;
                }
            }
            Expr::Column(col) => {
                expr.typ = ScalarType::String;
                self.analyze_expr(col, function, false)?;
            }
            Expr::NextLine => expr.typ = ScalarType::Float,
            Expr::Concatenation(vals) => {
                expr.typ = ScalarType::String;
                for val in vals {
                    self.analyze_expr(val, &function, false)?;
                }
            }
            Expr::ArrayIndex { indices, name } => {
                self.use_as_array(name, &function)?;
                for idx in indices {
                    self.analyze_expr(idx, function, false)?;
                }
            }
            Expr::InArray { indices, name } => {
                self.use_as_array(name, function)?;
                for idx in indices {
                    self.analyze_expr(idx, function, false)?;
                }
            }
            Expr::ArrayAssign { indices, name, value } => {
                self.use_as_array(name, function)?;
                for idx in indices {
                    self.analyze_expr(idx, function, false)?;
                }
                self.analyze_expr(value, function, false)?;
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
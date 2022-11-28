use crate::global_scalars::SymbolMapping;
use crate::parser::{ArgT, Program, ScalarType, Stmt, TypedExpr};
use crate::symbolizer::Symbol;
use crate::typing::structs::{Call, CallArg, FunctionMap, TypedUserFunction};
use crate::typing::{AnalysisResults, ITypedFunction, MapT, TypedProgram};
use crate::{Expr, PrintableError};
use hashbrown::{HashMap, HashSet};
use std::rc::Rc;

pub struct FunctionAnalysis {
    global_scalars: MapT,
    global_arrays: SymbolMapping,
    str_consts: HashSet<Symbol>,
    functions: FunctionMap,
}

pub fn function_pass(prog: Program) -> Result<TypedProgram, PrintableError> {
    let mut functions = HashMap::new();
    for (name, function) in prog.functions {
        functions.insert(name, Rc::new(TypedUserFunction::new(function)));
    }

    let analysis = FunctionAnalysis {
        global_scalars: MapT::new(),
        global_arrays: SymbolMapping::new(),
        str_consts: Default::default(),
        functions: FunctionMap::new(functions, &prog.symbolizer),
    };
    analysis.analyze_program()
}

impl FunctionAnalysis {
    pub fn analyze_program(mut self) -> Result<TypedProgram, PrintableError> {
        let user_functions: Vec<Rc<TypedUserFunction>> = self
            .functions
            .user_functions()
            .iter()
            .map(|(_, v)| v.clone())
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

    fn is_func_name(&mut self, sym: &Symbol) -> bool  {
        self.functions.get(sym).is_some()
    }

    fn use_as_scalar(
        &mut self,
        var: &Symbol,
        typ: ScalarType,
        function: &Rc<TypedUserFunction>,
    ) -> Result<(), PrintableError> {
        if let Some((_idx, arg_t)) = function.get_arg_idx_and_type(var) {
            match arg_t {
                ArgT::Scalar => {} // scalar arg used as scalar, lgtm
                ArgT::Array => {
                    return Err(PrintableError::new(format!(
                        "fatal: attempt to use array `{}` in a scalar context",
                        var
                    )));
                }
                ArgT::Unknown => {
                    function.set_arg_type(var, ArgT::Scalar)?;
                }
            }
            return Ok(());
        }
        if self.is_func_name(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to use function `{}` in a scalar context",
                var
            )));
        }
        if self.global_arrays.contains_key(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to use array `{}` in a scalar context",
                var
            )));
        }
        function.use_global(var);
        self.global_scalars = self.global_scalars.insert(var.clone(), typ).0;
        Ok(())
    }
    fn use_as_array(
        &mut self,
        var: &Symbol,
        function: &Rc<TypedUserFunction>,
    ) -> Result<(), PrintableError> {
        if let Some((_idx, arg_t)) = function.get_arg_idx_and_type(var) {
            match arg_t {
                ArgT::Scalar => {
                    return Err(PrintableError::new(format!(
                        "fatal: attempt to use scalar `{}` in a array context",
                        var
                    )));
                }
                ArgT::Array => {}
                ArgT::Unknown => {
                    function.set_arg_type(var, ArgT::Array)?;
                }
            }
            return Ok(());
        }
        if self.is_func_name(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to use function `{}` in a scalar context",
                var
            )));
        }
        if let Some(_type) = self.global_scalars.get(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to scalar `{}` in an array context",
                var
            )));
        }
        self.global_arrays.insert(&var);
        Ok(())
    }

    fn analyze_stmt(
        &mut self,
        stmt: &mut Stmt,
        function: &Rc<TypedUserFunction>,
    ) -> Result<(), PrintableError> {
        match stmt {
            Stmt::Return(ret) => {
                if let Some(ret_value) = ret {
                    self.analyze_expr(ret_value, function, true)?;
                }
            }
            Stmt::Printf {
                args: printf_args,
                fstring,
            } => {
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

                self.global_scalars =
                    FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);

                self.analyze_expr(test, function, false)?;

                let after_test_map = self.global_scalars.clone();
                self.analyze_stmt(body, function)?;
                let after_body_map = self.global_scalars.clone();

                // Pass in an empty map to show that it's possible body branch never taken
                self.global_scalars =
                    FunctionAnalysis::merge_maps(&[&after_test_map, &after_body_map, &pre_map]);
            }
        }
        Ok(())
    }

    fn analyze_expr(
        &mut self,
        expr: &mut TypedExpr,
        function: &Rc<TypedUserFunction>,
        is_returned: bool,
    ) -> Result<(), PrintableError> {
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
                    None => {
                        return Err(PrintableError::new(format!(
                            "Function `{}` does not exist. Called from function `{}`",
                            target,
                            function.name()
                        )));
                    }
                    Some(f) => f.clone(),
                };
                let call = Call::new(function.clone(), target_func.clone(), call_args.collect());
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
                        return Err(PrintableError::new(format!(
                            "fatal: attempted to use array {} in scalar context",
                            var
                        )));
                    }
                } else if self.global_arrays.contains_key(var) && is_returned {
                    return Err(PrintableError::new(format!(
                        "fatal: attempted to use array {} in scalar context",
                        var
                    )));
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
            Expr::ArrayAssign {
                indices,
                name,
                value,
            } => {
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
        let mut merged = vec![];
        for var in  children.into_iter()
            .map(|map| map.into_iter().map(|(k, _value)| k.clone()))
            .flatten() {

            if merged.iter().find(|(var_name,_v)| *var_name == var).is_some() { continue };
            // Invariant: at least one map contains `var` and thus typ will be assigned a non-0
            // value at least one in the loop leaving it as a valid ScalarType enum.
            let mut typ: i32 = 0;
            for map in children {
                let map_typ = match map.get(&var) {
                    None => ScalarType::Variable,
                    Some(typ) => *typ,
                };
                typ = map_typ as i32 | typ;
            }
            let scalar_type = unsafe { std::mem::transmute::<i32, ScalarType>(typ) };
            debug_assert!(scalar_type == ScalarType::Variable || scalar_type == ScalarType::String || scalar_type == ScalarType::Float);

            // let pos = binary.unwrap_or_else(|e| e);
            merged.push((var, scalar_type));
        }
        let merged_immutable = MapT::new();
        let map = merged_immutable.insert_many(merged.into_iter());
        map
    }
    fn merge_types(a: &ScalarType, b: &ScalarType) -> ScalarType {
        unsafe { std::mem::transmute::<i32, ScalarType>(*a as i32 | *b as i32 )}
    }
}
//
// #[cfg(test)]
// mod bench_mod {
//     use super::*;
//     use test::Bencher;
//     use crate::parser::ArgT::Scalar;
//     use crate::Symbolizer;
//
//
//     fn merge_maps(children: &[&MapT]) -> MapT {
//         let mut merged = vec![];
//         let all_vars: HashSet<Symbol> = children.into_iter()
//             .map(|map| map.into_iter().map(|(k, _value)| k.clone()))
//             .flatten()
//             .collect::<HashSet<Symbol>>();
//         for var in all_vars {
//             // Invariant: at least one map contains `var` and thus typ will be assigned a non-0
//             // value at least one in the loop leaving it as a valid ScalarType enum.
//             let mut typ: i32 = 0;
//             for map in children {
//                 let map_typ = match map.get(&var) {
//                     None => ScalarType::Variable,
//                     Some(typ) => *typ,
//                 };
//                 typ = map_typ as i32 | typ;
//             }
//             let scalar_type = unsafe { std::mem::transmute::<i32, ScalarType>(typ) };
//             debug_assert!(scalar_type == ScalarType::Variable || scalar_type == ScalarType::String || scalar_type == ScalarType::Float);
//             merged.push((var, scalar_type));
//         }
//         let merged_immutable = MapT::new();
//         let map = merged_immutable.insert_many(merged.into_iter());
//         map
//     }
//
//     fn merge_newer(children: &[&MapT]) -> MapT {
//         let mut merged = vec![];
//         for var in  children.into_iter()
//             .map(|map| map.into_iter().map(|(k, _value)| k.clone()))
//             .flatten() {
//
//             if merged.iter().find(|(k,v)| *k == var).is_some() { continue };
//             // Invariant: at least one map contains `var` and thus typ will be assigned a non-0
//             // value at least one in the loop leaving it as a valid ScalarType enum.
//             let mut typ: i32 = 0;
//             for map in children {
//                 let map_typ = match map.get(&var) {
//                     None => ScalarType::Variable,
//                     Some(typ) => *typ,
//                 };
//                 typ = map_typ as i32 | typ;
//             }
//             let scalar_type = unsafe { std::mem::transmute::<i32, ScalarType>(typ) };
//             debug_assert!(scalar_type == ScalarType::Variable || scalar_type == ScalarType::String || scalar_type == ScalarType::Float);
//
//             // let pos = binary.unwrap_or_else(|e| e);
//             merged.push((var, scalar_type));
//         }
//         let merged_immutable = MapT::new();
//         let map = merged_immutable.insert_many(merged.into_iter());
//         map
//     }
//
//     fn merge_binary_search(children: &[&MapT]) -> MapT {
//         let mut merged: Vec<(Symbol, ScalarType)> = vec![];
//         for var in  children.into_iter()
//             .map(|map| map.into_iter().map(|(k, _value)| k))
//             .flatten() {
//
//             let binary = merged.binary_search_by(|(name, _typ)| name.cmp(var));
//             if binary.is_ok() { continue }
//             // if merged.iter().find(|(k,v)| *k == var).is_some() { continue };
//             // Invariant: at least one map contains `var` and thus typ will be assigned a non-0
//             // value at least one in the loop leaving it as a valid ScalarType enum.
//
//             let mut typ: i32 = 0;
//             for map in children {
//                 let map_typ = match map.get(var) {
//                     None => ScalarType::Variable,
//                     Some(typ) => *typ,
//                 };
//                 typ = map_typ as i32 | typ;
//             }
//             let scalar_type = unsafe { std::mem::transmute::<i32, ScalarType>(typ) };
//             debug_assert!(scalar_type == ScalarType::Variable || scalar_type == ScalarType::String || scalar_type == ScalarType::Float);
//
//             let pos = binary.unwrap_or_else(|e| e);
//             merged.insert(pos, (var.clone(), scalar_type));
//         }
//         let merged_immutable = MapT::new();
//         let map = merged_immutable.insert_many(merged.into_iter());
//         map
//     }
//
//
//
//
//     fn gen_map1(s: &mut Symbolizer) -> MapT {
//         MapT::new()
//             .insert(s.get("a"), ScalarType::Float)
//             .0
//             .insert(s.get("b"), ScalarType::String)
//             .0
//             .insert(s.get("cccc"), ScalarType::Variable)
//             .0
//             .insert(s.get("ccccc"), ScalarType::Variable)
//             .0
//             .insert(s.get("cccCc"), ScalarType::Variable)
//             .0
//             .insert(s.get("ccZcc"), ScalarType::String)
//             .0
//             .insert(s.get("cccCZZc"), ScalarType::Variable)
//             .0
//             .insert(s.get("cc12311Zcc"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc1"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc2"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc3"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc4"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc5"), ScalarType::String)
//             .0
//     }
//
//     fn gen_map3(s: &mut Symbolizer) -> MapT {
//         MapT::new()
//             .insert(s.get("a"), ScalarType::String)
//             .0
//             .insert(s.get("b"), ScalarType::Variable)
//             .0
//             .insert(s.get("cccc"), ScalarType::Variable)
//             .0
//             .insert(s.get("123cccc"), ScalarType::Variable)
//             .0
//             .insert(s.get("c333ccc"), ScalarType::Variable)
//             .0
//             .insert(s.get("ccccc"), ScalarType::Float)
//             .0
//             .insert(s.get("cccCc"), ScalarType::Float)
//             .0
//             .insert(s.get("ccZcc"), ScalarType::String)
//             .0
//             .insert(s.get("cccCZ11Zc"), ScalarType::Float)
//             .0
//             .insert(s.get("cc12311Zcc"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc1"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc2"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc3"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc4"), ScalarType::String)
//             .0
//             .insert(s.get("cc12311Zcc5"), ScalarType::String)
//             .0
//     }
//
//     fn gen_map2(s: &mut Symbolizer) -> MapT {
//         MapT::new()
//             .insert(s.get("b"), ScalarType::String)
//             .0
//             .insert(s.get("a"), ScalarType::Float)
//             .0
//             .insert(s.get("azzz"), ScalarType::Float)
//             .0
//             .insert(s.get("sfdsda"), ScalarType::Float)
//             .0
//             .insert(s.get("cccc"), ScalarType::String)
//             .0
//     }
//
//     #[bench]
//     fn merge_init(b: &mut Bencher) {
//         let mut s = Symbolizer::new();
//         let map1 = gen_map1(&mut s);
//         let map2 = gen_map2(&mut s);
//         let map3 = gen_map3(&mut s);
//         b.iter(|| merge_maps(&[&map1, &map2, &map3]));
//     }
//
//     #[bench]
//     fn merged_newer(b: &mut Bencher) {
//         let mut s = Symbolizer::new();
//         let map1 = gen_map1(&mut s);
//         let map2 = gen_map2(&mut s);
//         let map3 = gen_map3(&mut s);
//
//         b.iter(|| merge_newer(&[&map1, &map2, &map3]));
//     }
//
//     #[bench]
//     fn merged_binary(b: &mut Bencher) {
//         let mut s = Symbolizer::new();
//         let map1 = gen_map1(&mut s);
//         let map2 = gen_map2(&mut s);
//         let map3 = gen_map3(&mut s);
//
//         b.iter(|| merge_binary_search(&[&map1, &map2, &map3]));
//     }
// }

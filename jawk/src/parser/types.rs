use crate::lexer::{BinOp, LogicalOp, MathOp};
use crate::symbolizer::Symbol;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use libc::write;
use crate::awk_str::AwkStr;

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
#[repr(i32)]
pub enum ScalarType {
    // This form is useful b/c String | Float == Variable
    // See merge_types_function
    String = 0b0000_0001,
    Float = 0b0000_0010,
    Variable = 0b0000_0011,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum AwkT {
    Scalar(ScalarType),
    Array,
}

impl Into<AwkT> for ScalarType {
    fn into(self) -> AwkT {
        AwkT::Scalar(self)
    }
}

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub enum Stmt {
    Expr(TypedExpr),
    Print(TypedExpr),
    Group(Vec<Stmt>),
    If(TypedExpr, Box<Stmt>, Option<Box<Stmt>>),
    While(TypedExpr, Box<Stmt>),
    Printf {
        fstring: TypedExpr,
        args: Vec<TypedExpr>,
    },
    Break,
    Return(Option<TypedExpr>),
}

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::Return(ret) => {
                write!(f, "return")?;
                if let Some(ret) = ret {
                    write!(f, " {}", ret)?;
                }
            }
            Stmt::Printf { fstring, args } => {
                write!(f, "printf \"{}\"", fstring)?;
                for (idx, mem) in args.iter().enumerate() {
                    write!(f, "{}", mem.expr)?;
                    if idx + 1 != args.len() {
                        write!(f, ", ")?;
                    }
                }
            }
            Stmt::Expr(expr) => write!(f, "{}", expr)?,
            Stmt::Print(expr) => write!(f, "print {}", expr)?,
            Stmt::Group(group) => {
                for elem in group {
                    write!(f, "{}", elem)?;
                }
            }
            Stmt::If(test, if_so, if_not) => {
                write!(f, "if {} {{{}}}", test, if_so)?;
                if let Some(else_case) = if_not {
                    write!(f, "else {{ {} }}", else_case)?;
                }
            }
            Stmt::While(test, body) => {
                write!(f, "while {} {{{}}} ", test, body)?;
            }
            Stmt::Break => write!(f, "break")?,
        };
        write!(f, "\n")
    }
}

#[derive(Debug, PartialEq)]
pub struct PatternAction {
    pub pattern: Option<TypedExpr>,
    pub action: Stmt,
}

impl PatternAction {
    pub fn new<ExprT: Into<Option<TypedExpr>>>(pattern: ExprT, action: Stmt) -> Self {
        Self {
            pattern: pattern.into(),
            action,
        }
    }
    pub fn new_pattern_only(test: TypedExpr) -> PatternAction {
        PatternAction::new(
            Some(test),
            Stmt::Print(Expr::Column(Box::new(Expr::NumberF64(0.0).into())).into()),
        )
    }
    pub fn new_action_only(body: Stmt) -> PatternAction {
        PatternAction::new(None, body)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct TypedExpr {
    pub typ: ScalarType,
    pub expr: Expr,
}

impl TypedExpr {
    pub fn new(expr: Expr) -> TypedExpr {
        TypedExpr {
            typ: ScalarType::Variable,
            expr,
        }
    }
}

impl Into<TypedExpr> for Expr {
    fn into(self) -> TypedExpr {
        TypedExpr::new(self)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Expr {
    ScalarAssign(Symbol, Box<TypedExpr>),
    ArrayAssign {
        name: Symbol,
        indices: Vec<TypedExpr>,
        value: Box<TypedExpr>,
    },
    NumberF64(f64),
    String(Rc<AwkStr>),
    Regex(Rc<AwkStr>),
    Concatenation(Vec<TypedExpr>),
    BinOp(Box<TypedExpr>, BinOp, Box<TypedExpr>),
    MathOp(Box<TypedExpr>, MathOp, Box<TypedExpr>),
    LogicalOp(Box<TypedExpr>, LogicalOp, Box<TypedExpr>),
    Variable(Symbol),
    Column(Box<TypedExpr>),
    NextLine,
    Ternary(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>),
    ArrayIndex {
        name: Symbol,
        indices: Vec<TypedExpr>,
    },
    InArray {
        name: Symbol,
        indices: Vec<TypedExpr>,
    },
    Call {
        target: Symbol,
        args: Vec<TypedExpr>,
    },
    // Sub is unqiue in it takes an LValue as an arg.
    // I could built out a whole LValue RValue system but it seems not worth
    // it just for this
    CallSub {
        arg1: Box<TypedExpr>,
        arg2: Box<TypedExpr>,
        arg3: LValue,
        global: bool, // true => gsub() else sub()
    },
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum LValue {
    Variable(Symbol),
    ArrayIndex {
        name: Symbol,
        indices: Vec<TypedExpr>,
    },
    Column(Box<TypedExpr>),
}

impl Display for LValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LValue::Variable(n) => write!(f, "{}", n),
            LValue::Column(col) => write!(f, "${}", col),
            LValue::ArrayIndex { name, indices } => {
                write!(f, "{}[", name)?;
                display_comma_sep_list(f, indices)?;
                write!(f, "]")
            }
        }
    }
}

impl Into<Expr> for LValue {
    fn into(self) -> Expr {
        match self {
            LValue::Variable(var) => Expr::Variable(var),
            LValue::ArrayIndex { name, indices } => Expr::ArrayIndex { name, indices },
            LValue::Column(expr) => Expr::Column(expr)
        }
    }
}

impl TryFrom<Expr> for LValue {
    type Error = ();

    fn try_from(expr: Expr) -> Result<Self, ()> {
        match expr {
            Expr::Variable(name) => Ok(LValue::Variable(name)),
            Expr::Column(col) => Ok(LValue::Column(col)),
            Expr::ArrayIndex { name, indices } => Ok(LValue::ArrayIndex { name, indices }),
            _ => Err(()),
        }
    }
}

impl Display for TypedExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.typ {
            ScalarType::String => write!(f, "(s {})", self.expr),
            ScalarType::Float => write!(f, "(f {})", self.expr),
            ScalarType::Variable => write!(f, "(v {})", self.expr),
        }
    }
}

fn display_comma_sep_list<T: Display>(f: &mut Formatter<'_>, indices: &[T]) -> std::fmt::Result {
    for (idx_idx, idx_expr) in indices.iter().enumerate() {
        if idx_idx != indices.len() - 1 {
            write!(f, "{},", idx_expr)?;
        } else {
            write!(f, "{}", idx_expr)?;
        }
    }
    Ok(())
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        #[cfg(not(debug_assertions))]
        panic!("compiler bug: displaying u8 strings cannot use rust format! it is not utf-8 safe");

        match self {
            Expr::Call { target, args } => {
                write!(f, "{}(", target)?;
                for arg in args {
                    write!(f, "{},", arg)?;
                }
                write!(f, ")")
            }
            Expr::ScalarAssign(var, expr) => write!(f, "{} = {}", var, expr),
            Expr::NextLine => write!(f, "check_if_there_is_another_line"),
            Expr::Variable(n) => write!(f, "{}", n),
            Expr::String(str) => {
                let str = unsafe { String::from_utf8_unchecked(str.bytes().to_vec()) };
                write!(f, "\"{}\"", str)
            },
            Expr::Regex(reg) => {
                let reg = unsafe { String::from_utf8_unchecked(reg.bytes().to_vec()) };
                write!(f, "\"{}\"", reg)
            }

            Expr::NumberF64(n) => write!(f, "{}", n),
            Expr::BinOp(left, op, right) => write!(f, "{}{}{}", left, op, right),
            Expr::Ternary(cond, expr1, expr2) => write!(f, "{} ? {} : {}", cond, expr1, expr2),
            Expr::MathOp(left, op, right) => write!(f, "{}{}{}", left, op, right),
            Expr::LogicalOp(left, op, right) => write!(f, "{}{}{}", left, op, right),
            Expr::Column(col) => write!(f, "${}", col),
            Expr::Concatenation(vals) => {
                let vals = vals
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<String>>();
                let str = vals.join(" ");
                write!(f, "{}", str)
            }

            Expr::ArrayIndex { name, indices } => {
                write!(f, "{}[", name)?;
                display_comma_sep_list(f, indices)?;
                write!(f, "]")
            }
            Expr::InArray { name, indices } => {
                write!(f, "(")?;
                display_comma_sep_list(f, indices)?;
                write!(f, ") in {}", name)
            }
            Expr::ArrayAssign {
                name,
                indices,
                value,
            } => {
                write!(f, "{}[", name)?;
                display_comma_sep_list(f, indices)?;
                write!(f, "] = {}", value)
            }

            Expr::CallSub { arg1, arg2, arg3, global } => {
                let name = if *global { "gsub"} else { "sub"};
                write!(f, "{}({},{},{})", name, arg1, arg2, arg3)
            }
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone)]
pub enum ArgT {
    Scalar,
    Array,
    Unknown,
}

impl Display for ArgT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgT::Scalar => write!(f, "s"),
            ArgT::Array => write!(f, "a"),
            ArgT::Unknown => write!(f, "u"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Arg {
    pub name: Symbol,
    pub typ: ArgT,
    pub builtin_optional: bool, // Like split(string, arr [, fs] ) this arguments is an optional for a builtin. All args for user functions are optional.
}

impl Arg {
    pub fn new(name: Symbol, typ: ArgT) -> Self {
        Self { name, typ, builtin_optional: false }
    }
    pub fn new_optional(name: Symbol, typ: ArgT) -> Self {
        Self { name, typ, builtin_optional: true }
    }
    pub fn new_scl(name: Symbol) -> Self { Self { name, typ: ArgT::Scalar, builtin_optional: false } }
    pub fn new_arr(name: Symbol) -> Self { Self { name, typ: ArgT::Array, builtin_optional: false } }
}

impl Display for Arg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.typ, self.name)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Function {
    pub name: Symbol,
    pub args: Vec<Symbol>,
    pub body: Stmt,
}

impl Function {
    pub fn new(name: Symbol, args: Vec<Symbol>, body: Stmt) -> Self {
        Function {
            name: name.into(),
            args,
            body,
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "function {}(", self.name)?;
        for (idx, arg) in self.args.iter().enumerate() {
            write!(f, "{}", arg)?;
            if idx != self.args.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, ") {{\n{}\n}}", self.body)
    }
}

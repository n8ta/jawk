mod test;
mod transformer;
mod types;

use crate::lexer::{BinOp, LogicalOp, MathOp, Token, TokenType};
use crate::parser::transformer::transform;
pub use crate::parser::types::PatternAction;
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::BuiltinFunc;
use crate::{AnalysisResults, PRINTF_MAX_ARGS, Symbolizer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
pub use types::{Arg, ArgT, Expr, LValue, Function, ScalarType, Stmt, TypedExpr};

// Pattern Action Type
// Normal eg: $1 == "a" { doSomething() }
// Begin 'BEGIN { ... }'
// End  'END { .... }'
enum PAType {
    Normal(PatternAction),
    Begin(Stmt),
    End(Stmt),
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub functions: HashMap<Symbol, Function>,
    pub global_analysis: AnalysisResults,
    pub symbolizer: Symbolizer,
}

impl Program {
    #[cfg(test)]
    fn new_action_only(name: Symbol, action: Stmt, symbolizer: Symbolizer) -> Program {
        let body = transform(vec![], vec![], vec![PatternAction::new_action_only(action)]);
        let mut functions = HashMap::new();
        functions.insert(name.clone(), Function::new(name, vec![], body));
        Program {
            functions,
            global_analysis: AnalysisResults::new(),
            symbolizer,
        }
    }
    pub fn new(
        name: Symbol,
        begins: Vec<Stmt>,
        ends: Vec<Stmt>,
        pas: Vec<PatternAction>,
        parsed_functions: Vec<Function>,
        symbolizer: Symbolizer,
    ) -> Program {
        let body = transform(begins, ends, pas);
        let main = Function::new(name.clone(), vec![], body);
        let mut functions = HashMap::new();
        functions.insert(name, main);
        for func in parsed_functions {
            functions.insert(func.name.clone(), func);
        }
        Program {
            functions,
            global_analysis: AnalysisResults::new(),
            symbolizer,
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Tests will print the program and compare it with another string
        // keep function order consistent by sorting.
        let mut sorted: Vec<Symbol> = self.functions.iter().map(|(sym, _)| sym.clone()).collect();
        sorted.sort();
        for func_name in &sorted {
            let func = self.functions.get(func_name).unwrap();
            write!(f, "{}\n", func)?;
        }
        Ok(())
    }
}

const STRING_CONCAT_SKIPS: u64 = TokenType::InplaceAssign as u64
    | TokenType::Less as u64
    | TokenType::LessEq as u64
    | TokenType::BangEq as u64
    | TokenType::EqEq as u64
    | TokenType::Greater as u64
    | TokenType::GreaterEq as u64
    | TokenType::And as u64
    | TokenType::Or as u64
    | TokenType::Eq as u64
    | TokenType::Semicolon as u64
    | TokenType::RightBrace as u64
    | TokenType::RightParen as u64
    | TokenType::LeftBrace as u64
    | TokenType::Question as u64
    | TokenType::Colon as u64
    | TokenType::MatchedBy as u64
    | TokenType::NotMatchedBy as u64
    | TokenType::Comma as u64
    | TokenType::In as u64
    | TokenType::LeftBracket as u64
    | TokenType::RightBracket as u64
    | TokenType::Printf as u64;

pub fn parse(tokens: Vec<Token>, symbolizer: &mut Symbolizer) -> Result<Program, PrintableError> {
    let sub = symbolizer.get("sub");
    let gsub = symbolizer.get("gsub");
    let mut parser = Parser {
        tokens,
        current: 0,
        symbolizer,
        sub,
        gsub,
    };
    parser.parse()
}

struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    symbolizer: &'a mut Symbolizer,
    sub: Symbol,
    gsub: Symbol,
}

macro_rules! flags {
    // Base case:
    ($x:expr) => ($x as u64);
    ($x:expr, $($y:expr),+) => ( flags!($x) | flags!($($y),+))
}

impl<'a> Parser<'a> {
    fn parse(&mut self) -> Result<Program, PrintableError> {
        let mut begins = vec![];
        let mut ends = vec![];
        let mut pattern_actions = vec![];
        let mut functions = vec![];
        while !self.is_at_end() {
            if self.matches(flags!(TokenType::Function)) {
                let name = self.ident_consume("Function name must follow function keyword")?;
                if BuiltinFunc::get(&name.sym).is_some() {
                    return Err(PrintableError::new(format!(
                        "Cannot name a function {} as that is a builtin function",
                        name
                    )));
                }
                self.consume(
                    TokenType::LeftParen,
                    "Function name must be followed by '('",
                )?;
                let mut args = vec![];
                loop {
                    if self.peek().ttype() != TokenType::RightParen {
                        args.push(self.ident_consume("Expected function argument name here")?);
                    } else {
                        break;
                    }
                    if self.peek().ttype() != TokenType::RightParen {
                        self.consume(
                            TokenType::Comma,
                            "Expected comma after function argument and before right paren",
                        )?;
                        continue;
                    }
                    break;
                }
                self.consume(
                    TokenType::RightParen,
                    "Expected right paren after function arguments",
                )?;
                let body = self.group()?;
                functions.push(Function::new(name, args, body))
            } else {
                match self.pattern_action()? {
                    PAType::Normal(pa) => pattern_actions.push(pa),
                    PAType::Begin(pa) => begins.push(pa),
                    PAType::End(pa) => ends.push(pa),
                };
            }
        }
        Ok(Program::new(
            self.symbolizer.get("main function"),
            begins,
            ends,
            pattern_actions,
            functions,
            self.symbolizer.clone(),
        ))
    }

    fn ident_consume(&mut self, error_msg: &str) -> Result<Symbol, PrintableError> {
        if let Token::Ident(ident) = self.consume(TokenType::Ident, error_msg)? {
            return Ok(ident);
        }
        unreachable!()
    }

    fn check(&mut self, typ: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            typ == self.peek().ttype()
        }
    }

    fn consume(&mut self, typ: TokenType, message: &str) -> Result<Token, PrintableError> {
        if self.check(typ.clone()) {
            return self.advance();
        }
        Err(PrintableError::new(format!(
            "{} - didn't find a `{}` as expected. Found `{}`",
            message,
            TokenType::name(typ),
            TokenType::name(self.peek().ttype()),
        )))
    }

    // Caller guarantees that last token parsed was an ident
    fn prior_indent_name_infallible(&mut self) -> Symbol {
        if let Some(Token::Ident(name)) = self.previous() {
            name.clone()
        } else {
            unreachable!()
        }
    }

    // The next N tokens type's match tokens arg
    fn matches_series(&mut self, tokens: &[TokenType]) -> bool {
        for (idx, tt) in tokens.iter().enumerate() {
            let peeked = self.peek_at(self.current + idx).ttype();
            if peeked != *tt {
                return false;
            }
        }
        self.current += tokens.len();
        return true;
    }

    // The current token is present in the tokens bitflags
    fn matches(&mut self, tokens: u64) -> bool {
        let tkn = match self.tokens.get(self.current) {
            None => return false,
            Some(t) => t.ttype().clone(),
        };
        if (tokens & tkn as u64) != 0 {
            self.advance().unwrap();
            return true;
        }
        false
    }

    fn previous(&self) -> Option<Token> {
        if self.current == 0 {
            return None;
        }
        Some(self.tokens[self.current - 1].clone())
    }

    fn peek_at(&self, idx: usize) -> &Token {
        if let Some(t) = self.tokens.get(idx) {
            t
        } else {
            &Token::EOF
        }
    }

    fn peek(&self) -> &Token {
        self.peek_at(self.current)
    }

    fn peek_next(&self) -> &Token {
        self.peek_at(self.current + 1)
    }

    fn is_at_end(&self) -> bool {
        self.tokens[self.current].ttype() == TokenType::EOF
    }

    fn advance(&mut self) -> Result<Token, PrintableError> {
        if !self.is_at_end() {
            self.current += 1;
        }
        match self.previous() {
            None => Err(PrintableError::new("Reached end of file unexpectedly.")),
            Some(tok) => Ok(tok),
        }
    }

    fn pattern_action(&mut self) -> Result<PAType, PrintableError> {
        let b = if self.matches(flags!(TokenType::LeftBrace)) {
            // { print 1; }
            let pa = PAType::Normal(PatternAction::new_action_only(self.stmts()?));
            self.consume(TokenType::RightBrace, "Expected '}' after action block")?;
            pa
        } else if self.matches(flags!(TokenType::Begin)) {
            // BEGIN { print 1; }
            self.consume(TokenType::LeftBrace, "Expected a `{` after a begin")?;
            let pa = PAType::Begin(self.stmts()?);
            self.consume(TokenType::RightBrace, "Begin action should end with '}'")?;
            pa
        } else if self.matches(flags!(TokenType::End)) {
            // END { print 1; }
            self.consume(TokenType::LeftBrace, "Expected a `{` after a end")?;
            let pa = PAType::End(self.stmts()?);
            self.consume(TokenType::RightBrace, "End action should end with '}'")?;
            pa
        } else {
            let test = self.expression()?;
            if self.matches(flags!(TokenType::LeftBrace)) {
                // test { print 1; }
                let pa = PAType::Normal(PatternAction::new(Some(test), self.stmts()?));
                self.consume(TokenType::RightBrace, "Patern action should end with '}'")?;
                pa
            } else {
                // test
                // ^ implicitly prints line if test passes
                PAType::Normal(PatternAction::new_pattern_only(test))
            }
        };
        Ok(b)
    }
    fn group(&mut self) -> Result<Stmt, PrintableError> {
        self.consume(TokenType::LeftBrace, "Expected a `{` to start group")?;
        let stmt = self.stmts()?;
        self.consume(TokenType::RightBrace, "Expected a `}` to end group")?;
        Ok(stmt)
    }

    fn stmt_and_optional_semicolon(&mut self) -> Result<Stmt, PrintableError> {
        let stmt = self.stmt()?;
        if self.peek().ttype() == TokenType::Semicolon {
            self.consume(TokenType::Semicolon, "not possible")?;
        }
        Ok(stmt)
    }

    fn stmt(&mut self) -> Result<Stmt, PrintableError> {
        let stmt = if self.matches(flags!(TokenType::Print)) {
            Stmt::Print(self.expression()?) // TODO: print 1,2,3
        } else if self.matches(flags!(TokenType::Ret)) {
            if self.peek().ttype() != TokenType::RightBrace
                && self.peek_next().ttype() != TokenType::Semicolon
            {
                let expr = self.expression()?;
                Stmt::Return(Some(expr))
            } else {
                Stmt::Return(None)
            }
        } else if self.matches(flags!(TokenType::Printf)) {
            let fstring = self.expression()?;
            let mut args = vec![];
            while self.matches(flags!(TokenType::Comma)) {
                args.push(self.expression()?);
            }
            if args.len() > PRINTF_MAX_ARGS {
                return Err(PrintableError::new(format!("printf supports a max of {} arguments. Called with {} arguments", PRINTF_MAX_ARGS, args.len())));
            }
            Stmt::Printf { fstring, args }
        } else if self.matches(flags!(TokenType::Break)) {
            Stmt::Break
        } else if self.matches(flags!(TokenType::For)) {
            self.consume(TokenType::LeftParen, "Expected a `(` after the for keyword")?;
            let init = self.stmt()?;
            self.consume(
                TokenType::Semicolon,
                "Expected a `;` after for loop init statement",
            )?;
            let test = self.expression()?;
            self.consume(
                TokenType::Semicolon,
                "Expected a `;` after for loop test statement",
            )?;
            let incr = self.stmt()?;
            self.consume(TokenType::RightParen, "Expected a `)` to end for loop")?;
            self.consume(
                TokenType::LeftBrace,
                "Expected a `{` to begin for loop body",
            )?;
            let body = self.stmts()?;
            self.consume(TokenType::RightBrace, "Expected a `}` after for loop body")?;
            Stmt::Group(vec![
                init,
                Stmt::While(test, Box::new(Stmt::Group(vec![body, incr]))),
            ])
        } else if self.peek_next().ttype() == TokenType::Eq {
            let str = if let Token::Ident(str) = self.consume(TokenType::Ident, "Expected identifier before `=`")?
            {
                str
            } else {
                return Err(PrintableError::new("Expected an identifier before an `=`"));
            };
            self.consume(TokenType::Eq, "Expected `=` after identifier")?;
            Stmt::Expr(TypedExpr::new(Expr::ScalarAssign(
                str,
                Box::new(self.expression()?),
            )))
            // } else if self.any_match(&[TokenType::Ret]) {
            //     self.return_stmt()
        } else if self.matches(flags!(TokenType::While)) {
            self.consume(TokenType::LeftParen, "Must have paren after while")?;
            let expr = self.expression()?;
            self.consume(
                TokenType::RightParen,
                "Must have right parent after while statement test expression",
            )?;
            self.consume(TokenType::LeftBrace, "Must have brace after `while (expr)`")?;
            let stmts = self.stmts()?;
            self.consume(TokenType::RightBrace, "While loop must be followed by '}'")?;
            Stmt::While(expr, Box::new(stmts))
        } else if self.matches(flags!(TokenType::Print)) {
            let expr = self.expression()?;
            Stmt::Print(expr)
        } else if self.matches(flags!(TokenType::If)) {
            self.if_stmt()?
        } else if self.matches(flags!(TokenType::LeftBrace)) {
            let s = self.stmts()?;
            self.consume(
                TokenType::RightBrace,
                "Expected a right brace after a group",
            )?;
            s
        } else {
            Stmt::Expr(self.expression()?)
        };
        Ok(stmt)
    }

    fn stmts(&mut self) -> Result<Stmt, PrintableError> {
        let mut stmts = Vec::with_capacity(5);
        while self.peek().ttype() != TokenType::RightBrace {
            let stmt = self.stmt_and_optional_semicolon()?;
            stmts.push(stmt);
        }
        if stmts.len() == 1 {
            return Ok(stmts.pop().unwrap());
        }
        Ok(Stmt::Group(stmts))
    }

    fn if_stmt(&mut self) -> Result<Stmt, PrintableError> {
        self.consume(TokenType::LeftParen, "Expected `(` after if")?;
        let predicate = self.expression()?;
        self.consume(TokenType::RightParen, "Expected `)` after if predicate")?;

        let then_blk = if self.peek().ttype() == TokenType::LeftBrace {
            self.group()?
        } else {
            self.stmt()?
        };

        let else_blk = if self.matches(flags!(TokenType::Else)) {
            let else_blk = if self.peek().ttype() == TokenType::LeftBrace {
                self.group()?
            } else {
                self.stmt()?
            };
            Some(Box::new(else_blk))
        } else {
            None
        };
        Ok(Stmt::If(predicate, Box::new(then_blk), else_blk))
    }

    fn expression(&mut self) -> Result<TypedExpr, PrintableError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<TypedExpr, PrintableError> {
        let expr = self.ternary()?;
        if let Expr::Variable(var) = &expr.expr {
            let var = var.clone();
            if self.matches(flags!(TokenType::Eq)) {
                // =
                return Ok(TypedExpr::new(Expr::ScalarAssign(
                    var,
                    Box::new(self.assignment()?),
                )));
            } else if self.matches(flags!(TokenType::InplaceAssign)) {
                // += -= *= ...
                let math_op = if let Token::InplaceEq(math_op) = self.previous().unwrap() {
                    math_op
                } else {
                    unreachable!()
                };
                let expr = Expr::MathOp(
                    Box::new(Expr::Variable(var.clone()).into()),
                    math_op,
                    Box::new(self.assignment()?),
                );
                return Ok(Expr::ScalarAssign(var, Box::new(expr.into())).into());
            }
        }
        let mut is_array_index = false;
        if let Expr::ArrayIndex { .. } = &expr.expr {
            is_array_index = true;
        }
        if is_array_index && self.matches(flags!(TokenType::Eq)) {
            if let Expr::ArrayIndex { name, indices } = expr.expr {
                let value = Box::new(self.assignment()?);
                return Ok(Expr::ArrayAssign {
                    name,
                    indices,
                    value,
                }
                    .into());
            } else {
                unreachable!()
            }
        }
        Ok(expr)
    }

    fn ternary(&mut self) -> Result<TypedExpr, PrintableError> {
        let cond = self.logical_or()?;
        while self.matches(flags!(TokenType::Question)) {
            let expr1 = self.ternary()?;
            self.consume(
                TokenType::Colon,
                "Expected a colon after question mark in a ternary!",
            )?;
            let expr2 = self.ternary()?;
            return Ok(TypedExpr::new(Expr::Ternary(
                Box::new(cond),
                Box::new(expr1),
                Box::new(expr2),
            )));
        }
        Ok(cond)
    }

    fn logical_or(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.logical_and()?;
        while self.matches(flags!(TokenType::Or)) {
            expr = TypedExpr::new(Expr::LogicalOp(
                Box::new(expr),
                LogicalOp::Or,
                Box::new(self.logical_and()?),
            ))
        }
        Ok(expr)
    }

    fn logical_and(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.array_membership()?;
        while self.matches(flags!(TokenType::And)) {
            expr = TypedExpr::new(Expr::LogicalOp(
                Box::new(expr),
                LogicalOp::And,
                Box::new(self.array_membership()?),
            ))
        }
        Ok(expr)
    }

    fn array_membership(&mut self) -> Result<TypedExpr, PrintableError> {
        // <expr> (in array_name)*
        // Weird example: `BEGIN { print 1 in a in b in c }`
        // is totally valid and prints 0
        let mut expr = self.multi_dim_array_membership()?;
        while self.matches(flags!(TokenType::In)) {
            let name = if let Token::Ident(name) =
                self.consume(TokenType::Ident, "An array name must follow `<expr> in`")?
            {
                name
            } else {
                unreachable!()
            };
            expr = Expr::InArray {
                name,
                indices: vec![expr],
            }
                .into()
        }
        Ok(expr)
    }

    fn helper_multi_dim_array(&mut self) -> Result<TypedExpr, PrintableError> {
        self.consume(
            TokenType::LeftParen,
            "Multidimensional array must begin with left paren",
        )?;
        let mut exprs = vec![self.regex()?];
        while self.matches(flags!(TokenType::Comma)) {
            if self.peek().ttype() == TokenType::RightParen {
                break;
            }
            exprs.push(self.regex()?);
        }
        self.consume(
            TokenType::RightParen,
            "Multidimensional array indices must end with right paren",
        )?;
        self.consume(
            TokenType::In,
            "Multidimensional array access must be followed by an 'in'",
        )?;
        let ident = self.consume(TokenType::Ident, "Multidimensional array access must be followed by an array name. Eg: (1,2,3) in ARRAY_NAME")?;
        let ident = if let Token::Ident(ident) = ident {
            ident
        } else {
            unreachable!("compiler bug consumed ident but got something else")
        };

        let mut expr = TypedExpr::new(Expr::InArray {
            name: ident,
            indices: exprs,
        });
        while self.matches(flags!(TokenType::In)) {
            let ident = self.consume(TokenType::Ident, "Multidimensional array access must be followed by an array name. Eg: (1,2,3) in ARRAY_NAME")?;
            let ident = if let Token::Ident(ident) = ident {
                ident
            } else {
                unreachable!("compiler bug consumed ident but got something else")
            };
            expr = Expr::InArray {
                name: ident,
                indices: vec![expr.into()],
            }
                .into();
        }
        Ok(expr)
    }

    fn multi_dim_array_membership(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut idx = self.current;
        // Check if we match the regex \(.+\) in if so call the helper
        if *self.peek_at(idx) == Token::LeftParen {
            while *self.peek_at(idx) != Token::RightParen && !self.is_at_end() {
                idx += 1;
            }
            if *self.peek_at(idx) == Token::RightParen && *self.peek_at(idx + 1) == Token::In {
                return self.helper_multi_dim_array();
            }
        }
        self.regex()
    }

    fn regex(&mut self) -> Result<TypedExpr, PrintableError> {
        // "a ~ /match/"
        let mut expr = self.compare()?;
        while self.matches(flags!(TokenType::MatchedBy, TokenType::NotMatchedBy)) {
            expr = Expr::BinOp(
                Box::new(expr),
                if self.previous().unwrap().ttype() == TokenType::MatchedBy {
                    BinOp::MatchedBy
                } else {
                    BinOp::NotMatchedBy
                },
                Box::new(self.compare()?),
            )
                .into();
        }
        Ok(expr)
    }

    fn compare(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.string_concat()?;
        while self.matches(flags!(
            TokenType::GreaterEq,
            TokenType::Greater,
            TokenType::Less,
            TokenType::LessEq,
            TokenType::EqEq,
            TokenType::BangEq
        )) {
            let op = match self.previous().unwrap() {
                Token::BinOp(BinOp::Less) => BinOp::Less,
                Token::BinOp(BinOp::LessEq) => BinOp::LessEq,
                Token::BinOp(BinOp::Greater) => BinOp::Greater,
                Token::BinOp(BinOp::GreaterEq) => BinOp::GreaterEq,
                Token::BinOp(BinOp::BangEq) => BinOp::BangEq,
                Token::BinOp(BinOp::EqEq) => BinOp::EqEq,
                _ => unreachable!("Parser bug in compare matches function"),
            };
            expr = Expr::BinOp(Box::new(expr), op, Box::new(self.string_concat()?)).into()
        }
        Ok(expr)
    }

    #[inline(always)]
    fn types_contain(bitflag_union: u64, flag: u64) -> bool {
        bitflag_union & flag != 0
    }

    fn string_concat(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.plus_minus()?;
        while !self.is_at_end()
            && !Parser::types_contain(STRING_CONCAT_SKIPS, self.peek().ttype() as u64)
        {
            if let Expr::Concatenation(vals) = &mut expr.expr {
                vals.push(self.plus_minus()?);
            } else {
                expr = TypedExpr::new(Expr::Concatenation(vec![expr, self.plus_minus()?]));
            }
        }
        Ok(expr)
    }

    fn plus_minus(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.term()?;
        while self.matches(flags!(TokenType::Plus, TokenType::Minus)) {
            let op = match self.previous().unwrap() {
                Token::MathOp(MathOp::Minus) => MathOp::Minus,
                Token::MathOp(MathOp::Plus) => MathOp::Plus,
                _ => unreachable!("Parser bug in comparison function"),
            };
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.plus_minus()?)).into();
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.unary()?;
        while self.matches(flags!(TokenType::Star, TokenType::Slash, TokenType::Modulo)) {
            let op = match self.previous().unwrap() {
                Token::MathOp(MathOp::Star) => MathOp::Star,
                Token::MathOp(MathOp::Slash) => MathOp::Slash,
                Token::MathOp(MathOp::Modulus) => MathOp::Modulus,
                _ => unreachable!("Parser bug in comparison function"),
            };
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.unary()?)).into()
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<TypedExpr, PrintableError> {
        if !(self.peek().ttype() == TokenType::Minus
            && self.peek_next().ttype() == TokenType::Minus)
            && !(self.peek().ttype() == TokenType::Plus
            && self.peek_next().ttype() == TokenType::Plus)
            && self.matches(flags!(TokenType::Minus, TokenType::Plus, TokenType::Bang))
        {
            let p = self.previous().unwrap().ttype();
            let rhs = self.unary()?;
            let one = TypedExpr::new(Expr::NumberF64(1.0));
            let zero = TypedExpr::new(Expr::NumberF64(0.0));
            return Ok(match p {
                TokenType::Bang => Expr::BinOp(Box::new(one), BinOp::BangEq, Box::new(rhs)),
                TokenType::Plus => Expr::MathOp(Box::new(zero), MathOp::Plus, Box::new(rhs)),
                TokenType::Minus => Expr::MathOp(Box::new(zero), MathOp::Minus, Box::new(rhs)),
                _ => unreachable!(),
            }
                .into());
        }
        self.exp()
    }

    fn exp(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.pre_op()?;
        while self.matches(flags!(TokenType::Exponent)) {
            let op = MathOp::Exponent;
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.pre_op()?)).into()
        }
        Ok(expr)
    }

    fn pre_op(&mut self) -> Result<TypedExpr, PrintableError> {
        if self.matches_series(&[TokenType::Plus, TokenType::Plus, TokenType::Ident])
        {
            let name = self.prior_indent_name_infallible();
            let var_expr = Expr::Variable(name.clone()).into();
            let increment = Expr::MathOp(
                Box::new(var_expr),
                MathOp::Plus,
                Box::new(Expr::NumberF64(1.0).into()),
            )
                .into();

            return Ok(Expr::ScalarAssign(name, Box::new(increment)).into());
        } else if self.matches_series(&[TokenType::Minus, TokenType::Minus, TokenType::Ident]) {
            let name = self.prior_indent_name_infallible();
            let var = Expr::Variable(name.clone()).into();
            let decrement = Expr::MathOp(
                Box::new(var),
                MathOp::Minus,
                Box::new(Expr::NumberF64(1.0).into()),
            )
                .into();

            return Ok(Expr::ScalarAssign(name, Box::new(decrement)).into());
        }

        self.post_op()
    }

    fn post_op(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut expr = self.column()?;

        if let Expr::Variable(_) = &expr.expr {
            // Check enum variant before cloning it since the clone is expensive
            if let Expr::Variable(name) = expr.expr.clone() {
                if self.matches_series(&[TokenType::Plus, TokenType::Plus]) {
                    let increment = Expr::MathOp(
                        Box::new(expr),
                        MathOp::Plus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    ).into();
                    let assign = Expr::ScalarAssign(name, Box::new(increment)).into();
                    expr = Expr::MathOp(
                        Box::new(assign),
                        MathOp::Minus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    ).into();
                } else if self.matches_series(&[TokenType::Minus, TokenType::Minus]) {
                    let decrement = Expr::MathOp(
                        Box::new(expr),
                        MathOp::Minus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    ).into();
                    let assign = Expr::ScalarAssign(name, Box::new(decrement)).into();
                    expr = Expr::MathOp(
                        Box::new(assign),
                        MathOp::Plus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    ).into();
                }
            } else {
                unreachable!()
            }
        }
        Ok(expr)
    }

    fn column(&mut self) -> Result<TypedExpr, PrintableError> {
        let mut num_cols: usize = 0;
        while self.matches(flags!(TokenType::Column)) {
            num_cols += 1;
        }
        let mut expr = self.primary()?;
        for _ in 0..num_cols {
            // If this isn't a col we loop 0 times and just return primary
            expr = TypedExpr::new(Expr::Column(Box::new(expr)));
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<TypedExpr, PrintableError> {
        if self.is_at_end() {
            return Err(PrintableError::new("Unexpected end of input"));
        }
        Ok(match self.tokens.get(self.current).unwrap().clone() {
            Token::NumberF64(num) => {
                self.advance()?;
                Expr::NumberF64(num).into()
            }
            Token::LeftParen => {
                self.consume(TokenType::LeftParen, "Expected to parse a left paren here")?;
                let expr = self.expression()?;
                self.consume(TokenType::RightParen, "Missing closing ')' after group")?;
                expr.into()
            }
            Token::Ident(name) => {
                self.consume(TokenType::Ident, "Expected to parse an ident here")?;

                if self.matches(flags!(TokenType::LeftBracket)) {
                    self.array_index(name)?
                } else if self.matches(flags!(TokenType::LeftParen)) {
                    self.call(name)?
                } else {
                    Expr::Variable(name).into()
                }
            }
            Token::String(string) => {
                self.consume(TokenType::String, "Expected to parse a string here")?;
                Expr::String(self.symbolizer.get_from_string(string)).into()
            }
            Token::Regex(string) => {
                self.consume(TokenType::Regex, "Expected to parse a string here")?;
                Expr::Regex(self.symbolizer.get_from_string(string)).into()
            }
            t => return Err(PrintableError::new(format!("Unexpected token {:?} {}", t, TokenType::name(t.ttype()))))
        })
    }

    fn call(&mut self, target: Symbol) -> Result<TypedExpr, PrintableError> {
        let mut args = vec![];
        loop {
            if self.matches(flags!(TokenType::RightParen)) {
                break;
            }
            if self.peek().ttype() == TokenType::EOF {
                return Err(PrintableError::new("Hit EOF while parsing function args"));
            }
            args.push(self.expression()?);
            if self.matches(flags!(TokenType::Comma)) {
                continue;
            } else {
                self.consume(
                    TokenType::RightParen,
                    "Expected a right paren ')' after a function call",
                )?;
                break;
            }
        }
        // sub and gsub take lvalue's as args. It's a PITA and handling these two funcs is done
        // separate from the rest of the builtins
        if target == self.sub || target == self.gsub {
            let global = target == self.gsub;
            let call = if args.len() == 2 || args.len() == 3 {
                let arg3 = if args.len() == 3 {
                    match LValue::try_from(args.pop().unwrap().expr) {
                        Ok(lvalue) => lvalue,
                        Err(()) => return Err(PrintableError::new(format!("Argument #3 to the {} builtin function must be an lvalue: a variable `B`, an index into an array `B[1]`, or a column `$3`.", target)))
                    }
                } else {
                    LValue::Column(Box::new(Expr::NumberF64(0.0).into()))
                };
                let arg2 = Box::new(args.pop().unwrap());
                let arg1 = Box::new(args.pop().unwrap());
                Expr::CallSub { arg1, arg2, arg3, global }
            } else {
                return Err(PrintableError::new(format!("Builtin function {} can only be called with 2 or 3 arguments", target)));
            };
            return Ok(call.into());
        }
        Ok(Expr::Call { target, args }.into())
    }

    fn array_index(&mut self, name: Symbol) -> Result<TypedExpr, PrintableError> {
        let mut indices = vec![self.expression()?];
        while self.matches(flags!(TokenType::Comma))
            && self.peek().ttype() != TokenType::RightBracket
        {
            indices.push(self.expression()?);
        }
        self.consume(
            TokenType::RightBracket,
            "Array indexing must end with a right bracket.",
        )?;
        Ok(Expr::ArrayIndex { name, indices }.into())
    }
}

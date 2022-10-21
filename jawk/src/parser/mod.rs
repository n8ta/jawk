mod types;
mod transformer;
mod test;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use crate::lexer::{BinOp, LogicalOp, MathOp, Token, TokenType};
pub use crate::parser::types::PatternAction;
pub use types::{Expr, Function, ScalarType, Stmt, TypedExpr, Arg, ArgT};
use crate::{AnalysisResults, Symbolizer};
use crate::parser::transformer::transform;
use crate::symbolizer::Symbol;

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
}

impl Program {
    #[cfg(test)]
    fn new_action_only(name: Symbol, action: Stmt) -> Program {
        let body = transform(vec![], vec![], vec![PatternAction::new_action_only(action)]);
        let mut functions = HashMap::new();
        functions.insert(name.clone(), Function::new(name, vec![], body));
        Program { functions, global_analysis: AnalysisResults::new() }
    }
    pub fn new(name: Symbol, begins: Vec<Stmt>, ends: Vec<Stmt>, pas: Vec<PatternAction>, parsed_functions: Vec<Function>) -> Program {
        let body = transform(begins, ends, pas);
        let main = Function::new(name.clone(), vec![], body);
        let mut functions = HashMap::new();
        functions.insert(name, main);
        for func in parsed_functions {
            functions.insert(func.name.clone(), func);
        }
        Program { functions, global_analysis: AnalysisResults::new() }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for func in &self.functions {
            write!(f, "{}\n", func.1)?;
        }
        Ok(())
    }
}

const STRING_CONCAT_SKIPS: u64 =
    TokenType::InplaceAssign as u64 |
        TokenType::Less as u64 |
        TokenType::LessEq as u64 |
        TokenType::BangEq as u64 |
        TokenType::EqEq as u64 |
        TokenType::Greater as u64 |
        TokenType::GreaterEq as u64 |
        TokenType::And as u64 |
        TokenType::Or as u64 |
        TokenType::Eq as u64 |
        TokenType::Semicolon as u64 |
        TokenType::RightBrace as u64 |
        TokenType::RightParen as u64 |
        TokenType::LeftBrace as u64 |
        TokenType::Question as u64 |
        TokenType::Colon as u64 |
        TokenType::MatchedBy as u64 |
        TokenType::NotMatchedBy as u64 |
        TokenType::Comma as u64 |
        TokenType::In as u64 |
        TokenType::LeftBracket as u64 |
        TokenType::RightBracket as u64 |
        TokenType::Printf as u64;

pub fn parse(tokens: Vec<Token>, symbolizer: &mut Symbolizer) -> Program {
    let mut parser = Parser { tokens, current: 0, symbolizer };
    parser.parse()
}

struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    symbolizer: &'a mut Symbolizer,
}

macro_rules! flags {
    // Base case:
    ($x:expr) => ($x as u64);
    ($x:expr, $($y:expr),+) => ( flags!($x) | flags!($($y),+))
}

impl<'a> Parser<'a> {
    fn parse(&mut self) -> Program {
        let mut begins = vec![];
        let mut ends = vec![];
        let mut pattern_actions = vec![];
        let mut functions = vec![];
        while !self.is_at_end() {
            if self.matches(flags!(TokenType::Function)) {
                let name = self.ident_consume("Function name must follow function keyword");
                self.consume(TokenType::LeftParen, "Function name must be followed by '('");
                let mut args = vec![];
                loop {
                    if self.peek().ttype() != TokenType::RightParen {
                        args.push(self.ident_consume("Expected function argument name here"));
                    } else {
                        break;
                    }
                    if self.peek().ttype() != TokenType::RightParen {
                        self.consume(TokenType::Comma, "Expected comma after function argument and before right paren");
                        continue;
                    }
                    break;
                }
                self.consume(TokenType::RightParen, "Expected right paren after function arguments");
                let body = self.group();
                functions.push(Function::new(name, args, body))
            } else {
                match self.pattern_action() {
                    PAType::Normal(pa) => pattern_actions.push(pa),
                    PAType::Begin(pa) => begins.push(pa),
                    PAType::End(pa) => ends.push(pa),
                }
            }
        }

        Program::new(self.symbolizer.get("main function"), begins, ends, pattern_actions, functions)
    }

    fn ident_consume(&mut self, error_msg: &str) -> Symbol {
        if let Token::Ident(ident) = self.consume(TokenType::Ident, error_msg) {
            return ident;
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

    fn consume(&mut self, typ: TokenType, message: &str) -> Token {
        if self.check(typ.clone()) {
            return self.advance();
        }
        panic!(
            "{} - didn't find a {} as expected. Found a {} {:?}",
            message,
            TokenType::name(typ),
            TokenType::name(self.peek().ttype()),
            self.peek()
        );
    }


    fn matches(&mut self, tokens: u64) -> bool {
        let tkn = match self.tokens.get(self.current) {
            None => return false,
            Some(t) => t.ttype().clone(),
        };
        if (tokens & tkn as u64) != 0 {
            self.advance();
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

    fn peek_next_next(&self) -> &Token {
        self.peek_at(self.current + 2)
    }

    fn is_at_end(&self) -> bool {
        self.tokens[self.current].ttype() == TokenType::EOF
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous().unwrap()
    }

    fn pattern_action(&mut self) -> PAType {
        let b = if self.matches(flags!(TokenType::LeftBrace)) {
            // { print 1; }
            let pa = PAType::Normal(PatternAction::new_action_only(self.stmts()));
            self.consume(TokenType::RightBrace, "Expected '}' after action block");
            pa
        } else if self.matches(flags!(TokenType::Begin)) {
            // BEGIN { print 1; }
            self.consume(TokenType::LeftBrace, "Expected a '{' after a begin");
            let pa = PAType::Begin(self.stmts());
            self.consume(TokenType::RightBrace, "Begin action should end with '}'");
            pa
        } else if self.matches(flags!(TokenType::End)) {
            // END { print 1; }
            self.consume(TokenType::LeftBrace, "Expected a {' after a end");
            let pa = PAType::End(self.stmts());
            self.consume(TokenType::RightBrace, "End action should end with '}'");
            pa
        } else {
            let test = self.expression();
            if self.matches(flags!(TokenType::LeftBrace)) {
                // test { print 1; }
                let pa = PAType::Normal(PatternAction::new(Some(test), self.stmts()));
                self.consume(TokenType::RightBrace, "Patern action should end with '}'");
                pa
            } else {
                // test
                // ^ implicitly prints line if test passes
                PAType::Normal(PatternAction::new_pattern_only(test))
            }
        };
        b
    }
    fn group(&mut self) -> Stmt {
        self.consume(TokenType::LeftBrace, "Expected a '{' to start group");
        let s = self.stmts();
        self.consume(TokenType::RightBrace, "Expected a '}' to end group");
        s
    }

    fn stmt_and_optional_semicolon(&mut self) -> Stmt {
        let stmt = self.stmt();
        if self.peek().ttype() == TokenType::Semicolon {
            self.consume(TokenType::Semicolon, "not possible");
        }
        stmt
    }

    fn stmt(&mut self) -> Stmt {
        let stmt = if self.matches(flags!(TokenType::Print)) {
            Stmt::Print(self.expression()) // TODO: print 1,2,3
        } else if self.matches(flags!(TokenType::Ret)) {
            if self.peek().ttype() != TokenType::RightBrace && self.peek_next().ttype() != TokenType::Semicolon {
                let expr = self.expression();
                Stmt::Return(Some(expr))
            } else {
                Stmt::Return(None)
            }
        } else if self.matches(flags!(TokenType::Printf)) {
            let fstring = self.expression();
            let mut args = vec![];
            while self.matches(flags!(TokenType::Comma)) {
                args.push(self.expression());
            }
            Stmt::Printf { fstring, args }
        } else if self.matches(flags!(TokenType::Break)) {
            Stmt::Break
        } else if self.matches(flags!(TokenType::For)) {
            self.consume(TokenType::LeftParen, "Expected a '(' after the for keyword");
            let init = self.stmt();
            self.consume(
                TokenType::Semicolon,
                "Expected a ';' after for loop init statement",
            );
            let test = self.expression();
            self.consume(
                TokenType::Semicolon,
                "Expected a ';' after for loop test statement",
            );
            let incr = self.stmt();
            self.consume(TokenType::RightParen, "Expected a ')' to end for loop");
            self.consume(
                TokenType::LeftBrace,
                "Expected a '{' to begin for loop body",
            );
            let body = self.stmts();
            self.consume(TokenType::RightBrace, "Expected a '}' after for loop body");
            Stmt::Group(vec![
                init,
                Stmt::While(test, Box::new(Stmt::Group(vec![body, incr]))),
            ])
        } else if self.peek_next().ttype() == TokenType::Eq {
            let str = if let Token::Ident(str) =
            self.consume(TokenType::Ident, "Expected identifier before '='")
            {
                str
            } else {
                panic!("Expected identifier before '='")
            };
            self.consume(TokenType::Eq, "Expected '=' after identifier");
            Stmt::Expr(TypedExpr::new(Expr::ScalarAssign(
                str,
                Box::new(self.expression()),
            )))
            // } else if self.any_match(&[TokenType::Ret]) {
            //     self.return_stmt()
        } else if self.matches(flags!(TokenType::While)) {
            self.consume(TokenType::LeftParen, "Must have paren after while");
            let expr = self.expression();
            self.consume(
                TokenType::RightParen,
                "Must have right parent after while statement test expression",
            );
            self.consume(TokenType::LeftBrace, "Must have brace after `while (expr)`");
            let stmts = self.stmts();
            self.consume(TokenType::RightBrace, "While loop must be followed by '}'");
            Stmt::While(expr, Box::new(stmts))
        } else if self.matches(flags!(TokenType::Print)) {
            let expr = self.expression();
            Stmt::Print(expr)
        } else if self.matches(flags!(TokenType::If)) {
            self.if_stmt()
        } else if self.matches(flags!(TokenType::LeftBrace)) {
            let s = self.stmts();
            self.consume(
                TokenType::RightBrace,
                "Expected a right brace after a group",
            );
            s
        } else {
            Stmt::Expr(self.expression())
        };
        stmt
    }

    fn stmts(&mut self) -> Stmt {
        let mut stmts = Vec::with_capacity(5);
        while self.peek().ttype() != TokenType::RightBrace {
            let stmt = self.stmt_and_optional_semicolon();
            stmts.push(stmt);
        }
        if stmts.len() == 1 {
            return stmts.pop().unwrap();
        }
        Stmt::Group(stmts)
    }

    fn if_stmt(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' after if");
        let predicate = self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after if predicate");

        let then_blk = if self.peek().ttype() == TokenType::LeftBrace {
            self.group()
        } else {
            self.stmt()
        };

        let else_blk = if self.matches(flags!(TokenType::Else)) {
            let else_blk = if self.peek().ttype() == TokenType::LeftBrace {
                self.group()
            } else {
                self.stmt()
            };
            Some(Box::new(else_blk))
        } else {
            None
        };
        Stmt::If(predicate, Box::new(then_blk), else_blk)
    }

    fn expression(&mut self) -> TypedExpr {
        self.assignment()
    }

    fn assignment(&mut self) -> TypedExpr {
        let expr = self.ternary();
        if let Expr::Variable(var) = &expr.expr {
            let var = var.clone();
            if self.matches(flags!(TokenType::Eq)) {
                // =
                return TypedExpr::new(Expr::ScalarAssign(var, Box::new(self.assignment())));
            } else if self.matches(flags!(TokenType::InplaceAssign)) {
                // ?=
                let math_op = if let Token::InplaceEq(math_op) = self.previous().unwrap() { math_op } else { unreachable!() };
                let expr = Expr::MathOp(
                    Box::new(Expr::Variable(var.clone()).into()),
                    math_op,
                    Box::new(self.assignment()),
                );
                return Expr::ScalarAssign(
                    var,
                    Box::new(expr.into())).into();
            }
        }
        let mut is_array_index = false;
        if let Expr::ArrayIndex { .. } = &expr.expr {
            is_array_index = true;
        }
        if is_array_index && self.matches(flags!(TokenType::Eq)) {
            if let Expr::ArrayIndex { name, indices } = expr.expr {
                let value = Box::new(self.assignment());
                return Expr::ArrayAssign { name, indices, value }.into();
            } else {
                unreachable!()
            }
        }
        expr
    }

    fn ternary(&mut self) -> TypedExpr {
        let cond = self.logical_or();
        while self.matches(flags!(TokenType::Question)) {
            let expr1 = self.ternary();
            self.consume(TokenType::Colon, "Expected a colon after question mark in a ternary!");
            let expr2 = self.ternary();
            return TypedExpr::new(Expr::Ternary(
                Box::new(cond),
                Box::new(expr1),
                Box::new(expr2),
            ));
        }
        cond
    }

    fn logical_or(&mut self) -> TypedExpr {
        let mut expr = self.logical_and();
        while self.matches(flags!(TokenType::Or)) {
            expr = TypedExpr::new(Expr::LogicalOp(
                Box::new(expr),
                LogicalOp::Or,
                Box::new(self.logical_and()),
            ))
        }
        expr
    }

    fn logical_and(&mut self) -> TypedExpr {
        let mut expr = self.array_membership();
        while self.matches(flags!(TokenType::And)) {
            expr = TypedExpr::new(Expr::LogicalOp(
                Box::new(expr),
                LogicalOp::And,
                Box::new(self.array_membership()),
            ))
        }
        expr
    }

    fn array_membership(&mut self) -> TypedExpr {
        // <expr> in array_name
        let mut expr = self.multi_dim_array_membership();
        while self.matches(flags!(TokenType::In)) {
            let name = if let Token::Ident(name) = self.consume(TokenType::Ident, "An array name must follow `<expr> in`") { name } else { unreachable!() };
            expr = Expr::InArray { name, indices: vec![expr] }.into()
        }
        expr
    }

    fn helper_multi_dim_array(&mut self) -> TypedExpr {
        self.consume(TokenType::LeftParen, "Multidimensional array must begin with left paren");
        let mut exprs = vec![self.regex()];
        while self.matches(flags!(TokenType::Comma)) {
            if self.peek().ttype() == TokenType::RightParen { break; }
            exprs.push(self.regex());
        }
        self.consume(TokenType::RightParen, "Multidimensional array indices must end with right paren");
        self.consume(TokenType::In, "Multidimensional array access must be followed by an 'in'");
        let ident = self.consume(TokenType::Ident, "Multidimensional array access must be followed by an array name. Eg: (1,2,3) in ARRAY_NAME");
        let ident = if let Token::Ident(ident) = ident { ident } else { unreachable!("compiler bug consumed ident but got something else") };

        let mut expr = TypedExpr::new(Expr::InArray { name: ident, indices: exprs });
        while self.matches(flags!(TokenType::In)) {
            let ident = self.consume(TokenType::Ident, "Multidimensional array access must be followed by an array name. Eg: (1,2,3) in ARRAY_NAME");
            let ident = if let Token::Ident(ident) = ident { ident } else { unreachable!("compiler bug consumed ident but got something else") };
            expr = Expr::InArray { name: ident, indices: vec![expr.into()] }.into();
        }
        expr
    }

    fn multi_dim_array_membership(&mut self) -> TypedExpr {
        let mut idx = self.current;

        // Check if we match the regex \(.+\) in if so call the helper
        if *self.peek_at(idx) == Token::LeftParen {
            while *self.peek_at(idx) != Token::RightParen { idx += 1; }
            if *self.peek_at(idx) == Token::RightParen && *self.peek_at(idx + 1) == Token::In {
                return self.helper_multi_dim_array();
            }
        }
        self.regex()
    }

    fn regex(&mut self) -> TypedExpr {
        // "a ~ /match/"
        let mut expr = self.compare();
        while self.matches(flags!(TokenType::MatchedBy, TokenType::NotMatchedBy)) {
            expr = Expr::BinOp(
                Box::new(expr),
                if self.previous().unwrap().ttype() == TokenType::MatchedBy { BinOp::MatchedBy } else { BinOp::NotMatchedBy },
                Box::new(self.compare())).into();
        }
        expr
    }

    fn compare(&mut self) -> TypedExpr {
        let mut expr = self.string_concat();
        while self.matches(flags!(TokenType::GreaterEq, TokenType::Greater, TokenType::Less, TokenType::LessEq, TokenType::EqEq, TokenType::BangEq)) {
            let op = match self.previous().unwrap() {
                Token::BinOp(BinOp::Less) => BinOp::Less,
                Token::BinOp(BinOp::LessEq) => BinOp::LessEq,
                Token::BinOp(BinOp::Greater) => BinOp::Greater,
                Token::BinOp(BinOp::GreaterEq) => BinOp::GreaterEq,
                Token::BinOp(BinOp::BangEq) => BinOp::BangEq,
                Token::BinOp(BinOp::EqEq) => BinOp::EqEq,
                _ => unreachable!("Parser bug in compare matches function"),
            };
            expr = Expr::BinOp(Box::new(expr), op, Box::new(self.string_concat())).into()
        }
        expr
    }

    #[inline(always)]
    fn types_contain(bitflag_union: u64, flag: u64) -> bool {
        bitflag_union & flag != 0
    }

    fn string_concat(&mut self) -> TypedExpr {
        let mut expr = self.plus_minus();
        while !self.is_at_end() &&
            !Parser::types_contain(STRING_CONCAT_SKIPS, self.peek().ttype() as u64) {
            if let Expr::Concatenation(vals) = &mut expr.expr {
                vals.push(self.plus_minus());
            } else {
                expr = TypedExpr::new(Expr::Concatenation(vec![expr, self.plus_minus()]));
            }
        }
        expr
    }

    fn plus_minus(&mut self) -> TypedExpr {
        let mut expr = self.term();
        while self.matches(flags!(TokenType::Plus, TokenType::Minus)) {
            let op = match self.previous().unwrap() {
                Token::MathOp(MathOp::Minus) => MathOp::Minus,
                Token::MathOp(MathOp::Plus) => MathOp::Plus,
                _ => unreachable!("Parser bug in comparison function"),
            };
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.plus_minus())).into();
        }
        expr
    }

    fn term(&mut self) -> TypedExpr {
        let mut expr = self.unary();
        while self.matches(flags!(TokenType::Star, TokenType::Slash, TokenType::Modulo)) {
            let op = match self.previous().unwrap() {
                Token::MathOp(MathOp::Star) => MathOp::Star,
                Token::MathOp(MathOp::Slash) => MathOp::Slash,
                Token::MathOp(MathOp::Modulus) => MathOp::Modulus,
                _ => unreachable!("Parser bug in comparison function"),
            };
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.unary())).into()
        }
        expr
    }

    fn unary(&mut self) -> TypedExpr {
        if !(self.peek().ttype() == TokenType::Minus
            && self.peek_next().ttype() == TokenType::Minus)
            && !(self.peek().ttype() == TokenType::Plus
            && self.peek_next().ttype() == TokenType::Plus)
            && self.matches(flags!(TokenType::Minus, TokenType::Plus, TokenType::Bang))
        {
            let p = self.previous().unwrap().ttype();
            let rhs = self.unary();
            let one = TypedExpr::new(Expr::NumberF64(1.0));
            let zero = TypedExpr::new(Expr::NumberF64(0.0));
            return match p {
                TokenType::Bang => Expr::BinOp(Box::new(one), BinOp::BangEq, Box::new(rhs)),
                TokenType::Plus => Expr::MathOp(Box::new(zero), MathOp::Plus, Box::new(rhs)),
                TokenType::Minus => Expr::MathOp(Box::new(zero), MathOp::Minus, Box::new(rhs)),
                _ => unreachable!(),
            }
                .into();
        }
        self.exp()
    }

    fn exp(&mut self) -> TypedExpr {
        let mut expr = self.pre_op();
        while self.matches(flags!(TokenType::Exponent)) {
            let op = MathOp::Exponent;
            expr = Expr::MathOp(Box::new(expr), op, Box::new(self.pre_op())).into()
        }
        expr
    }

    fn pre_op(&mut self) -> TypedExpr {
        if self.peek().ttype() == TokenType::Plus
            && self.peek_next().ttype() == TokenType::Plus
            && self.peek_next_next().ttype() == TokenType::Ident
        {
            self.advance();
            self.advance();
            self.advance();

            if let Token::Ident(name) = self.previous().unwrap() {
                let var_expr = Expr::Variable(name.clone()).into();
                let increment = Expr::MathOp(
                    Box::new(var_expr),
                    MathOp::Plus,
                    Box::new(Expr::NumberF64(1.0).into()),
                ).into();

                return Expr::ScalarAssign(name, Box::new(increment)).into();
            }
        } else if self.peek().ttype() == TokenType::Minus
            && self.peek_next().ttype() == TokenType::Minus
            && self.peek_next_next().ttype() == TokenType::Ident
        {
            self.advance();
            self.advance();
            self.advance();

            if let Token::Ident(name) = self.previous().unwrap() {
                let var = Expr::Variable(name.clone()).into();
                let decrement = Expr::MathOp(
                    Box::new(var),
                    MathOp::Minus,
                    Box::new(Expr::NumberF64(1.0).into()),
                ).into();

                return Expr::ScalarAssign(name, Box::new(decrement)).into();
            }
        }

        self.post_op()
    }

    fn post_op(&mut self) -> TypedExpr {
        let mut expr = self.column();

        if let Expr::Variable(_) = &expr.expr {
            // Check enum variant before cloning it since the clone is expensive
            if let Expr::Variable(name) = expr.expr.clone() {
                if self.peek().ttype() == TokenType::Plus && self.peek_next().ttype() == TokenType::Plus
                {
                    self.advance();
                    self.advance();
                    let increment = Expr::MathOp(
                        Box::new(expr),
                        MathOp::Plus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    )
                        .into();
                    let assign = Expr::ScalarAssign(name, Box::new(increment)).into();
                    expr = Expr::MathOp(
                        Box::new(assign),
                        MathOp::Minus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    )
                        .into();
                } else if self.peek().ttype() == TokenType::Minus
                    && self.peek_next().ttype() == TokenType::Minus
                {
                    self.advance();
                    self.advance();
                    let decrement = Expr::MathOp(
                        Box::new(expr),
                        MathOp::Minus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    )
                        .into();
                    let assign = Expr::ScalarAssign(name, Box::new(decrement)).into();
                    expr = Expr::MathOp(
                        Box::new(assign),
                        MathOp::Plus,
                        Box::new(Expr::NumberF64(1.0).into()),
                    )
                        .into();
                }
            } else { unreachable!()}
        }

        expr
    }

    fn column(&mut self) -> TypedExpr {
        let mut num_cols: usize = 0;
        while self.matches(flags!(TokenType::Column)) {
            num_cols += 1;
        }
        let mut expr = self.primary();
        for _ in 0..num_cols {
            // If this isn't a col we loop 0 times and just return primary
            expr = TypedExpr::new(Expr::Column(Box::new(expr)));
        }

        expr
    }

    fn primary(&mut self) -> TypedExpr {
        if self.is_at_end() {
            panic!("Unexpected end of input")
        }
        match self.tokens.get(self.current).unwrap().clone() {
            Token::NumberF64(num) => {
                self.advance();
                Expr::NumberF64(num).into()
            }
            Token::LeftParen => {
                self.consume(TokenType::LeftParen, "Expected to parse a left paren here");
                let expr = self.expression();
                self.consume(TokenType::RightParen, "Missing closing ')' after group");
                expr.into()
            }
            Token::Ident(name) => {
                self.consume(TokenType::Ident, "Expected to parse an ident here");

                if self.matches(flags!(TokenType::LeftBracket)) {
                    self.array_index(name)
                } else if self.matches(flags!(TokenType::LeftParen)) {
                    self.call(name)
                } else {
                    Expr::Variable(name).into()
                }
            }
            Token::String(string) => {
                self.consume(TokenType::String, "Expected to parse a string here");
                Expr::String(self.symbolizer.get(string)).into()
            }
            Token::Regex(string) => {
                self.consume(TokenType::Regex, "Expected to parse a string here");
                Expr::Regex(self.symbolizer.get(string)).into()
            }
            t => panic!("Unexpected token {:?} {}", t, TokenType::name(t.ttype())),
        }
    }

    fn call(&mut self, target: Symbol) -> TypedExpr {
        let mut args = vec![];
        loop {
            if self.matches(flags!(TokenType::RightParen)) {
                break;
            }
            if self.peek().ttype() == TokenType::EOF {
                panic!("Hit EOF while parsing function args")
            }
            args.push(self.expression());
            if self.matches(flags!(TokenType::Comma)) {
                continue;
            } else {
                self.consume(TokenType::RightParen, "Expected a right paren ')' after a function call");
                break;
            }
        }
        Expr::Call { target, args }.into()
    }

    fn array_index(&mut self, name: Symbol) -> TypedExpr {
        let mut indices = vec![self.expression()];
        while self.matches(flags!(TokenType::Comma)) && self.peek().ttype() != TokenType::RightBracket {
            indices.push(self.expression());
        }
        self.consume(TokenType::RightBracket, "Array indexing must end with a right bracket.");
        Expr::ArrayIndex { name, indices }.into()
    }
}
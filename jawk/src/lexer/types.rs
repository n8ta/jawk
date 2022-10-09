use std::fmt::{Display, Formatter};
use crate::symbolizer::Symbol;

#[derive(Debug, Clone, PartialOrd, PartialEq, Copy)]
pub enum MathOp {
    Minus,
    Plus,
    Slash,
    Star,
    Modulus,
    Exponent,
}

#[repr(i8)]
#[derive(Debug, Clone, PartialOrd, PartialEq, Copy)]
pub enum BinOp {
    Greater,
    GreaterEq,
    Less,
    LessEq,
    BangEq,
    EqEq,
    MatchedBy,
    NotMatchedBy,
}

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Greater => f.write_str(">"),
            BinOp::GreaterEq => f.write_str(">="),
            BinOp::Less => f.write_str("<"),
            BinOp::LessEq => f.write_str("<="),
            BinOp::BangEq => f.write_str("!="),
            BinOp::EqEq => f.write_str("=="),
            BinOp::MatchedBy => f.write_str("~"),
            BinOp::NotMatchedBy => f.write_str("!~"),
        }
    }
}

impl Display for MathOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MathOp::Minus => f.write_str("-"),
            MathOp::Plus => f.write_str("+"),
            MathOp::Slash => f.write_str("/"),
            MathOp::Star => f.write_str("*"),
            MathOp::Modulus => f.write_str("%"),
            MathOp::Exponent => f.write_str("^"),
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Copy)]
pub enum LogicalOp {
    And,
    Or,
}

impl Display for LogicalOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicalOp::And => f.write_str("&&"),
            LogicalOp::Or => f.write_str("||"),
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Token {
    Eq,
    Semicolon,
    Column,
    Function,
    BinOp(BinOp),
    // < <= >= >
    MathOp(MathOp),
    // + - ^ %
    LogicalOp(LogicalOp),
    // && ||
    Bang,
    String(String),
    Ident(Symbol),
    NumberF64(f64),
    False,
    True,
    EOF,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    Regex(String),
    Print,
    Printf,
    Ret,
    If,
    Begin,
    End,
    Else,
    While,
    For,
    Do,
    InplaceEq(MathOp),
    Question,
    Colon,
    LeftBracket,
    RightBracket,
    In,
    Comma,
    Break,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Hash, Eq, Ord)]
pub enum TokenType {
    Minus,
    Plus,
    Slash,
    Star,
    Modulo,
    Exponent,
    Bang,
    BangEq,
    EqEq,
    Greater,
    GreaterEq,
    Ident,
    Less,
    LessEq,
    String,
    NumberF64,
    And,
    Or,
    False,
    True,
    EOF,
    Column,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    Print,
    Printf,
    Semicolon,
    Eq,
    Ret,
    If,
    Begin,
    End,
    Else,
    For,
    While,
    Do,
    MatchedBy,
    NotMatchedBy,
    InplaceAssign,
    Question,
    Colon,
    Regex,
    LeftBracket,
    RightBracket,
    In,
    Comma,
    Function,
    Break,
}

impl Token {
    pub fn ttype(&self) -> TokenType {
        // Match statement mapping every single ttype to its id
        match self {
            Token::Function => TokenType::Function,
            Token::BinOp(bin_op) => match bin_op {
                BinOp::Greater => TokenType::Greater,
                BinOp::GreaterEq => TokenType::GreaterEq,
                BinOp::Less => TokenType::Less,
                BinOp::LessEq => TokenType::LessEq,
                BinOp::BangEq => TokenType::BangEq,
                BinOp::EqEq => TokenType::EqEq,
                BinOp::MatchedBy => TokenType::MatchedBy,
                BinOp::NotMatchedBy => TokenType::NotMatchedBy,
            },
            Token::InplaceEq(_math_op) => TokenType::InplaceAssign,
            Token::MathOp(math_op) => match math_op {
                MathOp::Minus => TokenType::Minus,
                MathOp::Plus => TokenType::Plus,
                MathOp::Slash => TokenType::Slash,
                MathOp::Star => TokenType::Star,
                MathOp::Modulus => TokenType::Modulo,
                MathOp::Exponent => TokenType::Exponent,
            },
            Token::LogicalOp(logical_op) => match logical_op {
                LogicalOp::And => TokenType::And,
                LogicalOp::Or => TokenType::Or,
            },
            Token::Bang => TokenType::Bang,
            Token::String(_) => TokenType::String,
            Token::NumberF64(_) => TokenType::NumberF64,
            Token::False => TokenType::False,
            Token::True => TokenType::True,
            Token::EOF => TokenType::EOF,
            Token::Column => TokenType::Column,
            Token::LeftBrace => TokenType::LeftBrace,
            Token::RightBrace => TokenType::RightBrace,
            Token::LeftParen => TokenType::LeftParen,
            Token::RightParen => TokenType::RightParen,
            Token::Print => TokenType::Print,
            Token::Semicolon => TokenType::Semicolon,
            Token::Eq => TokenType::Eq,
            Token::Ret => TokenType::Ret,
            Token::If => TokenType::If,
            Token::Else => TokenType::Else,
            Token::End => TokenType::End,
            Token::Begin => TokenType::Begin,
            Token::Ident(_) => TokenType::Ident,
            Token::While => TokenType::While,
            Token::For => TokenType::For,
            Token::Do => TokenType::Do,
            Token::Question => TokenType::Question,
            Token::Colon => TokenType::Colon,
            Token::Regex(_) => TokenType::Regex,
            Token::LeftBracket => TokenType::LeftBracket,
            Token::RightBracket => TokenType::RightBracket,
            Token::In => TokenType::In,
            Token::Comma => TokenType::Comma,
            Token::Break => TokenType::Break,
            Token::Printf => TokenType::Printf,
        }
    }
}

impl TokenType {
    pub fn name(token_type: TokenType) -> &'static str {
        match token_type {
            TokenType::Function => "function",
            TokenType::While => "while",
            TokenType::Minus => "-",
            TokenType::Plus => "+",
            TokenType::Slash => "/",
            TokenType::Star => "*",
            TokenType::Bang => "!",
            TokenType::EqEq => "==",
            TokenType::Greater => ">",
            TokenType::GreaterEq => ">=",
            TokenType::Less => "<",
            TokenType::LessEq => "<=",
            TokenType::String => "string",
            TokenType::NumberF64 => "f64",
            TokenType::And => "and",
            TokenType::Or => "or",
            TokenType::False => "false",
            TokenType::True => "true",
            TokenType::EOF => "EOF",
            TokenType::BangEq => "!=",
            TokenType::Column => "$",
            TokenType::LeftBrace => "{",
            TokenType::RightBrace => "}",
            TokenType::LeftParen => "(",
            TokenType::RightParen => ")",
            TokenType::Print => "Print",
            TokenType::Semicolon => ";",
            TokenType::Eq => "=",
            TokenType::Ret => "return",
            TokenType::If => "if",
            TokenType::Else => "else",
            TokenType::Begin => "BEGIN",
            TokenType::End => "END",
            TokenType::Ident => "ident",
            TokenType::For => "for",
            TokenType::Do => "do",
            TokenType::Question => "?",
            TokenType::Colon => ":",
            TokenType::MatchedBy => "~",
            TokenType::NotMatchedBy => "~!",
            TokenType::Modulo => "%",
            TokenType::Exponent => "^",
            TokenType::InplaceAssign => "?=",
            TokenType::Regex => "~/match/",
            TokenType::LeftBracket => "[",
            TokenType::RightBracket => "]",
            TokenType::In => "in",
            TokenType::Comma => ",",
            TokenType::Break => "break",
            TokenType::Printf => "printf",
        }
    }
}

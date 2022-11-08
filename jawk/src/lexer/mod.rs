mod types;

use std::iter::Peekable;
use std::str::Chars;
pub use types::{BinOp, LogicalOp, MathOp, Token, TokenType};
use crate::Symbolizer;

pub fn lex(str: &str, symbolizer: &mut Symbolizer) -> LexerResult {
    let mut lexer = Lexer::new(str, symbolizer);
    lexer.scan_tokens()?;
    Ok(lexer.tokens)
}

#[cfg(test)]
fn lex_test(str: &str, symbolizer: &mut Symbolizer) -> LexerResult {
    let mut lexer = Lexer::new(str, symbolizer);
    lexer.scan_tokens()?;
    Ok(lexer.tokens)
}

struct Lexer<'a, 'b> {
    src: Peekable<Chars<'a>>,
    buffer: String,
    line: usize,
    tokens: Vec<Token>,
    symbolizer: &'b mut Symbolizer
}

type LexerResult = Result<Vec<Token>, (String, usize)>;

impl<'a, 'b> Lexer<'a, 'b> {
    fn new(src: &'a str, symbolizer: &'b mut Symbolizer) -> Lexer<'a, 'b> {
        Lexer {
            src: src.chars().peekable(),
            line: 0,
            tokens: Vec::with_capacity(1000),
            buffer: String::with_capacity(30),
            symbolizer
        }
    }
    fn is_at_end(&mut self) -> bool {
        !self.src.peek().is_some()
    }
    fn collect_buffer(&mut self) -> String {
        let mut string: String = String::new();
        std::mem::swap(&mut self.buffer, &mut string);
        string
    }
    fn advance(&mut self) -> char {
        let next = self.src.next().unwrap();
        self.buffer.push(next);
        next
    }
    fn add_token(&mut self, tt: Token) {
        self.tokens.push(tt);
    }
    fn string(&mut self) -> Result<(), String> {
        self.buffer.clear();
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
        if self.is_at_end() {
            let string = self.collect_buffer();
            return Err(format!("Unterminated String: {}", string));
        }
        let str = self.collect_buffer();
        self.advance();
        self.add_token(Token::String(str));
        return Ok(());
    }
    fn regex(&mut self) -> Result<(), String> {
        // a ~ b 
        // a ~ /match/'
        self.buffer.clear();
        while self.peek() != '/' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            let string = self.collect_buffer();
            return Err(format!("Unterminated regex: {}", string));
        }
        let regex = self.collect_buffer();
        self.advance();
        self.add_token(Token::Regex(regex));
        return Ok(());
    }
    fn number(&mut self) -> Result<Token, String> {
        while self.peek().is_digit(10) {
            self.advance();
        }
        if self.matches('.') {}
        while self.peek().is_digit(10) {
            self.advance();
        }

        let num = self.collect_buffer();
        // TODO: scientific notation
        match num.parse::<f64>() {
            Ok(float) => Ok(Token::NumberF64(float)),
            Err(_) => {
                return Err(format!("Unable to parse f64 {}", num));
            }
        }
    }

    fn identifier(&mut self) -> Result<(), String> {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }
        let src: String = self.collect_buffer();
        let src = src.to_ascii_lowercase();
        if src == "true" {
            self.add_token(Token::True);
        } else if src == "false" {
            self.add_token(Token::False);
        } else if src == "return" {
            self.add_token(Token::Ret);
        } else if src == "function" {
            self.add_token(Token::Function);
        } else if src == "if" {
            self.add_token(Token::If);
        } else if src == "else" {
            self.add_token(Token::Else);
        } else if src == "begin" {
            self.add_token(Token::Begin);
        } else if src == "for" {
            self.add_token(Token::For);
        } else if src == "while" {
            self.add_token(Token::While);
        } else if src == "do" {
            self.add_token(Token::Do);
        } else if src == "end" {
            self.add_token(Token::End);
        } else if src == "print" {
            self.add_token(Token::Print);
        } else if src == "in" {
            self.add_token(Token::In);
        } else if src == "break" {
            self.add_token(Token::Break);
        } else if src == "printf" {
            self.add_token(Token::Printf);
        } else {
            let ident = self.symbolizer.get(src);
            self.add_token(Token::Ident(ident));
        }
        Ok(())
    }
    fn peek(&mut self) -> char {
        match self.src.peek() {
            None => 0x0 as char,
            Some(c) => {
                *c
            },
        }
    }
    fn scan_token(&mut self) -> Result<(), String> {
        let c = self.advance();
        match c {
            '$' => self.add_token(Token::Column),
            '-' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Minus)); // -=
                } else {
                    self.add_token(Token::MathOp(MathOp::Minus)) // -
                }
            }
            '+' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Plus));
                } else {
                    self.add_token(Token::MathOp(MathOp::Plus))
                }
            }
            // ';' => self.add_token(Token::Semicolon),
            '*' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Star));
                } else {
                    self.add_token(Token::MathOp(MathOp::Star))
                }
            }
            '!' => {
                if self.matches('~') {
                    self.add_token(Token::BinOp(BinOp::NotMatchedBy));
                    self.whitespaces();
                    self.collect_buffer();
                    if self.matches('/') {
                        self.regex()?;
                    }
                } else {
                    let token = match self.matches('=') {
                        true => Token::BinOp(BinOp::BangEq),
                        false => Token::Bang
                    };
                    self.add_token(token);
                };
                
            }
            '|' => {
                let tt = match self.matches('|') {
                    true => Token::LogicalOp(LogicalOp::Or),
                    false => return Err("| must be followed by ||".to_string()),
                };
                self.add_token(tt);
            }
            '&' => {
                let tt = match self.matches('&') {
                    true => Token::LogicalOp(LogicalOp::And),
                    false => return Err("| must be followed by &".to_string()),
                };
                self.add_token(tt);
            }
            '=' => {
                let tt = match self.matches('=') {
                    true => Token::BinOp(BinOp::EqEq),
                    false => Token::Eq,
                };
                self.add_token(tt)
            }
            '<' => {
                let tt = match self.matches('=') {
                    true => Token::BinOp(BinOp::LessEq),
                    false => Token::BinOp(BinOp::Less),
                };
                self.add_token(tt)
            }
            '>' => {
                let tt = match self.matches('=') {
                    true => Token::BinOp(BinOp::GreaterEq),
                    false => Token::BinOp(BinOp::Greater),
                };
                self.add_token(tt)
            }
            '%' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Modulus));
                } else {
                    self.add_token(Token::MathOp(MathOp::Modulus));
                }
            }
            '^' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Exponent));
                } else {
                    self.add_token(Token::MathOp(MathOp::Exponent));
                }
            }
            '/' => {
                if self.matches('=') {
                    self.add_token(Token::InplaceEq(MathOp::Slash));
                } else if self.matches('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(Token::MathOp(MathOp::Slash));
                }
            }
            '~' => {
                self.add_token(Token::BinOp(BinOp::MatchedBy));
                self.whitespaces();
                if self.matches('/') {
                    self.regex()?;
                }
            }
            '?' => self.add_token(Token::Question),
            ':' => self.add_token(Token::Colon),
            '{' => self.add_token(Token::LeftBrace),
            '}' => self.add_token(Token::RightBrace),
            '[' => self.add_token(Token::LeftBracket),
            ']' => self.add_token(Token::RightBracket),
            ',' => self.add_token(Token::Comma),
            '(' => self.add_token(Token::LeftParen),
            ')' => self.add_token(Token::RightParen),
            ';' => self.add_token(Token::Semicolon),
            '"' => self.string()?,
            '\r' => (),
            '\t' => (),
            ' ' => (),
            '\n' => self.line += 1,
            _ => {
                if c.is_digit(10) {
                    let num = self.number()?;
                    self.add_token(num);
                } else if c.is_alphabetic() {
                    self.identifier()?;
                } else {
                    return Err(format!("Unexpected token: `{}`", c));
                }
            }
        }
        Ok(())
    }

    fn whitespaces(&mut self) {
        while self.matches(' ') {}
    } 

    fn matches(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        match self.src.peek() {
            None => false,
            Some(char) => {
                if *char == expected {
                    self.buffer.push(*char);
                    self.src.next();
                    return true;
                }
                return false;
            }
        }
    }

    fn scan_tokens(&mut self) -> LexerResult {
        while !self.is_at_end() {
            if let Err(x) = self.scan_token() {
                return Err((x, self.line));
            }
            self.buffer.clear();
        }
        self.tokens.push(Token::EOF);
        Ok(self.tokens.clone())
    }
}

#[test]
fn test_braces() {
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test("{ } ( ) (( )) {{ }}", &mut symbolizer).unwrap(),
        vec![
            Token::LeftBrace,
            Token::RightBrace,
            Token::LeftParen,
            Token::RightParen,
            Token::LeftParen,
            Token::LeftParen,
            Token::RightParen,
            Token::RightParen,
            Token::LeftBrace,
            Token::LeftBrace,
            Token::RightBrace,
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_eq_eq() {
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test("== 2.2", &mut symbolizer).unwrap(),
        vec![Token::BinOp(BinOp::EqEq), Token::NumberF64(2.2), Token::EOF]
    )
}

#[test]
fn test_column_simple() {
    let mut symbolizer = Symbolizer::new();
    let str = "$1";
    let tokens = lex_test(str, &mut symbolizer).unwrap();
    assert_eq!(
        tokens,
        vec![Token::Column, Token::NumberF64(1.0), Token::EOF]
    );
}

#[test]
fn test_columns() {
    let mut symbolizer = Symbolizer::new();
    let str = "$1 + $2000 $0";
    let tokens = lex_test(str, &mut symbolizer).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Column,
            Token::NumberF64(1.0),
            Token::MathOp(MathOp::Plus),
            Token::Column,
            Token::NumberF64(2000.0),
            Token::Column,
            Token::NumberF64(0.0),
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_binops_and_true_false() {
    let mut symbolizer = Symbolizer::new();
    let str = "4*2+1-2+false/true";
    let tokens = lex_test(str, &mut symbolizer).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::NumberF64(4.0),
            Token::MathOp(MathOp::Star),
            Token::NumberF64(2.0),
            Token::MathOp(MathOp::Plus),
            Token::NumberF64(1.0),
            Token::MathOp(MathOp::Minus),
            Token::NumberF64(2.0),
            Token::MathOp(MathOp::Plus),
            Token::False,
            Token::MathOp(MathOp::Slash),
            Token::True,
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_decimals() {
    let mut symbolizer = Symbolizer::new();
    let str = "4.123-123.123";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::NumberF64(4.123),
            Token::MathOp(MathOp::Minus),
            Token::NumberF64(123.123),
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_equality() {
    let mut symbolizer = Symbolizer::new();
    let str = "4 != 5 == 6";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::NumberF64(4.0),
            Token::BinOp(BinOp::BangEq),
            Token::NumberF64(5.0),
            Token::BinOp(BinOp::EqEq),
            Token::NumberF64(6.0),
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_logical_op() {
    let mut symbolizer = Symbolizer::new();
    let str = "4 && 5 || 6";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::NumberF64(4.0),
            Token::LogicalOp(LogicalOp::And),
            Token::NumberF64(5.0),
            Token::LogicalOp(LogicalOp::Or),
            Token::NumberF64(6.0),
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_assignment() {
    let mut symbolizer = Symbolizer::new();
    let str = "abc = 4";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Ident(symbolizer.get("abc")),
            Token::Eq,
            Token::NumberF64(4.0),
            Token::EOF
        ]
    );
}

#[test]
fn test_ret() {
    let mut symbolizer = Symbolizer::new();
    let str = "return 1 return abc";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Ret,
            Token::NumberF64(1.0),
            Token::Ret,
            Token::Ident(symbolizer.get("abc")),
            Token::EOF
        ]
    );
}

#[test]
fn test_if_else() {
    let mut symbolizer = Symbolizer::new();
    let str = "if (1) { 2 } else { 3 }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::If,
            Token::LeftParen,
            Token::NumberF64(1.0),
            Token::RightParen,
            Token::LeftBrace,
            Token::NumberF64(2.0),
            Token::RightBrace,
            Token::Else,
            Token::LeftBrace,
            Token::NumberF64(3.0),
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_if_only() {
    let mut symbolizer = Symbolizer::new();
    let str = "if (1) { 2 }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::If,
            Token::LeftParen,
            Token::NumberF64(1.0),
            Token::RightParen,
            Token::LeftBrace,
            Token::NumberF64(2.0),
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_begin_end() {
    let mut symbolizer = Symbolizer::new();
    let str = "BEGIN begin END end";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Begin,
            Token::Begin,
            Token::End,
            Token::End,
            Token::EOF
        ]
    );
}

#[test]
fn test_ident() {
    let mut symbolizer = Symbolizer::new();
    let str = "{ x }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::LeftBrace,
            Token::Ident(symbolizer.get("x")),
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_string() {
    let mut symbolizer = Symbolizer::new();
    let str = "{ \"x\" }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::LeftBrace,
            Token::String("x".to_string()),
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_string_2() {
    let mut symbolizer = Symbolizer::new();
    let str = "{ \"abc123 444\" }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::LeftBrace,
            Token::String("abc123 444".to_string()),
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_while_l00p() {
    let mut symbolizer = Symbolizer::new();
    let str = " while ( x ) { }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::While,
            Token::LeftParen,
            Token::Ident(symbolizer.get("x")),
            Token::RightParen,
            Token::LeftBrace,
            Token::RightBrace,
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_do_while_l00p() {
    let mut symbolizer = Symbolizer::new();
    let str = " do print 1 while (132)";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Do,
            Token::Print,
            Token::NumberF64(1.0),
            Token::While,
            Token::LeftParen,
            Token::NumberF64(132.0),
            Token::RightParen,
            Token::EOF
        ]
    );
}

#[test]
fn test_lex_for_l00p() {
    let mut symbolizer = Symbolizer::new();
    let str = "for (a = 0;";
    let a = Token::Ident(symbolizer.get("a"));
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::For,
            Token::LeftParen,
            a.clone(),
            Token::Eq,
            Token::NumberF64(0.0),
            Token::Semicolon,
            Token::EOF
        ]
    );
}

#[test]
fn test_lt_gt_eq() {
    let mut symbolizer = Symbolizer::new();
    let str = "< <= >= >";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::BinOp(BinOp::Less),
            Token::BinOp(BinOp::LessEq),
            Token::BinOp(BinOp::GreaterEq),
            Token::BinOp(BinOp::Greater),
            Token::EOF
        ]
    );
}

#[test]
fn test_op_eq() {
    let str = "^= %= *= /= += -=";
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::InplaceEq(MathOp::Exponent),
            Token::InplaceEq(MathOp::Modulus),
            Token::InplaceEq(MathOp::Star),
            Token::InplaceEq(MathOp::Slash),
            Token::InplaceEq(MathOp::Plus),
            Token::InplaceEq(MathOp::Minus),
            Token::EOF
        ]
    );
}

#[test]
fn test_regex() {
    let str = "a ~ b a !~ b";
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Ident(symbolizer.get("a")),
            Token::BinOp(BinOp::MatchedBy),
            Token::Ident(symbolizer.get("b")),
            Token::Ident(symbolizer.get("a")),
            Token::BinOp(BinOp::NotMatchedBy),
            Token::Ident(symbolizer.get("b")),
            Token::EOF
        ]
    );
}

#[test]
fn test_regex_slash() {
    let str = "a ~ /match/";
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Ident(symbolizer.get("a")),
            Token::BinOp(BinOp::MatchedBy),
            Token::Regex(String::from("match")),
            Token::EOF
        ]
    );
}


#[test]
fn test_regex_slash_not() {
    let str = "a !~ /match/";
    let mut symbolizer = Symbolizer::new();
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Ident(symbolizer.get("a")),
            Token::BinOp(BinOp::NotMatchedBy),
            Token::Regex("match".to_string()),
            Token::EOF
        ]
    );
}


#[test]
fn test_array_ops_slash_not() {
    let mut symbolizer = Symbolizer::new();
    let str = "a[0] = 1; a[1,2,3,4] = 5; 6 in a";
    let a = Token::Ident(symbolizer.get("a"));
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            a.clone(),
            Token::LeftBracket,
            Token::NumberF64(0.0),
            Token::RightBracket,
            Token::Eq,
            Token::NumberF64(1.0),
            Token::Semicolon,
            a.clone(),
            Token::LeftBracket,
            Token::NumberF64(1.0),
            Token::Comma,
            Token::NumberF64(2.0),
            Token::Comma,
            Token::NumberF64(3.0),
            Token::Comma,
            Token::NumberF64(4.0),
            Token::RightBracket,
            Token::Eq,
            Token::NumberF64(5.0),
            Token::Semicolon,
            Token::NumberF64(6.0),
            Token::In,
            a,
            Token::EOF
        ]
    );
}


#[test]
fn test_function() {
    let mut symbolizer = Symbolizer::new();
    let str = "function a() { print 1 }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Function,
            Token::Ident(symbolizer.get("a")),
            Token::LeftParen,
            Token::RightParen,
            Token::LeftBrace,
            Token::Print,
            Token::NumberF64(1.0),
            Token::RightBrace,
            Token::EOF,
        ]
    );
}

#[test]
fn test_ident_bug_fix() {
    let mut symbolizer = Symbolizer::new();
    let str = "BEGIN { helper(a) }";
    assert_eq!(
        lex_test(str, &mut symbolizer).unwrap(),
        vec![
            Token::Begin,
            Token::LeftBrace,
            Token::Ident(symbolizer.get("helper")),
            Token::LeftParen,
            Token::Ident(symbolizer.get("a")),
            Token::RightParen,
            Token::RightBrace,
            Token::EOF,
        ]
    );

}


use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::slice::Iter;
use std::str::Chars;
use chrono::{Duration, NaiveDateTime};
use regex::Regex;
use crate::parser::{LogString, Value};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Token {
    WHERE,
    AND,
    OR,
    OpenBrace,
    CloseBrace,
    Identifier(String),
    String(String),
    Number(f64),
    Regex(Regex),
    Date(NaiveDateTime),
    DESC,
    ASC,

    Less,
    Greater,
    Equal,
    LE,
    GE,
    NE,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::WHERE => write!(f, "WHERE"),
            Token::AND => write!(f, "AND"),
            Token::OR => write!(f, "OR"),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "{}", s),
            Token::Number(s) => write!(f, "{}", s),
            Token::Regex(s) => write!(f, "{}", s),
            Token::Date(s) => write!(f, "{}", s),
            Token::DESC => write!(f, "DESC"),
            Token::ASC => write!(f, "ASC"),
            Token::Less => write!(f, "<"),
            Token::Greater => write!(f, ">"),
            Token::Equal => write!(f, "="),
            Token::LE => write!(f, "<="),
            Token::GE => write!(f, ">="),
            Token::NE => write!(f, "!="),
        }
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Token::WHERE, Token::WHERE) => true,
            (Token::AND, Token::AND) => true,
            (Token::OR, Token::OR) => true,
            (Token::OpenBrace, Token::OpenBrace) => true,
            (Token::CloseBrace, Token::CloseBrace) => true,
            (Token::Identifier(s1), Token::Identifier(s2)) => s1 == s2,
            (Token::String(s1), Token::String(s2)) => s1 == s2,
            (Token::Number(s1), Token::Number(s2)) => s1 == s2,
            //(Token::Regex(s1), Token::Regex(s2)) => s1 == s2,
            (Token::Date(s1), Token::Date(s2)) => s1 == s2,
            (Token::DESC, Token::DESC) => true,
            (Token::ASC, Token::ASC) => true,
            (Token::Less, Token::Less) => true,
            (Token::Greater, Token::Greater) => true,
            (Token::Equal, Token::Equal) => true,
            (Token::LE, Token::LE) => true,
            (Token::GE, Token::GE) => true,
            (Token::NE, Token::NE) => true,
            _ => false,
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    UnexpectedToken(Token),
    UnexpectedChar(char),
    RegexParseError(#[from] regex::Error),
    TimeParseError(#[from] chrono::ParseError),
    FloatParseError(#[from] std::num::ParseFloatError),
    InvalidDate,
    UnexpectedEndOfInput,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken(token) => write!(f, "Unexpected token: {}", token),
            ParseError::UnexpectedChar(c) => write!(f, "Unexpected char: {}", c),
            ParseError::RegexParseError(e) => write!(f, "Regex parse error: {}", e),
            ParseError::TimeParseError(e) => write!(f, "time parse error: {}", e),
            ParseError::FloatParseError(e) => write!(f, "float parse error: {}", e),
            ParseError::InvalidDate => write!(f, "Invalid date"),
            ParseError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
        }
    }
}

#[derive(Debug)]
pub enum Query {
    Expr(Option<Box<Query>>, Option<Box<Query>>),
    Regex(Regex),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),

    Equal(Token, Token),
    GE(Token, Token),
    LE(Token, Token),
    Greater(Token, Token),
    Less(Token, Token),
    NE(Token, Token),
}

impl Query {
    pub fn accept(&self, log_data: &LogString) -> bool {
        match self {
            Query::Expr(where_expr, _) => {
                if let Some(where_expr) = where_expr {
                    if !where_expr.accept(log_data) {
                        return false;
                    }
                }
                true
            }
            Query::Regex(regex) => {
                if let Value::String(s) = &log_data.event {
                    if regex.is_match(&s) {
                        return true
                    }
                }

                if let Value::String(s) = &log_data.process {
                    if regex.is_match(&s) {
                        return true
                    }
                }

                for (_, field) in log_data.fields.iter() {
                    if let Value::String(s) = field {
                        if regex.is_match(s) {
                            return true;
                        }
                    }
                }

                false
            }
            Query::And(left, right) => {
                left.accept(log_data) && right.accept(log_data)
            }
            Query::Or(left, right) => {
                left.accept(log_data) || right.accept(log_data)
            }
            Query::Equal(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x == *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x == *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Regex(right)) => {
                        log_data.get(left).map(|x| right.is_match(x.to_string().as_str())).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x == *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
            Query::GE(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x >= *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x >= *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x >= *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
            Query::LE(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x <= *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x <= *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x <= *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
            Query::Greater(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x > *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x > *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x > *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
            Query::Less(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x < *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x < *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x < *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
            Query::NE(left, right) => {
                match (left, right) {
                    (Token::Identifier(left), Token::String(right)) => {
                        log_data.get(left).map(|x| x != *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Number(right)) => {
                        log_data.get(left).map(|x| x != *right).unwrap_or(false)
                    }
                    (Token::Identifier(left), Token::Date(right)) => {
                        log_data.get(left).map(|x| x != *right).unwrap_or(false)
                    }
                    _ => {
                        false
                    }
                }
            }
        }
    }
}

pub struct Compiler {
    now: NaiveDateTime,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            now: chrono::Local::now().naive_local(),
        }
    }

    fn parse_numeric<T: Iterator<Item = char>>(&self, iter: &mut Peekable<T>) -> Result<f64, ParseError> {
        let mut tmp = String::new();
        while iter.peek().is_some() && iter.peek().unwrap().is_numeric() {
            tmp.push(iter.next().unwrap());
        }
        Ok(tmp.parse::<f64>()?)
    }

    fn parse_date(&self, iter: &mut Peekable<Chars>) -> Result<Token, ParseError> {
        let mut tmp = String::new();
        iter.next();
        while iter.peek().is_some() && iter.peek().unwrap().ne(&'\'') {
            tmp.push(iter.next().unwrap());
        }
        iter.next();
        if tmp.starts_with("now") {
            match tmp.chars().nth(3) {
                Some('-') => {
                    let mut str_iter = tmp.chars().skip(4).peekable();
                    let offset = self.parse_numeric(&mut str_iter)?;
                    match str_iter.next() {
                        Some('s') => {
                            Ok(Token::Date(self.now - Duration::seconds(offset as i64)))
                        }
                        Some('m') => {
                            Ok(Token::Date(self.now - Duration::minutes(offset as i64)))
                        }
                        Some('h') => {
                            Ok(Token::Date(self.now - Duration::hours(offset as i64)))
                        }
                        Some('d') => {
                            Ok(Token::Date(self.now - Duration::days(offset as i64)))
                        }
                        Some('w') => {
                            Ok(Token::Date(self.now - Duration::weeks(offset as i64)))
                        }
                        Some(c) => {
                            return Err(ParseError::UnexpectedChar(c))
                        }
                        _ => return Err(ParseError::UnexpectedEndOfInput),
                    }
                },
                Some(_) => return Err(ParseError::InvalidDate),
                None => Ok(Token::Date(self.now)),
            }
        }
        else {
            Ok(Token::Date(NaiveDateTime::parse_from_str(&tmp, "%Y-%m-%d %H:%M:%S")?))
        }
    }

    fn tokenize(&self, program: &str) -> Result<Vec<Token>, ParseError> {
        let mut tokens = vec![];
        let mut iter = program.chars().peekable();
        loop {
            match iter.peek() {
                Some(&c) => match c {
                    'a'..='z'|'A'..='Z' => {
                        let mut tmp = String::new();
                        while let Some(&peek) = iter.peek() {
                            match peek {
                                'a'..='z'|'A'..='Z'|'0'..='9'|'_'|':' if !tmp.is_empty() => {
                                    tmp.push(peek);
                                    iter.next();
                                }
                                'a'..='z'|'A'..='Z'|'_' => {
                                    tmp.push(peek);
                                    iter.next();
                                }
                                _ => break,
                            }
                        }

                        match tmp.as_str() {
                            "WHERE" => tokens.push(Token::WHERE),
                            "AND" => tokens.push(Token::AND),
                            "OR" => tokens.push(Token::OR),
                            "DESC" => tokens.push(Token::DESC),
                            "ASC" => tokens.push(Token::ASC),
                            _ => tokens.push(Token::Identifier(tmp))
                        }
                    },
                    '0'..='9' => {
                        tokens.push(Token::Number(self.parse_numeric(&mut iter)?));
                        iter.next();
                    },
                    '"' => {
                        let mut tmp = String::new();
                        iter.next();
                        while iter.peek().is_some() && iter.peek().unwrap().ne(&'"') {
                            tmp.push(iter.next().unwrap());
                        }
                        iter.next();
                        tokens.push(Token::String(tmp));
                    },
                    '\'' => {
                        tokens.push(self.parse_date(&mut iter)?);
                    },
                    '/' => { //regex
                        let mut tmp = String::new();
                        iter.next();
                        while iter.peek().is_some() && iter.peek().unwrap().ne(&'/') {
                            tmp.push(iter.next().unwrap());
                        }
                        iter.next();
                        tokens.push(Token::Regex(Regex::new(&tmp)?));
                    },
                    '(' => {
                        tokens.push(Token::OpenBrace);
                        iter.next();
                    },
                    ')' => {
                        tokens.push(Token::CloseBrace);
                        iter.next();
                    },
                    '=' => {
                        tokens.push(Token::Equal);
                        iter.next();
                    },
                    '>' => {
                        iter.next();
                        match iter.peek() {
                            Some(&'=') => {
                                iter.next();
                                tokens.push(Token::GE)
                            },
                            _ => tokens.push(Token::Greater)
                        }
                    },
                    '<' => {
                        iter.next();
                        match iter.peek() {
                            Some(&'=') => {
                                iter.next();
                                tokens.push(Token::LE)
                            },
                            _ => tokens.push(Token::Less)
                        }
                    },
                    '!' => {
                        iter.next();
                        match iter.peek() {
                            Some(&'=') => {
                                iter.next();
                                tokens.push(Token::NE)
                            },
                            Some(&c) => return Err(ParseError::UnexpectedChar(c)),
                            _ => return Err(ParseError::UnexpectedEndOfInput)
                        }
                    }
                    ' ' => { iter.next(); }
                    c => return Err(ParseError::UnexpectedChar(c))
                },
                None => break
            }
        }

        Ok(tokens)
    }

    fn compile_value(&self, iter: &mut Peekable<Iter<Token>>, allow_reg: bool) -> Result<Token, ParseError> {
        match iter.peek() {
            Some(Token::String(value)) => {
                iter.next();
                Ok(Token::String(value.clone()))
            },
            Some(Token::Number(value)) => {
                iter.next();
                Ok(Token::Number(value.clone()))
            },
            Some(Token::Regex(value)) if allow_reg => {
                iter.next();
                Ok(Token::Regex(value.clone()))
            },
            Some(Token::Date(value)) => {
                iter.next();
                Ok(Token::Date(value.clone()))
            },
            Some(&t) => Err(ParseError::UnexpectedToken(t.clone())),
            None => Err(ParseError::UnexpectedEndOfInput)
        }
    }

    fn compile_condition(&self, iter: &mut Peekable<Iter<Token>>) -> Result<Query, ParseError> {
        match iter.peek() {
            Some(Token::OpenBrace) => {
                iter.next();
                let expr = self.compile_expression(iter);
                iter.next();
                expr
            },
            Some(Token::Identifier(ident)) => {
                let left = Token::Identifier(ident.clone());
                iter.next();
                match iter.peek() {
                    Some(Token::Equal) => {
                        iter.next();
                        Ok(Query::Equal(left, self.compile_value(iter, true)?))
                    }
                    Some(Token::Greater) => {
                        iter.next();
                        Ok(Query::Greater(left, self.compile_value(iter, false)?))
                    }
                    Some(Token::Less) => {
                        iter.next();
                        Ok(Query::Less(left, self.compile_value(iter, false)?))
                    }
                    Some(Token::GE) => {
                        iter.next();
                        Ok(Query::GE(left, self.compile_value(iter, false)?))
                    }
                    Some(Token::LE) => {
                        iter.next();
                        Ok(Query::LE(left, self.compile_value(iter, false)?))
                    }
                    Some(Token::NE) => {
                        iter.next();
                        Ok(Query::NE(left, self.compile_value(iter, false)?))
                    }
                    Some(&t) => Err(ParseError::UnexpectedToken(t.clone())),
                    _ => Err(ParseError::UnexpectedEndOfInput)
                }
            },
            Some(&t) => Err(ParseError::UnexpectedToken(t.clone())),
            None => Err(ParseError::UnexpectedEndOfInput)
        }
    }

    fn compile_term(&self, iter: &mut Peekable<Iter<Token>>) -> Result<Query, ParseError> {
        let mut ast = self.compile_condition(iter)?;
        while let Some(Token::AND) = iter.peek() {
            iter.next();
            ast = Query::And(Box::new(ast), Box::new(self.compile_condition(iter)?));
        }
        Ok(ast)
    }

    fn compile_expression(&self, iter: &mut Peekable<Iter<Token>>) -> Result<Query, ParseError> {
        let mut ast = self.compile_term(iter)?;
        while let Some(Token::OR) = iter.peek() {
            iter.next();
            ast = Query::Or(Box::new(ast), Box::new(self.compile_term(iter)?));
        }
        Ok(ast)
    }

    pub(crate) fn compile(&self, program: &str) -> Result<Query, ParseError> {
        let tokens = self.tokenize(program)?;
        let mut iter = tokens.iter().peekable();
        let mut ast = Query::Expr(None, None);
        while iter.peek().is_some() {
            match iter.next() {
                Some(Token::WHERE) => {
                    if let Query::Expr(left, _) = &mut ast {
                        *left = Some(Box::new(self.compile_expression(&mut iter)?));
                    }
                }
                Some(Token::Regex(regex)) => {
                    ast = Query::Regex(regex.clone());
                    if let Some(token) = iter.next() {
                        return Err(ParseError::UnexpectedToken(token.clone()))
                    }
                }
                Some(other) => return Err(ParseError::UnexpectedToken(other.clone())),
                None => return Err(ParseError::UnexpectedEndOfInput)
            }
        }

        Ok(ast)
    }
}

#[test]
fn test_tokenizer() {
    let compiler = Compiler::new();
    let tokens = compiler.tokenize("WHERE date > 'now' AND date < 'now-1d'").unwrap();
    dbg!(tokens);
}

#[test]
fn compile_regex() {
    let compiler = Compiler::new();
    let query = compiler.compile("/John/").unwrap();
    dbg!(query);
}

#[test]
fn test_regex_tokenize() {
    let compiler = Compiler::new();
    let tokens = compiler.tokenize("WHERE name = /John/ AND age > 20").unwrap();
    assert!(matches!(tokens[3], Token::Regex(_)));
}
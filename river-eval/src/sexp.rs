//! A small S-expression reader.
//!
//! This is intentionally not a general Lisp reader. It supports the forms the
//! River IR needs: lists, symbols, keywords, strings, numbers, booleans, and
//! nil. Comments begin with `;`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Symbol(String),
    Keyword(String),
    String(String),
    List(Vec<Value>),
}

impl Value {
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Self::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Self::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(values) => Some(values),
            // Lisp's empty list and nil are the same value.
            Self::Nil => Some(&[]),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(value) => Some(*value as f64),
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) | Self::Symbol(value) | Self::Keyword(value) => Some(value),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(formatter, "nil"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Int(value) => write!(formatter, "{value}"),
            Self::Float(value) => write!(formatter, "{value}"),
            Self::Symbol(value) => write!(formatter, "{value}"),
            Self::Keyword(value) => write!(formatter, ":{value}"),
            Self::String(value) => write!(formatter, "{value:?}"),
            Self::List(values) => {
                write!(formatter, "(")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, " ")?;
                    }
                    write!(formatter, "{value}")?;
                }
                write!(formatter, ")")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_all(input: &str) -> Result<Vec<Value>, ParseError> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let mut values = Vec::new();

    loop {
        parser.skip_ws();
        if parser.pos >= parser.bytes.len() {
            break;
        }
        values.push(parser.parse_value()?);
    }

    Ok(values)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            offset: self.pos,
        }
    }

    fn skip_ws(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            if byte == b';' {
                while let Some(&comment_byte) = self.bytes.get(self.pos) {
                    self.pos += 1;
                    if comment_byte == b'\n' {
                        break;
                    }
                }
            } else if byte.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        let Some(&byte) = self.bytes.get(self.pos) else {
            return Err(self.error("unexpected end of input"));
        };

        match byte {
            b'(' => self.parse_list(),
            b')' => Err(self.error("unexpected `)`")),
            b'\'' => {
                self.pos += 1;
                let value = self.parse_value()?;
                Ok(Value::List(vec![Value::Symbol("quote".to_string()), value]))
            }
            b'"' => self.parse_string(),
            _ => self.parse_atom(),
        }
    }

    fn parse_list(&mut self) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut values = Vec::new();

        loop {
            self.skip_ws();
            let Some(&byte) = self.bytes.get(self.pos) else {
                return Err(self.error("unterminated list"));
            };
            if byte == b')' {
                self.pos += 1;
                return Ok(if values.is_empty() {
                    Value::Nil
                } else {
                    Value::List(values)
                });
            }
            values.push(self.parse_value()?);
        }
    }

    fn parse_string(&mut self) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut value = String::new();

        loop {
            let Some(&byte) = self.bytes.get(self.pos) else {
                return Err(self.error("unterminated string"));
            };
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(Value::String(value));
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(&escaped) = self.bytes.get(self.pos) else {
                        return Err(self.error("unterminated escape"));
                    };
                    match escaped {
                        b'n' => value.push('\n'),
                        b'r' => value.push('\r'),
                        b't' => value.push('\t'),
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        other => value.push(other as char),
                    }
                    self.pos += 1;
                }
                _ => {
                    let rest = std::str::from_utf8(&self.bytes[self.pos..])
                        .map_err(|_| self.error("invalid UTF-8"))?;
                    let character = rest
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("unexpected end of string"))?;
                    value.push(character);
                    self.pos += character.len_utf8();
                }
            }
        }
    }

    fn parse_atom(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        while let Some(&byte) = self.bytes.get(self.pos) {
            if is_delimiter(byte) {
                break;
            }
            self.pos += 1;
        }

        if start == self.pos {
            return Err(self.error("expected an atom"));
        }

        let atom = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.error("invalid UTF-8"))?;

        match atom {
            "nil" => Ok(Value::Nil),
            "true" | "#t" => Ok(Value::Bool(true)),
            "false" | "#f" => Ok(Value::Bool(false)),
            _ if atom.starts_with(':') => Ok(Value::Keyword(atom[1..].to_string())),
            _ => {
                if let Ok(value) = atom.parse::<i64>() {
                    Ok(Value::Int(value))
                } else if let Ok(value) = atom.parse::<f64>() {
                    Ok(Value::Float(value))
                } else {
                    Ok(Value::Symbol(atom.to_string()))
                }
            }
        }
    }
}

fn is_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace() || matches!(byte, b'(' | b')' | b'"' | b';')
}

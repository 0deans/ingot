use super::nbt::Nbt;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SnbtError {
    #[error("Unexpected trailing data at position {0}")]
    TrailingData(usize),
    #[error("Expected '{expected}' at position {pos}")]
    ExpectedChar { expected: char, pos: usize },
    #[error("Unexpected end of SNBT")]
    UnexpectedEnd,
    #[error("Expected ',' or '{expected}' at position {pos}")]
    ExpectedDelimiter { expected: char, pos: usize },
    #[error("Unterminated string")]
    UnterminatedString,
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Unexpected character at position {0}")]
    UnexpectedChar(usize),
}

struct SnbtParser<'a> {
    src: &'a [u8],
    pos: usize,
}

pub fn parse_snbt(text: &str) -> Result<Nbt, SnbtError> {
    let mut parser = SnbtParser {
        src: text.as_bytes(),
        pos: 0,
    };
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.src.len() {
        return Err(SnbtError::TrailingData(parser.pos));
    }
    Ok(value)
}

impl SnbtParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), SnbtError> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(SnbtError::ExpectedChar {
                expected: c as char,
                pos: self.pos,
            })
        }
    }

    fn value(&mut self) -> Result<Nbt, SnbtError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.compound(),
            Some(b'[') => self.list(),
            Some(b'"') | Some(b'\'') => Ok(Nbt::String(self.quoted()?)),
            Some(_) => Ok(Self::scalar(&self.token()?)),
            None => Err(SnbtError::UnexpectedEnd),
        }
    }

    fn compound(&mut self) -> Result<Nbt, SnbtError> {
        self.expect(b'{')?;
        let mut map = HashMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Nbt::Compound(map));
        }
        loop {
            self.skip_ws();
            let key = match self.peek() {
                Some(b'"') | Some(b'\'') => self.quoted()?,
                _ => self.token()?,
            };
            self.expect(b':')?;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Nbt::Compound(map));
                }
                _ => {
                    return Err(SnbtError::ExpectedDelimiter {
                        expected: '}',
                        pos: self.pos,
                    })
                }
            }
        }
    }

    fn list(&mut self) -> Result<Nbt, SnbtError> {
        self.expect(b'[')?;
        let array_kind = match (self.src.get(self.pos), self.src.get(self.pos + 1)) {
            (Some(k @ (b'B' | b'I' | b'L')), Some(b';')) => Some(*k),
            _ => None,
        };
        if array_kind.is_some() {
            self.pos += 2;
        }
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
        } else {
            loop {
                items.push(self.value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b']') => {
                        self.pos += 1;
                        break;
                    }
                    _ => {
                        return Err(SnbtError::ExpectedDelimiter {
                            expected: ']',
                            pos: self.pos,
                        })
                    }
                }
            }
        }
        let nums = || items.iter().filter_map(Nbt::as_f64);
        Ok(match array_kind {
            Some(b'B') => Nbt::ByteArray(nums().map(|v| v as i8).collect()),
            Some(b'I') => Nbt::IntArray(nums().map(|v| v as i32).collect()),
            Some(b'L') => Nbt::LongArray(nums().map(|v| v as i64).collect()),
            _ => Nbt::List(items),
        })
    }

    fn quoted(&mut self) -> Result<String, SnbtError> {
        let quote = self.peek().ok_or(SnbtError::UnexpectedEnd)?;
        self.pos += 1;
        let mut bytes = Vec::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == quote {
                return String::from_utf8(bytes).map_err(SnbtError::from);
            }
            if c == b'\\' {
                let escaped = self.peek().ok_or(SnbtError::UnexpectedEnd)?;
                self.pos += 1;
                bytes.push(match escaped {
                    b'n' => b'\n',
                    b't' => b'\t',
                    other => other,
                });
            } else {
                bytes.push(c);
            }
        }
        Err(SnbtError::UnterminatedString)
    }

    fn token(&mut self) -> Result<String, SnbtError> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'+'))
        {
            self.pos += 1;
        }
        if start == self.pos {
            return Err(SnbtError::UnexpectedChar(self.pos));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    fn scalar(token: &str) -> Nbt {
        match token {
            "true" => return Nbt::Byte(1),
            "false" => return Nbt::Byte(0),
            _ => {}
        }
        let (body, suffix) = match token.chars().last() {
            Some(c) if "bBsSlLfFdD".contains(c) && token.len() > 1 => {
                (&token[..token.len() - 1], Some(c))
            }
            _ => (token, None),
        };
        let parsed = match suffix.map(|c| c.to_ascii_lowercase()) {
            Some('b') => body.parse().ok().map(Nbt::Byte),
            Some('s') => body.parse().ok().map(Nbt::Short),
            Some('l') => body.parse().ok().map(Nbt::Long),
            Some('f') => body.parse().ok().map(Nbt::Float),
            Some('d') => body.parse().ok().map(Nbt::Double),
            _ => body
                .parse()
                .ok()
                .map(Nbt::Int)
                .or_else(|| body.parse().ok().map(Nbt::Double)),
        };
        parsed.unwrap_or_else(|| Nbt::String(token.to_string()))
    }
}

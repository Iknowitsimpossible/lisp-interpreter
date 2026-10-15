use serde_json::{json, Value as Json};
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub incomplete: bool,
}

impl Error {
    pub(crate) fn at(pos: Position, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: pos.line,
            column: pos.column,
            incomplete: false,
        }
    }
    fn unfinished(pos: Position, message: &str) -> Self {
        Self {
            incomplete: true,
            ..Self::at(pos, message)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Int(i64),
    Float(f64),
    String(String),
    Symbol(String),
    List(Vec<Expr>),
}
#[derive(Clone, Debug)]
pub(crate) struct Expr {
    pub kind: Kind,
    pub pos: Position,
}
impl Expr {
    pub fn name(&self) -> Result<String, Error> {
        match &self.kind {
            Kind::Symbol(name) => Ok(name.clone()),
            _ => Err(Error::at(self.pos, "expected a variable or function name")),
        }
    }
    pub fn json(&self) -> Json {
        match &self.kind {
            Kind::Int(n) => json!(n),
            Kind::Float(n) => json!(n),
            Kind::String(s) | Kind::Symbol(s) => json!(s),
            Kind::List(xs) => Json::Array(xs.iter().map(Expr::json).collect()),
        }
    }
}

#[derive(Clone, Debug)]
enum TokenKind {
    Open,
    Close,
    Quote,
    Atom(Kind),
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    pos: Position,
}

fn atom(text: String, pos: Position) -> Result<Kind, Error> {
    let unsigned = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(&text);
    let integer = !unsigned.is_empty() && unsigned.chars().all(|c| c.is_ascii_digit());
    if integer {
        return text
            .parse()
            .map(Kind::Int)
            .map_err(|_| Error::at(pos, "integer exceeds signed 64-bit range"));
    }
    let decimal = unsigned.contains('.')
        && unsigned.chars().filter(|c| *c == '.').count() == 1
        && unsigned.chars().any(|c| c.is_ascii_digit())
        && unsigned.chars().all(|c| c.is_ascii_digit() || c == '.');
    if decimal {
        let number: f64 = text
            .parse()
            .map_err(|_| Error::at(pos, "invalid decimal"))?;
        if !number.is_finite() {
            return Err(Error::at(pos, "decimal out of range"));
        }
        return Ok(Kind::Float(number));
    }
    Ok(Kind::Symbol(text))
}

fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = source.chars().collect();
    let (mut i, mut line, mut column) = (0, 1, 1);
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        let pos = Position { line, column };
        if c.is_whitespace() {
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            i += 1;
        } else if c == ';' || c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
                column += 1;
            }
        } else if "()'".contains(c) {
            let kind = match c {
                '(' => TokenKind::Open,
                ')' => TokenKind::Close,
                _ => TokenKind::Quote,
            };
            tokens.push(Token { kind, pos });
            i += 1;
            column += 1;
        } else if c == '"' {
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\n' {
                    return Err(Error::at(pos, "use \\n inside strings"));
                }
                if chars[i] == '\\' {
                    i += 2;
                } else if chars[i] == '"' {
                    break;
                } else {
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err(Error::unfinished(pos, "unclosed string"));
            }
            let raw: String = chars[start..=i].iter().collect();
            let value = serde_json::from_str::<String>(&raw)
                .map_err(|_| Error::at(pos, "invalid string escape"))?;
            tokens.push(Token {
                kind: TokenKind::Atom(Kind::String(value)),
                pos,
            });
            column += i - start + 1;
            i += 1;
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && !"()'\";#".contains(chars[i]) {
                if chars[i].is_control() {
                    return Err(Error::at(pos, "illegal control character"));
                }
                i += 1;
                column += 1;
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(Token {
                kind: TokenKind::Atom(atom(text, pos)?),
                pos,
            });
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        pos: Position { line, column },
    });
    Ok(tokens)
}

pub(crate) fn parse(source: &str) -> Result<Vec<Expr>, Error> {
    fn read(tokens: &[Token], index: &mut usize, depth: usize) -> Result<Expr, Error> {
        let token = &tokens[*index];
        let pos = token.pos;
        if depth > 150 {
            return Err(Error::at(pos, "nesting limit exceeded (150)"));
        }
        *index += 1;
        let kind = match &token.kind {
            TokenKind::Open => {
                let mut items = Vec::new();
                while !matches!(tokens[*index].kind, TokenKind::Close) {
                    if matches!(tokens[*index].kind, TokenKind::Eof) {
                        return Err(Error::unfinished(pos, "unclosed '('"));
                    }
                    items.push(read(tokens, index, depth + 1)?);
                }
                *index += 1;
                Kind::List(items)
            }
            TokenKind::Quote => Kind::List(vec![
                Expr {
                    kind: Kind::Symbol("quote".into()),
                    pos,
                },
                read(tokens, index, depth + 1)?,
            ]),
            TokenKind::Close => return Err(Error::at(pos, "unexpected ')'")),
            TokenKind::Eof => return Err(Error::unfinished(pos, "expected expression")),
            TokenKind::Atom(value) => value.clone(),
        };
        Ok(Expr { kind, pos })
    }
    let tokens = tokenize(source)?;
    let mut index = 0;
    let mut program = Vec::new();
    while !matches!(tokens[index].kind, TokenKind::Eof) {
        program.push(read(&tokens, &mut index, 0)?);
    }
    Ok(program)
}

/// Return positioned tokens as JSON, without executing the program.
pub fn inspect_tokens(source: &str) -> Result<Json, Error> {
    Ok(Json::Array(
        tokenize(source)?
            .iter()
            .map(|token| {
                let (kind, value) = match &token.kind {
                    TokenKind::Open => ("(", json!("(")),
                    TokenKind::Close => (")", json!(")")),
                    TokenKind::Quote => ("'", json!("'")),
                    TokenKind::Eof => ("EOF", Json::Null),
                    TokenKind::Atom(kind) => (
                        "atom",
                        Expr {
                            kind: kind.clone(),
                            pos: token.pos,
                        }
                        .json(),
                    ),
                };
                json!({"kind":kind,"value":value,"line":token.pos.line,"column":token.pos.column})
            })
            .collect(),
    ))
}

/// Return nested expression trees as JSON, without executing the program.
pub fn inspect_ast(source: &str) -> Result<Json, Error> {
    Ok(Json::Array(parse(source)?.iter().map(Expr::json).collect()))
}

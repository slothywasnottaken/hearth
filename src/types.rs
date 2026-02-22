#![allow(unused)]

use std::{
    collections::{HashMap, HashSet, hash_map::Iter},
    default,
    fmt::{Debug, Display},
    num::{ParseIntError, TryFromIntError},
    ops::Add,
};

use tracing::{debug, info, instrument, trace, warn};

use crate::{
    parser::{ParseError, ParseResult},
    tokenizer::Token,
};

#[derive(Debug, Clone)]
pub(crate) struct Typer<'a> {
    types: HashMap<&'a str, ComplexTypeID>,
    type_ids: HashMap<ComplexTypeID, ComplexType<'a>>,
    next_id: ComplexTypeID,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ComplexTypeID {
    id: usize,
}

impl<'a> Default for Typer<'a> {
    fn default() -> Self {
        Self {
            types: HashMap::default(),
            type_ids: HashMap::default(),
            next_id: ComplexTypeID { id: 0 },
        }
    }
}

impl Display for Typer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.types)
    }
}

impl<'a> Typer<'a> {
    pub fn register_unknown(&mut self, type_name: &'a str, value: ComplexTypeDecl<'a>) {
        trace!(?self, ?type_name, ?value);
        assert!(self.types.insert(type_name, self.next_id).is_none());
        assert!(
            self.type_ids
                .insert(self.next_id, ComplexType::Unknown(value))
                .is_none()
        );
        self.next_id.id += 1;
    }

    pub fn register(&mut self, type_name: &'a str, value: ComplexTypeDecl<'a>) {
        trace!(?self, ?type_name, ?value);
        assert!(self.types.insert(type_name, self.next_id).is_none());
        assert!(
            self.type_ids
                .insert(self.next_id, ComplexType::Known(value))
                .is_none()
        );

        self.next_id.id += 1;
    }

    pub fn id(&self, type_name: &str) -> Option<ComplexTypeID> {
        self.types.get(type_name).copied()
    }

    pub fn get_id(&self, id: ComplexTypeID) -> Option<&ComplexType<'a>> {
        self.type_ids.get(&id)
    }

    #[instrument(name = "Typer::get")]
    pub fn get(&self, type_name: &str) -> Option<&ComplexType<'_>> {
        self.type_ids.get(self.types.get(type_name)?)
    }

    pub fn get_mut(&'a mut self, type_name: &str) -> Option<&'a mut ComplexType<'a>> {
        let id = self.types.get(type_name)?;
        self.type_ids.get_mut(id)
    }

    pub fn remove(&mut self, type_name: &str) {
        self.types.remove(type_name);
    }

    pub fn iter(&'a self) -> Iter<'a, ComplexTypeID, ComplexType<'a>> {
        self.type_ids.iter()
    }
}

#[derive(Debug, Clone)]
pub enum ComplexType<'a> {
    Known(ComplexTypeDecl<'a>),
    Unknown(ComplexTypeDecl<'a>),
}

impl<'a> ComplexType<'a> {
    pub fn inner(&self) -> &ComplexTypeDecl<'_> {
        match self {
            Self::Known(s) | Self::Unknown(s) => s,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TypeID {
    Primitive(PrimitiveID),
    Complex(ComplexTypeID),
}

impl From<crate::tokenizer::TypeID> for TypeID {
    fn from(value: crate::tokenizer::TypeID) -> Self {
        match value {
            crate::tokenizer::TypeID::I8 => Self::Primitive(PrimitiveID::I8),
            crate::tokenizer::TypeID::I16 => Self::Primitive(PrimitiveID::I16),
            crate::tokenizer::TypeID::I32 => Self::Primitive(PrimitiveID::I32),
            crate::tokenizer::TypeID::I64 => Self::Primitive(PrimitiveID::I64),
            crate::tokenizer::TypeID::U8 => Self::Primitive(PrimitiveID::U8),
            crate::tokenizer::TypeID::U16 => Self::Primitive(PrimitiveID::U16),
            crate::tokenizer::TypeID::U32 => Self::Primitive(PrimitiveID::U32),
            crate::tokenizer::TypeID::U64 => Self::Primitive(PrimitiveID::U64),
            crate::tokenizer::TypeID::F32 => Self::Primitive(PrimitiveID::F32),
            crate::tokenizer::TypeID::F64 => Self::Primitive(PrimitiveID::F64),
            crate::tokenizer::TypeID::String => Self::Primitive(PrimitiveID::String),
            crate::tokenizer::TypeID::QuotedString => Self::Primitive(PrimitiveID::String),
            crate::tokenizer::TypeID::Bool => Self::Primitive(PrimitiveID::Bool),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub(crate) enum Number {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl Eq for Number {}

impl Number {
    pub fn can_fit(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::I8(_), Number::I8(_))
            | (Number::I16(_), Number::I16(_))
            | (Number::I32(_), Number::I32(_))
            | (Number::I64(_), Number::I64(_))
            | (Number::U8(_), Number::U8(_))
            | (Number::U16(_), Number::U16(_))
            | (Number::U32(_), Number::U32(_))
            | (Number::U64(_), Number::U64(_)) => true,
            (Number::I8(n), Number::I16(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::I32(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::I64(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::U8(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::U16(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::U32(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::U64(nn)) => i8::try_from(*nn).is_ok(),
            (Number::I8(n), Number::F32(nn)) => false,
            (Number::I8(n), Number::F64(nn)) => false,
            (Number::I16(n), Number::I8(nn)) => true,
            (Number::I16(n), Number::I32(nn)) => i16::try_from(*nn).is_ok(),
            (Number::I16(n), Number::I64(nn)) => i16::try_from(*nn).is_ok(),
            (Number::I16(n), Number::U8(nn)) => true,
            (Number::I16(n), Number::U16(nn)) => i16::try_from(*nn).is_ok(),
            (Number::I16(n), Number::U32(nn)) => i16::try_from(*nn).is_ok(),
            (Number::I16(n), Number::U64(nn)) => i16::try_from(*nn).is_ok(),
            (Number::I16(n), Number::F32(nn)) => false,
            (Number::I16(n), Number::F64(nn)) => false,
            (Number::I32(n), Number::I8(nn)) => true,
            (Number::I32(n), Number::I16(nn)) => true,
            (Number::I32(n), Number::I64(nn)) => i32::try_from(*nn).is_ok(),
            (Number::I32(n), Number::U8(nn)) => true,
            (Number::I32(n), Number::U16(nn)) => true,
            (Number::I32(n), Number::U32(nn)) => i32::try_from(*nn).is_ok(),
            (Number::I32(n), Number::U64(nn)) => i32::try_from(*nn).is_ok(),
            (Number::I32(n), Number::F32(nn)) => false,
            (Number::I32(n), Number::F64(nn)) => false,
            (Number::I64(n), Number::I8(nn)) => true,
            (Number::I64(n), Number::I16(nn)) => true,
            (Number::I64(n), Number::I32(nn)) => true,
            (Number::I64(n), Number::U8(nn)) => true,
            (Number::I64(n), Number::U16(nn)) => true,
            (Number::I64(n), Number::U32(nn)) => true,
            (Number::I64(n), Number::U64(nn)) => i64::try_from(*nn).is_ok(),
            (Number::I64(n), Number::F32(nn)) => false,
            (Number::I64(n), Number::F64(nn)) => false,
            (Number::U8(n), Number::I8(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::I16(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::I32(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::I64(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::U8(nn)) => true,
            (Number::U8(n), Number::U16(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::U32(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::U64(nn)) => u8::try_from(*nn).is_ok(),
            (Number::U8(n), Number::F32(nn)) => false,
            (Number::U8(n), Number::F64(nn)) => false,
            (Number::U16(n), Number::I8(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::I16(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::I32(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::I64(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::U8(nn)) => true,
            (Number::U16(n), Number::U16(nn)) => true,
            (Number::U16(n), Number::U32(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::U64(nn)) => u16::try_from(*nn).is_ok(),
            (Number::U16(n), Number::F32(nn)) => false,
            (Number::U16(n), Number::F64(nn)) => false,
            (Number::U32(n), Number::I8(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U32(n), Number::I16(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U32(n), Number::I32(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U32(n), Number::I64(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U32(n), Number::U8(nn)) => true,
            (Number::U32(n), Number::U16(nn)) => true,
            (Number::U32(n), Number::U32(nn)) => true,
            (Number::U32(n), Number::U64(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U32(n), Number::F32(nn)) => false,
            (Number::U32(n), Number::F64(nn)) => false,
            (Number::U64(n), Number::I8(nn)) => u32::try_from(*nn).is_ok(),
            (Number::U64(n), Number::I16(nn)) => u64::try_from(*nn).is_ok(),
            (Number::U64(n), Number::I32(nn)) => u64::try_from(*nn).is_ok(),
            (Number::U64(n), Number::I64(nn)) => u64::try_from(*nn).is_ok(),
            (Number::U64(n), Number::U8(nn)) => true,
            (Number::U64(n), Number::U16(nn)) => true,
            (Number::U64(n), Number::U32(nn)) => true,
            (Number::U64(n), Number::U64(nn)) => true,
            (Number::U64(n), Number::F32(nn)) => false,
            (Number::U64(n), Number::F64(nn)) => false,
            (Number::F32(n), Number::I8(nn)) => true,
            (Number::F32(n), Number::I16(nn)) => true,
            (Number::F32(n), Number::I32(nn)) => false,
            (Number::F32(n), Number::I64(nn)) => false,
            (Number::F32(n), Number::U8(nn)) => true,
            (Number::F32(n), Number::U16(nn)) => true,
            (Number::F32(n), Number::U32(nn)) => false,
            (Number::F32(n), Number::U64(nn)) => false,
            (Number::F32(n), Number::F32(nn)) => true,
            (Number::F32(n), Number::F64(nn)) => true,
            (Number::F64(n), Number::I8(nn)) => true,
            (Number::F64(n), Number::I16(nn)) => true,
            (Number::F64(n), Number::I32(nn)) => true,
            (Number::F64(n), Number::U8(nn)) => true,
            (Number::F64(n), Number::U16(nn)) => true,
            (Number::F64(n), Number::U32(nn)) => true,
            (Number::F64(n), Number::F32(nn)) => true,
            (Number::F64(n), Number::F64(nn)) => true,
            (Number::F64(n), Number::I64(nn)) => false,
            (Number::F64(n), Number::U64(nn)) => false,
        }
    }

    pub fn id(&self) -> PrimitiveID {
        match self {
            Number::I8(_) => PrimitiveID::I8,
            Number::I16(_) => PrimitiveID::I16,
            Number::I32(_) => PrimitiveID::I32,
            Number::I64(_) => PrimitiveID::I64,
            Number::U8(_) => PrimitiveID::U8,
            Number::U16(_) => PrimitiveID::U16,
            Number::U32(_) => PrimitiveID::U32,
            Number::U64(_) => PrimitiveID::U64,
            Number::F32(_) => PrimitiveID::F32,
            Number::F64(_) => PrimitiveID::F64,
        }
    }
}

impl<'a> From<Number> for Primitive<'a> {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone, Copy)]
pub(crate) enum Primitive<'a> {
    Number(Number),
    String(&'a str),
    Bool(bool),
}

impl<'a> Primitive<'a> {
    pub fn id(&self) -> PrimitiveID {
        match self {
            Primitive::Number(number) => number.id(),
            Primitive::String(_) => PrimitiveID::String,
            Primitive::Bool(_) => PrimitiveID::Bool,
        }
    }

    pub fn number(&self) -> Option<&Number> {
        match self {
            Self::Number(n) => Some(n),
            Self::String(_) => None,
            Self::Bool(_) => None,
        }
    }

    /// dumb stupid hack to change struct fields in variable assignment (auto set to i64/f64/string) and convert it to the type declared in the struct decl
    // maps Foo {i:i32} -> let foo = Foo {i:0} (0 is auto set as i64) and needs to be set as a i32
    #[instrument(name = "Primitive::coerce", err)]
    pub fn coerce(&self, id: PrimitiveID) -> Result<Primitive<'a>, ParseError> {
        match (self, id) {
            (Primitive::Number(number), PrimitiveID::I8) => {
                Ok(Primitive::Number(Number::I8(match number {
                    Number::I8(n) => *n,
                    Number::I16(n) => i8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::I32(n) => i8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::I64(n) => i8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::I16) => {
                Ok(Primitive::Number(Number::I16(match number {
                    Number::I8(n) => *n as i16,
                    Number::I16(n) => *n,
                    Number::I32(n) => i16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::I64(n) => i16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    t => panic!("{t:?}"),
                })))
            }
            (Primitive::Number(number), PrimitiveID::I32) => {
                Ok(Primitive::Number(Number::I32(match number {
                    Number::I8(n) => *n as i32,
                    Number::I16(n) => *n as i32,
                    Number::I32(n) => *n,
                    Number::I64(n) => i32::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    t => panic!("{t:?}"),
                })))
            }
            (Primitive::Number(number), PrimitiveID::I64) => {
                Ok(Primitive::Number(Number::I64(match number {
                    Number::I8(n) => *n as i64,
                    Number::I16(n) => *n as i64,
                    Number::I32(n) => *n as i64,
                    Number::I64(n) => *n,
                    Number::U64(n) => i64::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    t => panic!("{t:?}"),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U8) => {
                Ok(Primitive::Number(Number::U8(match number {
                    Number::U8(n) => *n,
                    Number::U16(n) => u8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::U32(n) => u8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::U64(n) => u8::try_from(*n).map_err(|_| ParseError::IncorrectType)?,

                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U16) => {
                Ok(Primitive::Number(Number::U16(match number {
                    Number::U8(n) => *n as u16,
                    Number::U16(n) => *n,
                    Number::U32(n) => u16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::U64(n) => u16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U32) => {
                Ok(Primitive::Number(Number::U32(match number {
                    Number::U8(n) => *n as u32,
                    Number::U16(n) => *n as u32,
                    Number::U32(n) => *n,
                    Number::U64(n) => u32::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U64) => {
                Ok(Primitive::Number(Number::U64(match number {
                    Number::U8(n) => *n as u64,
                    Number::U16(n) => *n as u64,
                    Number::U32(n) => *n as u64,
                    Number::U64(n) => *n,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::F32) => {
                Ok(Primitive::Number(Number::F32(match number {
                    Number::F32(n) => *n,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::F64) => {
                Ok(Primitive::Number(Number::F64(match number {
                    Number::F32(n) => *n as f64,
                    Number::F64(n) => *n,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::String) => {
                return Err(ParseError::IncorrectType);
            }
            (Primitive::String(_), PrimitiveID::I8)
            | (Primitive::String(_), PrimitiveID::I16)
            | (Primitive::String(_), PrimitiveID::I32)
            | (Primitive::String(_), PrimitiveID::I64)
            | (Primitive::String(_), PrimitiveID::U8)
            | (Primitive::String(_), PrimitiveID::U16)
            | (Primitive::String(_), PrimitiveID::U32)
            | (Primitive::String(_), PrimitiveID::U64)
            | (Primitive::String(_), PrimitiveID::F32)
            | (Primitive::String(_), PrimitiveID::F64) => return Err(ParseError::IncorrectType),
            (Primitive::String(s), PrimitiveID::String) => Ok(Primitive::String(s)),
            (Primitive::Number(_), PrimitiveID::Bool)
            | (Primitive::String(_), PrimitiveID::Bool) => return Err(ParseError::IncorrectType),
            (Primitive::Bool(b), PrimitiveID::Bool) => return Ok(Primitive::Bool(*b)),
            (Primitive::Bool(_), _) => return Err(ParseError::IncorrectType),
        }
    }

    #[instrument(name = "Primitive::parse", skip_all, err)]
    pub fn parse(tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        match tokens[0] {
            Token::Number(n) => match n.contains('.') {
                true => match n.parse::<f64>() {
                    Ok(f) => Ok((Primitive::Number(Number::F64(f)), 1)),
                    Err(_e) => Err(ParseError::IncorrectType),
                },

                false => match n.parse::<i64>() {
                    Ok(n) => Ok((Primitive::Number(Number::I64(n)), 1)),
                    Err(_e) => Err(ParseError::IncorrectType),
                },
            },
            Token::Str(s) | Token::QuotedString(s) => Ok((Primitive::String(s), 1)),
            Token::True => Ok((Primitive::Bool(true), 1)),
            Token::False => Ok((Primitive::Bool(false), 1)),
            t => panic!("{t:?}"),
        }
    }

    #[instrument(name = "Primitive::parse_ctx", skip_all, err)]
    pub fn parse_ctx(ctx: &Option<TypeID>, tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        match ctx {
            None => Self::parse(tokens),
            Some(id) => {
                match (tokens[0], id) {
                    (Token::Number(num), TypeID::Primitive(primitive_id)) => {
                        return Ok((
                            Primitive::Number(match primitive_id {
                                PrimitiveID::I8 => Number::I8(num.parse::<i8>().unwrap()),
                                PrimitiveID::I16 => Number::I16(num.parse::<i16>().unwrap()),
                                PrimitiveID::I32 => Number::I32(num.parse::<i32>().unwrap()),
                                PrimitiveID::I64 => Number::I64(num.parse::<i64>().unwrap()),
                                PrimitiveID::U8 => Number::U8(num.parse::<u8>().unwrap()),
                                PrimitiveID::U16 => Number::U16(num.parse::<u16>().unwrap()),
                                PrimitiveID::U32 => Number::U32(num.parse::<u32>().unwrap()),
                                PrimitiveID::U64 => Number::U64(num.parse::<u64>().unwrap()),
                                PrimitiveID::F32 => Number::F32(num.parse::<f32>().unwrap()),
                                PrimitiveID::F64 => Number::F64(num.parse::<f64>().unwrap()),
                                PrimitiveID::String => return Err(ParseError::IncorrectType),
                                PrimitiveID::Bool => return Err(ParseError::IncorrectType),
                            }),
                            1,
                        ));
                    }
                    (Token::Str(s), TypeID::Primitive(primitive_id))
                    | (Token::QuotedString(s), TypeID::Primitive(primitive_id)) => {
                        if primitive_id == &PrimitiveID::String {
                            return Ok((Primitive::String(s), 1));
                        } else {
                            return Err(ParseError::IncorrectType);
                        }
                    }
                    (Token::True, TypeID::Primitive(PrimitiveID::Bool)) => {
                        return Ok((Primitive::Bool(true), 1));
                    }
                    (Token::False, TypeID::Primitive(PrimitiveID::Bool)) => {
                        return Ok((Primitive::Bool(false), 1));
                    }
                    t => panic!("{t:?}"),
                };
            }
        }
    }
}

impl<'a> From<Primitive<'a>> for Value<'a> {
    fn from(value: Primitive<'a>) -> Value<'a> {
        Value::Primitive(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum PrimitiveID {
    I8,
    I16,
    I32,
    I64,

    U8,
    U16,
    U32,
    U64,

    F32,
    F64,

    String,

    Bool,
}

impl From<PrimitiveID> for TypeID {
    fn from(value: PrimitiveID) -> Self {
        Self::Primitive(value)
    }
}

impl PrimitiveID {
    pub fn can_fit(&self, other: Self) -> bool {
        info!(?self, ?other);
        matches!(
            (self, other),
            (PrimitiveID::I8, PrimitiveID::I8)
                | (PrimitiveID::I16, PrimitiveID::I8)
                | (PrimitiveID::I16, PrimitiveID::I16)
                | (PrimitiveID::I16, PrimitiveID::U8)
                | (PrimitiveID::I16, PrimitiveID::U16)
                | (PrimitiveID::I32, PrimitiveID::I8)
                | (PrimitiveID::I32, PrimitiveID::I16)
                | (PrimitiveID::I32, PrimitiveID::I32)
                | (PrimitiveID::I32, PrimitiveID::U8)
                | (PrimitiveID::I32, PrimitiveID::U16)
                | (PrimitiveID::I32, PrimitiveID::U32)
                | (PrimitiveID::I64, PrimitiveID::I8)
                | (PrimitiveID::I64, PrimitiveID::I16)
                | (PrimitiveID::I64, PrimitiveID::I32)
                | (PrimitiveID::I64, PrimitiveID::I64)
                | (PrimitiveID::I64, PrimitiveID::U8)
                | (PrimitiveID::I64, PrimitiveID::U16)
                | (PrimitiveID::I64, PrimitiveID::U32)
                | (PrimitiveID::U8, PrimitiveID::I8)
                | (PrimitiveID::U8, PrimitiveID::U8)
                | (PrimitiveID::U16, PrimitiveID::I8)
                | (PrimitiveID::U16, PrimitiveID::I16)
                | (PrimitiveID::U16, PrimitiveID::U8)
                | (PrimitiveID::U16, PrimitiveID::U16)
                | (PrimitiveID::U32, PrimitiveID::I8)
                | (PrimitiveID::U32, PrimitiveID::I16)
                | (PrimitiveID::U32, PrimitiveID::I32)
                | (PrimitiveID::U32, PrimitiveID::U8)
                | (PrimitiveID::U32, PrimitiveID::U16)
                | (PrimitiveID::U32, PrimitiveID::U32)
                | (PrimitiveID::U64, PrimitiveID::I8)
                | (PrimitiveID::U64, PrimitiveID::I16)
                | (PrimitiveID::U64, PrimitiveID::I32)
                | (PrimitiveID::U64, PrimitiveID::I64)
                | (PrimitiveID::U64, PrimitiveID::U8)
                | (PrimitiveID::U64, PrimitiveID::U16)
                | (PrimitiveID::U64, PrimitiveID::U32)
                | (PrimitiveID::U64, PrimitiveID::U64)
                | (PrimitiveID::F32, PrimitiveID::F32)
                | (PrimitiveID::F64, PrimitiveID::F32)
                | (PrimitiveID::F64, PrimitiveID::F64)
                | (PrimitiveID::String, PrimitiveID::String)
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Array<'a> {
    type_id: TypeID,
    values: Vec<Value<'a>>,
}

impl<'a> Array<'a> {
    pub fn new(type_id: TypeID) -> Self {
        Self {
            type_id,
            values: vec![],
        }
    }

    pub fn push(&mut self, value: Value<'a>) {
        match &value {
            Value::Primitive(primitive) => {
                if self.type_id != TypeID::Primitive(primitive.id()) {
                    panic!()
                }
            }
            Value::Complex(_complex_value) => {
                panic!()
            }
        }
        self.values.push(value);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Enum {
    pub(crate) id: ComplexTypeID,
    pub(crate) field: Number,
}

impl Enum {
    #[instrument(name = "Enum::parse_ctx", skip_all, err)]
    pub fn parse_ctx<'a>(ctx: &Typer<'a>, tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        enum State {
            Name,
            Ident,
            Value,
        }

        let mut state = State::Name;
        let mut left = None;
        let mut field = None;

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(left.is_none());
                        left = Some(s);
                    }
                    Token::Comma => left = None,
                    Token::Colon => state = State::Ident,
                    t => panic!("{t:?}"),
                },
                State::Ident => match token {
                    Token::Colon => state = State::Value,
                    t => panic!("{t:?}"),
                },
                State::Value => match token {
                    Token::Str(s) => {
                        if let Some(ComplexType::Known(ComplexTypeDecl::Enum(enu))) =
                            ctx.get(left.unwrap())
                        {
                            field = Some(*enu.fields.get(s).unwrap());
                        } else {
                            panic!()
                        }
                    }
                    Token::Semicolon => {
                        return Ok((
                            Enum {
                                id: ctx.id(left.unwrap()).unwrap(),
                                field: field.unwrap(),
                            },
                            i.saturating_sub(1),
                        ));
                    }
                    t => panic!("{t:?}"),
                },
            }
        }
        panic!("{tokens:?}");
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) enum Visibility {
    #[default]
    Private,
    Pub,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ComplexTypeName<'a> {
    Known(&'a str),
    Unknown(&'a str),
}

impl<'a> Default for ComplexTypeName<'a> {
    fn default() -> Self {
        Self::Unknown("")
    }
}

impl<'a> ComplexTypeName<'a> {
    pub fn name(&self) -> &'a str {
        match self {
            ComplexTypeName::Known(s) | ComplexTypeName::Unknown(s) => s,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StructDecl<'a> {
    pub(crate) visibility: Visibility,

    pub(crate) fields: HashMap<&'a str, TypeID>,
}

impl<'a> StructDecl<'a> {
    #[instrument(name = "StructDecl::parse_ctx_mut", skip_all, err)]
    pub fn parse_ctx_mut(
        ctx: &mut Typer<'a>,
        tokens: &[Token<'a>],
    ) -> ParseResult<(&'a str, Self, usize)> {
        enum State {
            StructDecl,
            Name,
            Ident,
            Type,
        }

        let mut state = State::StructDecl;
        let mut decl = StructDecl::default();
        let mut name = None;
        let mut ident = None;

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::StructDecl => match token {
                    Token::Struct => {
                        state = State::Name;
                    }

                    Token::Pub => decl.visibility = Visibility::Pub,
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(name.is_none());
                        name = Some(s);
                    }
                    Token::LeftAngleBracket => state = State::Ident,

                    t => panic!("{t:?}"),
                },
                State::Ident => match token {
                    Token::Str(s) => {
                        assert!(ident.is_none());
                        ident = Some(s);
                    }
                    Token::Colon => {
                        assert!(ident.is_some());
                        state = State::Type;
                    }
                    Token::Equal => {
                        state = State::Type;
                    }
                    Token::RightAngleBracket => {
                        return Ok((name.unwrap(), decl, i + 1));
                    }
                    t => panic!("{t:?}"),
                },
                State::Type => match token {
                    Token::TypeID(s) => {
                        assert!(
                            decl.fields
                                .insert(ident.unwrap(), TypeID::from(*s))
                                .is_none()
                        );
                        state = State::Type;
                    }
                    Token::Str(s) => {
                        match *s {
                            "string" => assert!(
                                decl.fields
                                    .insert(ident.unwrap(), TypeID::Primitive(PrimitiveID::String))
                                    .is_none()
                            ),
                            "u32" => assert!(
                                decl.fields
                                    .insert(ident.unwrap(), TypeID::Primitive(PrimitiveID::U32))
                                    .is_none()
                            ),
                            _ => {
                                match ctx.id(s) {
                                    Some(id) => {
                                        assert!(
                                            decl.fields
                                                .insert(ident.unwrap(), TypeID::Complex(id))
                                                .is_none()
                                        );
                                    }
                                    None => {
                                        panic!("{s:?}");
                                        // match unknown_types {
                                        //     None => unknown_types = Some(vec![s]),
                                        //     Some(ref mut types) => types.push(s),
                                        // };
                                    }
                                }
                            }
                        };
                        state = State::Type;
                    }
                    Token::RightAngleBracket => {
                        if let Some(ComplexType::Unknown(ComplexTypeDecl::StructDecl(unknown))) =
                            ctx.get(name.unwrap())
                        {
                            let mut matching = 0;
                            for field in &unknown.fields {
                                match decl.fields.get(field.0) {
                                    Some(f) => {
                                        if *f != *field.1 {
                                            match (*f, *field.1) {
                                                (TypeID::Primitive(n), TypeID::Primitive(nn)) => {
                                                    if !nn.can_fit(n) {
                                                        panic!()
                                                    }
                                                }
                                                _ => panic!(),
                                            }
                                        }
                                        matching += 1;
                                    }
                                    None => panic!(),
                                }
                            }
                            if matching != unknown.fields.len() {
                                panic!()
                            }
                            ctx.remove(name.unwrap());
                        }
                        return Ok((name.unwrap(), decl, i + 1));
                    }
                    Token::Comma => {
                        ident = None;
                        state = State::Ident;
                    }
                    t => panic!("{t:?}"),
                },
            }
        }
        Err(ParseError::IncorrectType)
    }
}

#[derive(Debug)]
struct Frame<'a> {
    pending_name: Option<&'a str>,
    name: &'a str,
    fields: Vec<(&'a str, Value<'a>)>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub(crate) struct Struct<'a> {
    pub(crate) name: ComplexTypeName<'a>,
    pub(crate) fields: Vec<(&'a str, Value<'a>)>,
}

impl<'a> Struct<'a> {
    pub fn new(name: ComplexTypeName<'a>, fields: Option<Vec<(&'a str, Value<'a>)>>) -> Self {
        Self {
            name,
            fields: fields.unwrap_or_default(),
        }
    }

    #[instrument(name = "Struct::parse_ctx", skip_all, err)]
    pub fn parse_ctx(ctx: &Typer, tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Name,
            Value,
        }

        let mut stack = vec![Frame {
            pending_name: None,
            name: "",
            fields: vec![],
        }];

        let mut state = State::Name;
        let mut i = 0;

        let mut field_name = None;

        while let Some(token) = tokens.get(i) {
            debug!(?state, ?token);
            match state {
                State::Name => match token {
                    Token::Str(s) => {
                        stack.last_mut().unwrap().name = s;
                        i += 1;
                    }
                    Token::LeftAngleBracket => {
                        i += 1;
                        state = State::Value;
                    }
                    _ => {}
                },
                State::Value => match token {
                    Token::Colon => {
                        i += 1;
                    }
                    Token::Str(s) => {
                        stack.last_mut().unwrap().pending_name = Some(s);

                        if let Some(Token::Colon) = tokens.get(i + 1)
                            && let Some(Token::Str(typ)) = tokens.get(i + 2)
                            && let Some(Token::LeftAngleBracket) = tokens.get(i + 3)
                        {
                            stack.push(Frame {
                                pending_name: None,
                                name: typ,
                                fields: vec![],
                            });
                            i += 2;
                        } else {
                            field_name = Some(*s);
                            i += 1;
                        }
                    }
                    Token::LeftAngleBracket => i += 1,
                    Token::RightAngleBracket => {
                        let finished = stack.pop().unwrap();
                        let comp = Struct {
                            name: ComplexTypeName::Known(finished.name),
                            fields: finished.fields,
                        };
                        if stack.is_empty() {
                            return Ok((comp, i));
                        }

                        // ensures that the type already exists
                        assert!(ctx.get(finished.name).is_some());

                        let paren = stack.last_mut().unwrap();
                        let paren_name = paren.pending_name.take().unwrap();
                        paren.fields.push((
                            paren_name,
                            Value::Complex(crate::types::ComplexValue::Struct(comp)),
                        ));
                        i += 1;
                    }
                    Token::QuotedString(_s) | Token::Number(_s) => {
                        let (val, inc) = Primitive::parse(&[*token])?;
                        stack
                            .last_mut()
                            .unwrap()
                            .fields
                            .push((field_name.unwrap(), Value::Primitive(val)));
                        i += inc;
                    }
                    Token::Comma => {
                        field_name = None;
                        i += 1;
                    }
                    t => warn!(?t, "unhandled token:"),
                },
            }
        }

        Err(ParseError::Unterminated('}'))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct EnumDecl<'a> {
    pub(crate) visibility: Visibility,

    pub(crate) fields: HashMap<&'a str, Number>,
}

impl<'a> EnumDecl<'a> {
    #[instrument(name = "EnumDecl::parse", skip_all, err)]
    pub fn parse(tokens: &[Token<'a>]) -> ParseResult<(&'a str, Self, usize)> {
        #[derive(Debug)]
        enum State {
            Enum,
            Name,
            Ident,
            Value,
        }
        let mut name: Option<&str> = None;
        let mut state = State::Enum;
        let mut ident = None;
        let mut decl = EnumDecl::default();

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Enum => match token {
                    Token::Pub => decl.visibility = Visibility::Pub,
                    Token::Enum => {
                        state = State::Name;
                    }

                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => match name {
                        None => name = Some(s),
                        Some(_) => panic!(),
                    },
                    Token::LeftAngleBracket => {
                        assert!(name.is_some());
                        state = State::Ident;
                    }

                    t => panic!("{t:?}"),
                },
                State::Ident => match token {
                    Token::Str(s) => match ident {
                        None => {
                            assert!(ident.is_none());
                            ident = Some(s);
                        }
                        Some(_) => panic!(),
                    },
                    Token::Comma => {
                        decl.fields
                            .insert(ident.unwrap(), Number::I64(decl.fields.len() as i64));
                        ident = None;
                    }
                    Token::Equal => state = State::Value,
                    Token::RightAngleBracket => {
                        if let Some(iden) = ident {
                            decl.fields
                                .insert(iden, Number::I64(decl.fields.len() as i64));
                        }
                        return Ok((name.unwrap(), decl, i + 1));
                    }

                    t => panic!("{t:?}"),
                },
                State::Value => match token {
                    Token::Comma => {
                        state = State::Ident;
                    }
                    Token::Number(n) => {
                        decl.fields
                            .insert(ident.unwrap(), Number::I64(n.parse::<i64>().unwrap()));
                        ident = None;
                    }
                    Token::RightAngleBracket => return Ok((name.unwrap(), decl, i + 1)),
                    t => panic!("{t:?}"),
                },
            }
        }
        Err(ParseError::IncorrectType)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Variable<'a> {
    pub(crate) typeid: TypeID,
    pub(crate) mutable: bool,
    pub(crate) val: VariableValue<'a>,
}

impl<'a> Variable<'a> {
    fn new(typeid: TypeID, mutable: bool, val: VariableValue<'a>) -> Self {
        Self {
            typeid,
            mutable,
            val,
        }
    }
}

#[derive(Debug)]
struct VariableType {
    typ: TypeID,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Operation {
    Add,
    Sub,
    Mult,
    Div,
    Assign,
    AddAssign,
    SubAssign,
    MultAssign,
    DivAssign,
}

#[derive(Debug, PartialEq)]
pub(crate) enum BlockValue<'a> {
    VariableDecl(Variable<'a>),
    VariableReAssignment(Variable<'a>),
    Block(Vec<BlockValue<'a>>),
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct FunctionDecl<'a> {
    pub(crate) visibility: Visibility,
    pub(crate) name: &'a str,

    pub(crate) args: Option<Vec<(bool, (&'a str, TypeID))>>,
    pub(crate) block: Vec<(&'a str, BlockValue<'a>)>,

    pub(crate) return_type: Option<TypeID>,
}

impl<'a> FunctionDecl<'a> {
    #[instrument(name = "FunctionDecl::parse_ctx", skip_all, err)]
    fn parse_ctx(ctx: &Typer<'a>, tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Fn,
            Name,
            Arg,
            TypeID,
            ReturnType,
            Block,
            Return,
        }

        let mut state = State::Fn;
        let mut decl = FunctionDecl::default();

        let mut found_arg = None;
        let mut mutable = false;

        let mut i = 0;

        loop {
            let Some(token) = tokens.get(i) else {
                break;
            };

            match state {
                State::Fn => match token {
                    Token::Pub => decl.visibility = Visibility::Pub,
                    Token::Function => {
                        state = State::Name;
                    }
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(decl.name.is_empty());
                        decl.name = s;
                    }
                    Token::LeftParen => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::Arg => match token {
                    Token::Mutable => {
                        assert!(!mutable);
                        mutable = true;
                    }
                    Token::Str(s) => {
                        assert!(found_arg.is_none());
                        found_arg = Some(s);
                    }
                    Token::Colon => state = State::TypeID,
                    Token::RightBracket => state = State::Return,
                    Token::RightParen => {
                        state = State::ReturnType;
                    }
                    t => panic!("{t:?}"),
                },
                State::TypeID => match token {
                    Token::TypeID(t) => {
                        match &mut decl.args {
                            Some(args) => {
                                args.push((mutable, (found_arg.unwrap(), TypeID::from(*t))))
                            }
                            None => {
                                decl.args =
                                    Some(vec![(mutable, (found_arg.unwrap(), TypeID::from(*t)))])
                            }
                        }
                        found_arg = None;
                        mutable = false;
                    }
                    Token::Str(s) => match *s {
                        "string" => {
                            match &mut decl.args {
                                Some(args) => args.push((
                                    mutable,
                                    (found_arg.unwrap(), TypeID::Primitive(PrimitiveID::String)),
                                )),
                                None => {
                                    decl.args = Some(vec![(
                                        mutable,
                                        (
                                            found_arg.unwrap(),
                                            TypeID::Primitive(PrimitiveID::String),
                                        ),
                                    )])
                                }
                            }
                            found_arg = None;
                            mutable = false;
                        }
                        _ => panic!("{s:?}"),
                    },
                    Token::RightParen => state = State::ReturnType,
                    Token::Comma => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::ReturnType => match token {
                    Token::TypeID(t) => decl.return_type = Some(TypeID::from(*t)),
                    Token::LeftAngleBracket => state = State::Block,
                    t => panic!("{t:?}"),
                },

                State::Block => match token {
                    Token::RightAngleBracket => {
                        return Ok((decl, i));
                    }
                    Token::Return => state = State::Return,
                    Token::Str(s) => {
                        let (var_name, inc) = VariableUse::parse_ctx(ctx, &tokens[i..])?;
                        let mut block_val = None;
                        for (name, val) in &decl.block {
                            if s == name {
                                block_val = Some((name, val));
                                match val {
                                    BlockValue::VariableDecl(variable) => {
                                        mutable = variable.mutable
                                    }
                                    t => panic!("{t:?}"),
                                }
                                break;
                            }
                        }
                        let block_val = block_val.unwrap();

                        match var_name {
                            VariableValueReturn::Assignment(ref variable_value) => {
                                match variable_value {
                                    VariableValue::Value(value) => {
                                        decl.block.push((
                                            s,
                                            BlockValue::VariableReAssignment(Variable {
                                                typeid: value.id(Some(ctx)).unwrap(),
                                                mutable,
                                                val: VariableValue::Value(value.clone()),
                                            }),
                                        ));
                                    }
                                    VariableValue::Name(_var_name) => match block_val {
                                        (_nam, BlockValue::VariableDecl(variable)) => {
                                            decl.block.push((
                                                s,
                                                BlockValue::VariableDecl(Variable {
                                                    typeid: variable.typeid,
                                                    mutable,
                                                    val: match &variable.val {
                                                        VariableValue::Value(value) => {
                                                            VariableValue::Value(value.clone())
                                                        }
                                                        VariableValue::Name(_) => panic!(),
                                                        VariableValue::Expr(_) => panic!(),
                                                    },
                                                }),
                                            ));
                                        }
                                        t => panic!("{t:?}"),
                                    },
                                    VariableValue::Expr(expr) => {
                                        let id = match &expr[0] {
                                            MathItem::Prim(primitive) => primitive.id(),
                                            MathItem::Op(operation) => todo!(),
                                        };
                                        decl.block.push((
                                            s,
                                            BlockValue::VariableDecl(Variable {
                                                typeid: TypeID::Primitive(id),
                                                mutable,
                                                val: VariableValue::Expr(expr.clone()),
                                            }),
                                        ));
                                    }
                                }
                            }
                            VariableValueReturn::ReAssignment(variable_value) => todo!(),
                            VariableValueReturn::Expr(math_items) => todo!(),
                        }
                        i += inc;
                        info!("{var_name:?} toks {:?}", &tokens[i..]);
                    }
                    Token::Let => {
                        let (name, mutable, id, value, inc) =
                            Variable::parse_ctx(ctx, &tokens[i..])?;

                        i += inc;
                        let block_val = decl.block.last();

                        let id = match id {
                            Some(id) => Some(id),
                            None => match &value {
                                VariableValue::Value(value) => value.id(Some(ctx)),
                                VariableValue::Name(_var_name) => match block_val {
                                    Some((_, BlockValue::VariableDecl(variable))) => {
                                        Some(variable.typeid)
                                    }
                                    t => panic!("{t:?}"),
                                },
                                VariableValue::Expr(_expr) => None,
                            },
                        };

                        match value {
                            VariableValue::Value(value) => {
                                decl.block.push((
                                    name,
                                    BlockValue::VariableDecl(Variable {
                                        typeid: id.unwrap(),
                                        mutable,
                                        val: VariableValue::Value(value),
                                    }),
                                ));
                            }
                            VariableValue::Name(_var_name) => match block_val {
                                Some((_nam, BlockValue::VariableDecl(variable))) => {
                                    decl.block.push((
                                        name,
                                        BlockValue::VariableDecl(Variable {
                                            typeid: variable.typeid,
                                            mutable,
                                            val: match &variable.val {
                                                VariableValue::Value(value) => {
                                                    VariableValue::Value(value.clone())
                                                }
                                                VariableValue::Name(_) => panic!(),
                                                VariableValue::Expr(_) => panic!(),
                                            },
                                        }),
                                    ));
                                }
                                t => panic!("{t:?}"),
                            },
                            VariableValue::Expr(expr) => {
                                decl.block.push((
                                    name,
                                    BlockValue::VariableDecl(Variable {
                                        typeid: id.unwrap(),
                                        mutable,
                                        val: VariableValue::Expr(expr),
                                    }),
                                ));
                            }
                        }
                    }
                    t => panic!("{t:?}"),
                },
                State::Return => {
                    if decl.return_type.is_none() {
                        panic!()
                    }
                    match token {
                        Token::Str(s) => {
                            for (name, val) in &decl.block {
                                if name == s
                                    && let BlockValue::VariableDecl(variable) = val
                                    && decl.return_type.unwrap() != variable.typeid
                                {
                                    panic!()
                                }
                            }
                        }
                        Token::Number(_n) | Token::QuotedString(_n) => {
                            let (prim, _i) = Primitive::parse_ctx(&decl.return_type, &tokens[i..])?;
                            let prim_id = prim.id();
                            match (decl.return_type, prim_id) {
                                (Some(val_id), prim_id) => match val_id {
                                    TypeID::Primitive(primitive_id) => {
                                        if !primitive_id.can_fit(prim_id) {
                                            panic!("{val_id:?} {prim_id:?}");
                                        }
                                    }
                                    TypeID::Complex(_complex_type_id) => todo!(),
                                },
                                t => panic!("{t:?}"),
                            }
                        }

                        Token::Semicolon => {
                            return Ok((decl, i));
                        }
                        t => panic!("{t:?}"),
                    }
                }
            }

            i += 1;
        }

        panic!()
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct FunctionCall<'a> {
    pub(crate) name: &'a str,
    pub(crate) args: Vec<Value<'a>>,
}

impl<'a> FunctionCall<'a> {
    #[instrument(name = "FunctionCall::parse_ctx", skip_all, err)]
    pub fn parse_ctx(_ctx: &(), tokens: &[Token<'a>]) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Name,
            Arg,
            FnEnd,
        }
        let mut fn_name: Option<&str> = None;
        let mut args: Vec<Value> = vec![];
        let mut state = State::Name;
        let mut needs_comma = false;

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(fn_name.is_none());
                        fn_name = Some(s);
                    }
                    Token::LeftParen => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::Arg => match token {
                    Token::Number(_s) | Token::QuotedString(_s) => {
                        if needs_comma {
                            panic!()
                        }
                        let (val, _idx) = Primitive::parse(&tokens[i..])?;
                        args.push(Value::Primitive(val));
                        needs_comma = true;
                    }
                    Token::Comma => needs_comma = false,
                    Token::Str(s) => todo!("{s:?}"),
                    Token::RightParen => state = State::FnEnd,
                    t => panic!("{t:?}"),
                },
                State::FnEnd => match token {
                    Token::Semicolon => {
                        return Ok((
                            FunctionCall {
                                name: fn_name.unwrap(),
                                args,
                            },
                            i,
                        ));
                    }
                    t => panic!("{t:?}"),
                },
            }
        }

        panic!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ComplexTypeDecl<'a> {
    StructDecl(StructDecl<'a>),
    Enum(EnumDecl<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum ComplexValue<'a> {
    Struct(Struct<'a>),
    Enum(Enum),
}

impl<'a> ComplexValue<'a> {
    fn as_decl(&'a self, typer: &'a Typer) -> ComplexTypeDecl<'a> {
        match self {
            ComplexValue::Struct(struc) => {
                let decl = struc
                    .fields
                    .iter()
                    .map(|f| (f.0, f.1.id(None).unwrap()))
                    .collect::<HashMap<&'a str, TypeID>>();
                ComplexTypeDecl::StructDecl(StructDecl {
                    visibility: Visibility::Private,
                    fields: decl,
                })
            }
            ComplexValue::Enum(_) => todo!(),
        }
    }
    fn id(&'a self, typer: &'a Typer) -> Option<&'a ComplexType<'a>> {
        match self {
            ComplexValue::Struct(struc) => {
                info!(?typer);
                typer.get(struc.name.name())
            }
            ComplexValue::Enum(_) => todo!(),
        }
    }
}

impl<'a> From<ComplexValue<'a>> for Value<'a> {
    #[inline]
    fn from(value: ComplexValue<'a>) -> Self {
        Value::Complex(value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Value<'a> {
    Primitive(Primitive<'a>),
    Complex(ComplexValue<'a>),
}

impl<'a> Value<'a> {
    pub fn id(&self, typer: Option<&Typer>) -> Option<TypeID> {
        match self {
            Value::Primitive(prim) => Some(TypeID::Primitive(prim.id())),
            Value::Complex(v) => {
                let typer = typer?;

                match v {
                    ComplexValue::Struct(struc) => typer.id(struc.name.name()).map(TypeID::Complex),

                    ComplexValue::Enum(enu) => Some(TypeID::Complex(enu.id)),
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum TypeDeclReturn<'a> {
    Enum(EnumDecl<'a>),
    Struct(StructDecl<'a>),
    Function(FunctionDecl<'a>),
}

#[derive(Debug)]
pub(crate) struct TypeDecl();

impl TypeDecl {
    #[instrument(name = "TypeDecl::parse_ctx_mut", skip_all, err)]
    pub fn parse_ctx_mut<'a>(
        ctx: &mut Typer<'a>,
        tokens: &[Token<'a>],
    ) -> ParseResult<(
        Option<&'a str>,
        TypeDeclReturn<'a>,
        Option<Vec<&'a str>>,
        usize,
    )> {
        let mut start = 0;
        let vis = if tokens[0] == Token::Pub {
            start = 1;
            Visibility::Pub
        } else {
            Visibility::Private
        };
        match tokens[start] {
            Token::Function => {
                let (mut decl, i) = FunctionDecl::parse_ctx(ctx, tokens)?;
                decl.visibility = vis;

                Ok((None, TypeDeclReturn::Function(decl), None, i))
            }
            Token::Struct => {
                let (name, mut decl, i) = StructDecl::parse_ctx_mut(ctx, tokens)?;
                decl.visibility = vis;

                Ok((Some(name), TypeDeclReturn::Struct(decl), None, i))
            }
            Token::Enum => {
                let (name, mut decl, i) = EnumDecl::parse(tokens)?;
                decl.visibility = vis;

                Ok((Some(name), TypeDeclReturn::Enum(decl), None, i))
            }
            t => panic!("{t:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VariableValue<'a> {
    Value(Value<'a>),
    Name(&'a str),
    Expr(Vec<MathItem<'a>>),
}

impl<'a> VariableValue<'a> {
    #[instrument(name = "VariableValue::parse_ctx", skip_all, ret)]
    pub fn parse_ctx(
        ctx: &(Option<TypeID>, Typer<'a>),
        tokens: &[Token<'a>],
    ) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Value,
        }

        let mut value: Option<VariableValue> = None;

        let type_id = &ctx.0;
        let typer = &ctx.1;
        let mut i = 0;
        info!(?tokens);

        while let Some(token) = tokens.get(i) {
            match token {
                Token::Str(s) => match typer.get(s) {
                    Some(typ) => match typ {
                        ComplexType::Known(complex_type_decl) => match complex_type_decl {
                            ComplexTypeDecl::StructDecl(_struct_decl) => {
                                let (decl, inc) = Struct::parse_ctx(typer, &tokens[i..])?;
                                i += inc;
                                value = Some(VariableValue::Value(Value::Complex(
                                    ComplexValue::Struct(decl),
                                )));
                                return Ok((value.unwrap(), i));
                            }
                            ComplexTypeDecl::Enum(_enum_decl) => {
                                let (decl, inc) = Enum::parse_ctx(&ctx.1, &tokens[i..])?;
                                i += inc;
                                value = Some(VariableValue::Value(Value::Complex(
                                    ComplexValue::Enum(decl),
                                )));
                                return Ok((value.unwrap(), i));
                            }
                        },
                        ComplexType::Unknown(_complex_type_decl) => todo!(),
                    },
                    None => {
                        value = Some(VariableValue::Name(s));
                        return Ok((value.unwrap(), i));
                    }
                },
                Token::Number(_) | Token::QuotedString(_) | Token::True | Token::False => {
                    let (prim, inc) = Primitive::parse_ctx(type_id, &tokens[i..])?;
                    value = Some(VariableValue::Value(Value::Primitive(prim)));
                    match tokens.get(i + 1) {
                        Some(token) => match token {
                            Token::Plus | Token::Minus | Token::Multiply | Token::Divide => {
                                panic!("{token:?} {:?}", &tokens[i..])
                            }
                            Token::LeftCarrot => todo!("potential less than expr"),
                            Token::RightCarrot => todo!("potential greater than expr"),
                            _ => return Ok((value.unwrap(), i + inc)),
                        },
                        _ => return Ok((value.unwrap(), i)),
                    }
                }
                t => panic!("{t:?}"),
            }
        }

        Err(ParseError::IncorrectType)
    }
}

impl<'a> Variable<'a> {
    #[instrument(name = "Variable::parse_ctx", skip_all, err)]
    pub fn parse_ctx(
        ctx: &Typer<'a>,
        tokens: &[Token<'a>],
    ) -> ParseResult<(&'a str, bool, Option<TypeID>, VariableValue<'a>, usize)> {
        enum VariableState {
            Let,
            Name,
            Type,
            Value,
            Semicolon,
        }

        let mut state = VariableState::Let;
        let mut name = None;
        let mut mutable = false;
        let mut type_id: Option<TypeID> = None;
        let mut value: Option<VariableValue> = None;
        let mut i = 0;

        loop {
            let Some(token) = tokens.get(i) else {
                break;
            };

            match state {
                VariableState::Let => match token {
                    Token::Let => state = VariableState::Name,
                    _ => panic!(),
                },
                VariableState::Name => match token {
                    Token::Mutable => mutable = true,
                    Token::Str(s) => {
                        assert!(name.is_none());
                        name = Some(s);
                    }
                    Token::Equal => state = VariableState::Value,
                    Token::Colon => state = VariableState::Type,
                    t => panic!("{t:?}"),
                },
                VariableState::Type => match token {
                    Token::TypeID(id) => {
                        type_id = Some(TypeID::from(*id));
                    }
                    Token::Equal => state = VariableState::Value,
                    t => panic!("{t:?}"),
                },
                VariableState::Value => {
                    let (left, inc) = VariableUse::parse_ctx(ctx, &tokens[i..])?;
                    i += inc;
                    info!("{:?}", &tokens[i..]);
                    value = match left {
                        VariableValueReturn::Assignment(variable_value) => Some(variable_value),
                        VariableValueReturn::ReAssignment(variable_value) => Some(variable_value),
                        VariableValueReturn::Expr(math_items) => {
                            type_id = Some(TypeID::Primitive(match &math_items[0] {
                                MathItem::Prim(primitive) => primitive.id(),
                                MathItem::Op(_operation) => todo!(),
                            }));
                            Some(VariableValue::Expr(math_items))
                        }
                    };
                    // info!(?value);
                    state = VariableState::Semicolon;
                    // continue because without it, we finish the loop incrementing i but we are on
                    // the semicolon
                    continue;
                }
                VariableState::Semicolon => match token {
                    Token::Semicolon => {
                        info!(?name, ?value);
                        return Ok((name.unwrap(), mutable, type_id, value.unwrap(), i));
                    }
                    t => panic!("{t:?} {:?}", &tokens[i..]),
                },
            }
            i += 1;
        }

        panic!()
    }
}

#[derive(Debug)]
struct MathExpr;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MathItem<'a> {
    Prim(Primitive<'a>),
    Op(Operation),
}

impl<'a> MathExpr {
    #[instrument(name = "MathExpr::parse", skip_all, err)]
    pub fn parse(tokens: &[Token<'a>]) -> ParseResult<(Vec<MathItem<'a>>, usize)> {
        let mut i = 0;
        let mut items = vec![];
        while let Some(token) = tokens.get(i) {
            match token {
                Token::Plus => items.push(MathItem::Op(Operation::Add)),
                Token::Minus => items.push(MathItem::Op(Operation::Sub)),
                Token::Multiply => items.push(MathItem::Op(Operation::Mult)),
                Token::Divide => items.push(MathItem::Op(Operation::Div)),

                Token::Number(_n) => items.push(MathItem::Prim(Primitive::parse(&[*token])?.0)),
                Token::Semicolon => return Ok((items, i.saturating_sub(1))),
                t => panic!("{t:?}"),
            }
            i += 1;
        }

        Err(ParseError::IncorrectType)
    }
}

#[derive(Debug)]
struct VariableUse;

#[derive(Debug)]
enum VariableValueReturn<'a> {
    Assignment(VariableValue<'a>),
    ReAssignment(VariableValue<'a>),
    Expr(Vec<MathItem<'a>>),
}

impl VariableUse {
    #[instrument(name = "VariableUse::parse_ctx", skip_all, ret)]
    pub fn parse_ctx<'a>(
        _ctx: &Typer<'a>,
        tokens: &[Token<'a>],
    ) -> ParseResult<(VariableValueReturn<'a>, usize)> {
        #[derive(Debug)]
        enum State {
            Name,
            Operator,
        }

        let mut state = State::Operator;

        let mut name = None;
        let mut op = Some(Operation::Assign);
        let mut value = None;
        let mut i = 0;

        while let Some(tok) = tokens.get(i) {
            match state {
                State::Name => match tok {
                    Token::Str(s) => {
                        match name {
                            None => {
                                name = Some(VariableValue::Name(s));
                            }
                            Some(_) => {
                                assert!(value.is_none());
                                value = Some(VariableValue::Name(s));
                            }
                        }
                        state = State::Operator;
                        i += 1;
                    }
                    Token::Number(_n) => {
                        let prim =
                            VariableValue::Value(Value::Primitive(Primitive::parse(&[*tok])?.0));
                        match name {
                            None => {
                                name = Some(prim);
                            }
                            Some(_) => {
                                assert!(value.is_none());
                                value = Some(prim);
                            }
                        }
                        state = State::Operator;
                        i += 1;
                    }
                    Token::Semicolon => {
                        panic!("{i:?}");
                        return Ok((VariableValueReturn::Assignment(name.unwrap()), i));
                    }
                    t => panic!("{t:?} {:?}", tokens),
                },
                State::Operator => {
                    match (tok, tokens.get(i + 1)) {
                        // (Token::Plus, Some(Token::Equal)) => {
                        //     op = Some(Operation::AddAssign);
                        //     i += 2;
                        // }
                        // (Token::Minus, Some(Token::Equal)) => {
                        //     op = Some(Operation::SubAssign);
                        //     i += 2;
                        // }
                        // (Token::Multiply, Some(Token::Equal)) => {
                        //     op = Some(Operation::MultAssign);
                        //     i += 2;
                        // }
                        // (Token::Divide, Some(Token::Equal)) => {
                        //     op = Some(Operation::DivAssign);
                        //     i += 2;
                        // }
                        (Token::Equal, _) => {
                            op = Some(Operation::Assign);
                            i += 1;
                        }
                        // (Token::Plus, _) => {
                        //     op = Some(Operation::Add);
                        //     i += 1;
                        // }
                        // (Token::Minus, _) => {
                        //     op = Some(Operation::Sub);
                        //     i += 1;
                        // }
                        // (Token::Multiply, _) => {
                        //     op = Some(Operation::Mult);
                        //     i += 1;
                        // }
                        // (Token::Divide, _) => {
                        //     op = Some(Operation::Div);
                        //     i += 1;
                        // }
                        (Token::Exclamation, Some(Token::Equal)) => {
                            todo!("should this lang support things like let foo = bar != baz")
                        }
                        (Token::Number(_n), Some(Token::Semicolon))
                        | (Token::QuotedString(_n), Some(Token::Semicolon)) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            let prim = VariableValue::Value(Value::Primitive(
                                Primitive::parse(&[*tok])?.0,
                            ));
                            return Ok((VariableValueReturn::Assignment(prim), i + 1));
                        }
                        (Token::Str(_s), Some(Token::Equal)) => {
                            let (val, inc) =
                                VariableValue::parse_ctx(&(None, _ctx.clone()), &tokens[i + 2..])?;
                            // [Token::Str(_), Token::Equal, Val, Token::Semicolon];
                            // ^ start         ^ end         ^ goal
                            // so we do i += inc + 2;
                            i += inc + 2;
                            return Ok((VariableValueReturn::Assignment(val), i));
                        }
                        (Token::Str(_s), _) => {
                            let (val, inc) =
                                VariableValue::parse_ctx(&(None, _ctx.clone()), &tokens[i..])?;
                            // [Token::Str(_), Token::Equal, Val, Token::Semicolon];
                            // ^ start         ^ end         ^ goal
                            // so we do i += inc + 2;
                            i += inc + 1;
                            return Ok((VariableValueReturn::Assignment(val), i));
                        }
                        (Token::Number(_n), Some(Token::Plus))
                        | (Token::Number(_n), Some(Token::Minus))
                        | (Token::Number(_n), Some(Token::Multiply))
                        | (Token::Number(_n), Some(Token::Divide)) => {
                            let (val, inc) = MathExpr::parse(&tokens[i..])?;
                            return Ok((VariableValueReturn::Expr(val), i + inc + 1));
                        }
                        (Token::True, Some(Token::Semicolon)) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            return Ok((
                                VariableValueReturn::Assignment(VariableValue::Value(
                                    Value::Primitive(Primitive::Bool(true)),
                                )),
                                i + 1,
                            ));
                        }
                        (Token::False, Some(Token::Semicolon)) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            return Ok((
                                VariableValueReturn::Assignment(VariableValue::Value(
                                    Value::Primitive(Primitive::Bool(false)),
                                )),
                                i + 1,
                            ));
                        }

                        t => {
                            panic!("{t:?} {op:?}");
                        }
                    };
                    state = State::Name;
                }
            }
        }

        Err(ParseError::IncorrectType)
    }
}

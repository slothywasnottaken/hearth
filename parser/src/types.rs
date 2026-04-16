#![allow(unused)]

use std::{
    collections::{HashMap, HashSet, hash_map::Iter},
    default,
    fmt::{Debug, Display},
    num::{ParseIntError, TryFromIntError},
    ops::Add,
};

use tracing::{debug, error, info, instrument, trace, warn};

use crate::parser::{ParseError, ParseResult};

use tokenizer::{Span, Token};

#[derive(Debug, Clone)]
pub struct Typer<'a> {
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

#[derive(Debug, Clone, PartialEq)]
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

impl From<tokenizer::TypeID> for TypeID {
    fn from(value: tokenizer::TypeID) -> Self {
        match value {
            tokenizer::TypeID::I8 => Self::Primitive(PrimitiveID::I8),
            tokenizer::TypeID::I16 => Self::Primitive(PrimitiveID::I16),
            tokenizer::TypeID::I32 => Self::Primitive(PrimitiveID::I32),
            tokenizer::TypeID::I64 => Self::Primitive(PrimitiveID::I64),
            tokenizer::TypeID::U8 => Self::Primitive(PrimitiveID::U8),
            tokenizer::TypeID::U16 => Self::Primitive(PrimitiveID::U16),
            tokenizer::TypeID::U32 => Self::Primitive(PrimitiveID::U32),
            tokenizer::TypeID::U64 => Self::Primitive(PrimitiveID::U64),
            tokenizer::TypeID::F32 => Self::Primitive(PrimitiveID::F32),
            tokenizer::TypeID::F64 => Self::Primitive(PrimitiveID::F64),
            tokenizer::TypeID::String => Self::Primitive(PrimitiveID::String),
            tokenizer::TypeID::QuotedString => Self::Primitive(PrimitiveID::String),
            tokenizer::TypeID::Bool => Self::Primitive(PrimitiveID::Bool),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum Number {
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
pub enum Primitive<'a> {
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
pub struct Array<'a> {
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
pub struct Enum {
    pub(crate) id: ComplexTypeID,
    pub(crate) field: Number,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Visibility {
    #[default]
    Private,
    Pub,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComplexTypeName<'a> {
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
pub struct StructDecl<'a> {
    pub(crate) visibility: Visibility,

    pub(crate) fields: HashMap<&'a str, TypeID>,
}

#[derive(Debug)]
pub(crate) struct Frame<'a> {
    pub(crate) pending_name: Option<&'a str>,
    pub(crate) name: &'a str,
    pub(crate) fields: Vec<(usize, &'a str, Value<'a>)>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Struct<'a> {
    pub(crate) name: ComplexTypeName<'a>,
    pub(crate) fields: Vec<(usize, &'a str, Value<'a>)>,
}

impl<'a> Struct<'a> {
    pub fn new(
        name: ComplexTypeName<'a>,
        fields: Option<Vec<(usize, &'a str, Value<'a>)>>,
    ) -> Self {
        Self {
            name,
            fields: fields.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnumDecl<'a> {
    pub(crate) visibility: Visibility,

    pub(crate) fields: HashMap<&'a str, Number>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'a> {
    pub(crate) typeid: TypeID,
    pub(crate) mutable: bool,
    pub(crate) val: VariableValue<'a>,
}

impl<'a> Variable<'a> {
    pub fn new(typeid: TypeID, mutable: bool, val: VariableValue<'a>) -> Self {
        Self {
            typeid,
            mutable,
            val,
        }
    }

    pub fn from_value(value: Value<'a>, mutable: bool, typer: Option<&Typer<'a>>) -> Self {
        let typeid = match &value {
            Value::Primitive(primitive) => TypeID::Primitive(primitive.id()),
            Value::Complex(complex_value) => TypeID::Complex(match complex_value {
                ComplexValue::Struct(decl) => typer.unwrap().id(decl.name.name()).unwrap(),
                ComplexValue::Enum(enu) => enu.id,
            }),
        };

        Self {
            typeid,
            mutable,
            val: VariableValue::Value(value),
        }
    }
}

#[derive(Debug)]
pub struct VariableType {
    typ: TypeID,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
    Assign,
    AddAssign,
    SubAssign,
    MultAssign,
    DivAssign,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Condition {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessthanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConditionItem<'a> {
    Item(VariableValue<'a>),
    Condition(Condition),
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfStatement<'a> {
    pub(crate) cond: Vec<ConditionItem<'a>>,
    pub(crate) block: Block<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ElseIfStatement<'a> {
    pub(crate) cond: Vec<ConditionItem<'a>>,
    pub(crate) block: Block<'a>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Else<'a> {
    pub(crate) block: Block<'a>,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Block<'a> {
    pub(crate) values: Vec<BlockValue<'a>>,
}

impl<'a> Block<'a> {
    pub fn push(&mut self, value: BlockValue<'a>) {
        self.values.push(value);
    }

    pub fn pop(&mut self) -> Option<BlockValue<'a>> {
        self.values.pop()
    }

    pub const fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter_mut(&'_ mut self) -> core::slice::IterMut<'_, BlockValue<'a>> {
        self.values.iter_mut()
    }

    pub fn iter(&'_ self) -> core::slice::Iter<'_, BlockValue<'a>> {
        self.values.iter()
    }

    pub fn last(&self) -> Option<&BlockValue<'a>> {
        self.values.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut BlockValue<'a>> {
        self.values.last_mut()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BlockValue<'a> {
    VariableDecl((&'a str, Variable<'a>)),
    VariableReAssignment((&'a str, VariableValue<'a>)),
    Return(VariableValue<'a>),
    IfStatement(IfStatement<'a>),
    Else(Block<'a>),
    ElseIf(ElseIfStatement<'a>),
    Block(Block<'a>),
    FunctionCall(FunctionCall<'a>),
}

#[derive(Debug, Default, PartialEq)]
pub struct FunctionDecl<'a> {
    pub(crate) visibility: Visibility,
    pub(crate) name: &'a str,
    pub(crate) args: Option<Vec<(bool, &'a str, TypeID)>>,
    pub(crate) return_type: Option<TypeID>,

    pub(crate) block: Vec<(usize, BlockValue<'a>)>,
}

impl<'a> FunctionDecl<'a> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn args(&self) -> Option<&[(bool, &'a str, TypeID)]> {
        self.args.as_deref()
    }

    pub fn block(&self) -> &Vec<(usize, BlockValue<'a>)> {
        &self.block
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionCall<'a> {
    pub(crate) name: &'a str,
    pub(crate) args: Vec<Value<'a>>,
    pub(crate) return_type: Option<TypeID>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplexTypeDecl<'a> {
    StructDecl(StructDecl<'a>),
    Enum(EnumDecl<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ComplexValue<'a> {
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
                    .map(|f| (f.1, f.2.id(None).unwrap()))
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
pub enum Value<'a> {
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
pub enum TypeDeclReturn<'a> {
    Enum(EnumDecl<'a>),
    Struct(StructDecl<'a>),
    Function(FunctionDecl<'a>),
}

#[derive(Debug)]
pub(crate) struct TypeDecl();

#[derive(Debug, Clone, PartialEq)]
pub enum VariableValue<'a> {
    StructAccess(Vec<(&'a str, &'a str)>),
    Value(Value<'a>),
    Name(&'a str),
    Expr(Vec<MathItem<'a>>),
    FunctionCall(FunctionCall<'a>),
}

#[derive(Debug)]
pub(crate) struct MathExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum MathItem<'a> {
    Prim(Primitive<'a>),
    Op(Operation),
}

#[derive(Debug)]
pub(crate) struct VariableUse;

#[derive(Debug)]
pub enum VariableValueReturn<'a> {
    Assignment(VariableValue<'a>),
    ReAssignment(VariableValue<'a>),
    Expr(Vec<MathItem<'a>>),
}

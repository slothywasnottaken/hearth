#![allow(unused)]

use std::{
    collections::{HashMap, HashSet, hash_map::Iter},
    default,
    error::Error,
    fmt::{Debug, Display, format},
    num::{ParseIntError, TryFromIntError},
    ops::Add,
};

use tracing::{debug, trace};

use crate::parser::{ParseError, ParseResult};

use tokenizer::{Span, Token};

#[derive(Debug, Clone)]
pub struct Typer<'a> {
    types: HashMap<&'a str, ComplexTypeID>,
    type_ids: HashMap<ComplexTypeID, ComplexType<'a>>,
    next_id: ComplexTypeID,
}

#[derive(Debug, Hash, PartialEq, PartialOrd, Eq, Clone, Copy)]
pub struct ComplexTypeID {
    id: usize,
}

impl ComplexTypeID {
    pub(crate) fn new(id: usize) -> Self {
        Self { id }
    }
}

impl Display for ComplexTypeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl Default for Typer<'_> {
    fn default() -> Self {
        assert!(PrimitiveID::Bool as usize == 11);
        Self {
            types: HashMap::default(),
            type_ids: HashMap::default(),
            next_id: ComplexTypeID { id: 12 },
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
        debug!(?self, ?type_name, ?value);
        assert!(self.types.insert(type_name, self.next_id).is_none());
        assert!(
            self.type_ids
                .insert(self.next_id, ComplexType::Unknown(value))
                .is_none()
        );
        self.next_id.id += 1;
    }

    pub fn register(&mut self, type_name: &'a str, value: ComplexTypeDecl<'a>) {
        debug!(?self, ?type_name, ?value);
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

impl ComplexType<'_> {
    pub fn inner(&self) -> &ComplexTypeDecl<'_> {
        match self {
            Self::Known(s) | Self::Unknown(s) => s,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone, Copy)]
pub struct TypeId {
    pub(crate) id: usize,
    pub(crate) array: bool,
}

impl TypeId {
    pub fn as_primitive(&self) -> PrimitiveID {
        match self.id {
            0 => PrimitiveID::I8,
            1 => PrimitiveID::I16,
            2 => PrimitiveID::I32,
            3 => PrimitiveID::I64,

            4 => PrimitiveID::U8,
            5 => PrimitiveID::U16,
            6 => PrimitiveID::U32,
            7 => PrimitiveID::U64,

            8 => PrimitiveID::F32,
            9 => PrimitiveID::F64,

            10 => PrimitiveID::String,

            11 => PrimitiveID::Bool,
            t => panic!("{t:?}"),
        }
    }

    pub fn as_complex(&self) -> ComplexTypeID {
        match self.id {
            0..=11 => panic!("Expected complex type, found primitive"),
            _ => ComplexTypeID { id: self.id },
        }
    }
}

impl Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = match self.id {
            0 => PrimitiveID::I8,
            1 => PrimitiveID::I16,
            2 => PrimitiveID::I32,
            3 => PrimitiveID::I64,

            4 => PrimitiveID::U8,
            5 => PrimitiveID::U16,
            6 => PrimitiveID::U32,
            7 => PrimitiveID::U64,

            8 => PrimitiveID::F32,
            9 => PrimitiveID::F64,

            10 => PrimitiveID::String,

            11 => PrimitiveID::Bool,
            t => panic!("{t:?}"),
        };

        write!(f, "{id}")
    }
}

impl From<PrimitiveID> for TypeId {
    fn from(value: PrimitiveID) -> Self {
        let id = match value {
            PrimitiveID::I8 => 0,
            PrimitiveID::I16 => 1,
            PrimitiveID::I32 => 2,
            PrimitiveID::I64 => 3,
            PrimitiveID::U8 => 4,
            PrimitiveID::U16 => 5,
            PrimitiveID::U32 => 6,
            PrimitiveID::U64 => 7,
            PrimitiveID::F32 => 8,
            PrimitiveID::F64 => 9,
            PrimitiveID::String => 10,
            PrimitiveID::Bool => 11,
        };

        Self { id, array: false }
    }
}

impl From<ComplexTypeID> for TypeId {
    fn from(value: ComplexTypeID) -> Self {
        Self {
            id: (PrimitiveID::Bool as usize) + value.id,
            array: false,
        }
    }
}

impl From<tokenizer::TypeID> for TypeId {
    fn from(value: tokenizer::TypeID) -> Self {
        match value {
            tokenizer::TypeID::I8 => PrimitiveID::I8.into(),
            tokenizer::TypeID::I16 => PrimitiveID::I16.into(),
            tokenizer::TypeID::I32 => PrimitiveID::I32.into(),
            tokenizer::TypeID::I64 => PrimitiveID::I64.into(),
            tokenizer::TypeID::U8 => PrimitiveID::U8.into(),
            tokenizer::TypeID::U16 => PrimitiveID::U16.into(),
            tokenizer::TypeID::U32 => PrimitiveID::U32.into(),
            tokenizer::TypeID::U64 => PrimitiveID::U64.into(),
            tokenizer::TypeID::F32 => PrimitiveID::F32.into(),
            tokenizer::TypeID::F64 => PrimitiveID::F64.into(),
            tokenizer::TypeID::String | tokenizer::TypeID::QuotedString => {
                PrimitiveID::String.into()
            }
            tokenizer::TypeID::Bool => PrimitiveID::Bool.into(),
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

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::I8(num) => write!(f, "{num}"),
            Number::I16(num) => write!(f, "{num}"),
            Number::I32(num) => write!(f, "{num}"),
            Number::I64(num) => write!(f, "{num}"),
            Number::U8(num) => write!(f, "{num}"),
            Number::U16(num) => write!(f, "{num}"),
            Number::U32(num) => write!(f, "{num}"),
            Number::U64(num) => write!(f, "{num}"),
            Number::F32(num) => write!(f, "{num}"),
            Number::F64(num) => write!(f, "{num}"),
        }
    }
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

impl From<Number> for Primitive<'_> {
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

impl Display for Primitive<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Primitive::Number(number) => write!(f, "{number}"),
            Primitive::String(val) => write!(f, "{val}"),
            Primitive::Bool(val) => write!(f, "{val}"),
        }
    }
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
            Self::String(_) | Self::Bool(_) => None,
        }
    }

    /// dumb stupid hack to change struct fields in variable assignment (auto set to i64/f64/string) and convert it to the type declared in the struct decl
    // maps Foo {i:i32} -> let foo = Foo {i:0} (0 is auto set as i64) and needs to be set as a i32
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
                    Number::I8(n) => i16::from(*n),
                    Number::I16(n) => *n,
                    Number::I32(n) => i16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::I64(n) => i16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    t => panic!("{t:?}"),
                })))
            }
            (Primitive::Number(number), PrimitiveID::I32) => {
                Ok(Primitive::Number(Number::I32(match number {
                    Number::I8(n) => i32::from(*n),
                    Number::I16(n) => i32::from(*n),
                    Number::I32(n) => *n,
                    Number::I64(n) => i32::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    t => panic!("{t:?}"),
                })))
            }
            (Primitive::Number(number), PrimitiveID::I64) => {
                Ok(Primitive::Number(Number::I64(match number {
                    Number::I8(n) => i64::from(*n),
                    Number::I16(n) => i64::from(*n),
                    Number::I32(n) => i64::from(*n),
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
                    Number::U8(n) => u16::from(*n),
                    Number::U16(n) => *n,
                    Number::U32(n) => u16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    Number::U64(n) => u16::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U32) => {
                Ok(Primitive::Number(Number::U32(match number {
                    Number::U8(n) => u32::from(*n),
                    Number::U16(n) => u32::from(*n),
                    Number::U32(n) => *n,
                    Number::U64(n) => u32::try_from(*n).map_err(|_| ParseError::IncorrectType)?,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::U64) => {
                Ok(Primitive::Number(Number::U64(match number {
                    Number::U8(n) => u64::from(*n),
                    Number::U16(n) => u64::from(*n),
                    Number::U32(n) => u64::from(*n),
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
                    Number::F32(n) => f64::from(*n),
                    Number::F64(n) => *n,
                    _ => return Err(ParseError::IncorrectType),
                })))
            }
            (Primitive::Number(number), PrimitiveID::String) => Err(ParseError::IncorrectType),
            (
                Primitive::String(_),
                PrimitiveID::I8
                | PrimitiveID::I16
                | PrimitiveID::I32
                | PrimitiveID::I64
                | PrimitiveID::U8
                | PrimitiveID::U16
                | PrimitiveID::U32
                | PrimitiveID::U64
                | PrimitiveID::F32
                | PrimitiveID::F64,
            )
            | (Primitive::Bool(_), _) => Err(ParseError::IncorrectType),
            (Primitive::String(s), PrimitiveID::String) => Ok(Primitive::String(s)),
            (Primitive::Number(_) | Primitive::String(_), PrimitiveID::Bool) => {
                Err(ParseError::IncorrectType)
            }
            (Primitive::Bool(b), PrimitiveID::Bool) => Ok(Primitive::Bool(*b)),
            (Primitive::Bool(_), _) => Err(ParseError::IncorrectType),
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

impl Display for PrimitiveID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            PrimitiveID::I8 => "i8",
            PrimitiveID::I16 => "i16",
            PrimitiveID::I32 => "i32",
            PrimitiveID::I64 => "i64",
            PrimitiveID::U8 => "u8",
            PrimitiveID::U16 => "u16",
            PrimitiveID::U32 => "u32",
            PrimitiveID::U64 => "u64",
            PrimitiveID::F32 => "f32",
            PrimitiveID::F64 => "f64",
            PrimitiveID::String => "String",
            PrimitiveID::Bool => "bool",
        };

        write!(f, "{v}")
    }
}

impl PrimitiveID {
    pub fn can_fit(self, other: Self) -> bool {
        debug!(?self, ?other);
        matches!(
            (self, other),
            (
                PrimitiveID::I8
                    | PrimitiveID::I16
                    | PrimitiveID::I32
                    | PrimitiveID::I64
                    | PrimitiveID::U8
                    | PrimitiveID::U16
                    | PrimitiveID::U32
                    | PrimitiveID::U64,
                PrimitiveID::I8
            ) | (
                PrimitiveID::I16
                    | PrimitiveID::I32
                    | PrimitiveID::I64
                    | PrimitiveID::U16
                    | PrimitiveID::U32
                    | PrimitiveID::U64,
                PrimitiveID::I16 | PrimitiveID::U16
            ) | (
                PrimitiveID::I16
                    | PrimitiveID::I32
                    | PrimitiveID::I64
                    | PrimitiveID::U8
                    | PrimitiveID::U16
                    | PrimitiveID::U32
                    | PrimitiveID::U64,
                PrimitiveID::U8
            ) | (
                PrimitiveID::I32 | PrimitiveID::I64 | PrimitiveID::U32 | PrimitiveID::U64,
                PrimitiveID::I32 | PrimitiveID::U32
            ) | (PrimitiveID::I64 | PrimitiveID::U64, PrimitiveID::I64)
                | (PrimitiveID::U64, PrimitiveID::U64)
                | (PrimitiveID::F32 | PrimitiveID::F64, PrimitiveID::F32)
                | (PrimitiveID::F64, PrimitiveID::F64)
                | (PrimitiveID::String, PrimitiveID::String)
        )
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone)]
pub struct Array<'a> {
    pub(crate) type_id: TypeId,
    pub(crate) values: Vec<Value<'a>>,
}

impl<'a> Array<'a> {
    pub fn new(type_id: TypeId) -> Self {
        let id = TypeId {
            id: type_id.id,
            array: true,
        };
        Self {
            type_id: id,
            values: vec![],
        }
    }

    pub fn push(&mut self, value: Value<'a>) {
        match &value {
            Value::Primitive(primitive) => {
                assert!(self.type_id == primitive.id().into());
            }
            Value::Complex(_complex_value) => {
                panic!()
            }
            Value::Array(_array) => {
                panic!()
            }
        }
        self.values.push(value);
    }
}

impl Display for Array<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = String::from('[');

        for v in &self.values {
            write!(f, "{v} ")?;
        }

        fmt.push(']');

        write!(f, "{fmt}");

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Enum {
    pub(crate) id: ComplexTypeID,
    pub(crate) field: Number,
}

impl Display for Enum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.field)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Visibility {
    #[default]
    Private,
    Pub,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum ComplexTypeName<'a> {
    Known(&'a str),
    Unknown(&'a str),
}

impl Display for ComplexTypeName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for ComplexTypeName<'_> {
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

    pub(crate) fields: HashMap<&'a str, TypeId>,
}

#[derive(Debug)]
pub(crate) struct Frame<'a> {
    pub(crate) pending_name: Option<&'a str>,
    pub(crate) name: &'a str,
    pub(crate) fields: Vec<(usize, &'a str, Value<'a>)>,
}

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Clone)]
pub struct Struct<'a> {
    pub(crate) name: ComplexTypeName<'a>,
    pub(crate) fields: Vec<(usize, &'a str, Value<'a>)>,
}

impl Display for Struct<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = String::from(self.name.name());
        fmt.push_str(" { ");

        self.fields
            .iter()
            .enumerate()
            .for_each(|(i, (lvl, name, val))| match val {
                Value::Primitive(primitive) => {
                    write!(f, "{name}: {primitive} ");
                    if i + 1 < self.fields.len() {
                        fmt.push(',');
                    }
                }
                Value::Complex(complex_value) => {
                    write!(f, "{name}: {complex_value} ");
                    if i + 1 < self.fields.len() {
                        fmt.push(',');
                    }
                }
                Value::Array(array) => {
                    write!(f, "{array}");
                }
            });

        fmt.push_str("} ");

        write!(f, "{fmt}")
    }
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
    pub(crate) typeid: TypeId,
    pub(crate) mutable: bool,
    pub(crate) val: VariableValue<'a>,
}

impl Display for Variable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl<'a> Variable<'a> {
    pub fn new(typeid: TypeId, mutable: bool, val: VariableValue<'a>) -> Self {
        Self {
            typeid,
            mutable,
            val,
        }
    }

    pub fn from_value(value: Value<'a>, mutable: bool, typer: Option<&Typer<'a>>) -> Self {
        let typeid = match &value {
            Value::Primitive(primitive) => TypeId::from(primitive.id()),
            Value::Complex(complex_value) => TypeId::from(match complex_value {
                ComplexValue::Struct(decl) => typer.unwrap().id(decl.name.name()).unwrap(),
                ComplexValue::Enum(enu) => enu.id,
            }),
            Value::Array(array) => array.type_id,
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
    typ: TypeId,
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

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Operation::Add => "+",
            Operation::Sub => "-",
            Operation::Mult => "*",
            Operation::Div => "/",
            Operation::Mod => "%",
            Operation::Assign => "=",
            Operation::AddAssign => "+=",
            Operation::SubAssign => "-=",
            Operation::MultAssign => "*=",
            Operation::DivAssign => "/=",
        };

        write!(f, "{s}")
    }
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
}

#[derive(Debug, PartialEq, Clone)]
pub struct ElseIfStatement<'a> {
    pub(crate) cond: Vec<ConditionItem<'a>>,
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
    Else,
    ElseIf(ElseIfStatement<'a>),
    Block,
    FunctionCall(FunctionCall<'a>),
}

#[derive(Debug, Default, PartialEq)]
pub struct FunctionDecl<'a> {
    pub(crate) visibility: Visibility,
    pub(crate) name: &'a str,
    pub(crate) args: Option<Vec<(bool, &'a str, TypeId)>>,
    pub(crate) return_type: Option<TypeId>,

    pub(crate) block: Vec<(usize, BlockValue<'a>)>,
}

impl<'a> FunctionDecl<'a> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn args(&self) -> Option<&[(bool, &'a str, TypeId)]> {
        self.args.as_deref()
    }

    pub fn block(&self) -> &[(usize, BlockValue<'a>)] {
        &self.block
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionCall<'a> {
    pub(crate) name: &'a str,
    pub(crate) args: Vec<Value<'a>>,
    pub(crate) return_type: Option<TypeId>,
}

impl Display for FunctionCall<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = String::default();

        for v in &self.args {
            write!(f, "{v}");
        }

        match self.return_type {
            Some(typ) => {
                write!(f, "{}({}) -> {}", self.name, fmt, typ)
            }
            None => {
                write!(f, "{}({})", self.name, fmt)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplexTypeDecl<'a> {
    StructDecl(StructDecl<'a>),
    Enum(EnumDecl<'a>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub enum ComplexValue<'a> {
    Struct(Struct<'a>),
    Enum(Enum),
}

impl Display for ComplexValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplexValue::Struct(struc) => write!(f, "{struc}"),
            ComplexValue::Enum(enu) => write!(f, "{enu}"),
        }
    }
}

impl<'a> ComplexValue<'a> {
    fn as_decl(&'a self, typer: &'a Typer) -> ComplexTypeDecl<'a> {
        match self {
            ComplexValue::Struct(struc) => {
                let decl = struc
                    .fields
                    .iter()
                    .map(|f| (f.1, f.2.id(None).unwrap()))
                    .collect::<HashMap<&'a str, TypeId>>();
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
                trace!(?typer);
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub enum Value<'a> {
    Primitive(Primitive<'a>),
    Complex(ComplexValue<'a>),
    Array(Array<'a>),
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Primitive(primitive) => write!(f, "{primitive}"),
            Value::Complex(complex_value) => match complex_value {
                ComplexValue::Struct(struc) => {
                    let mut fmt = String::default();
                    let mut last = 0;
                    write!(f, "{} {{", struc.name);
                    for (i, (lvl, name, field)) in struc.fields.iter().enumerate() {
                        if *lvl > last {
                            write!(f, "{{");
                        }
                        if *lvl < last {
                            write!(f, "}}");
                        }
                        if i < struc.fields.len().saturating_sub(1) {
                            match field {
                                Value::Primitive(primitive) => {
                                    write!(f, "{name}: {primitive},");
                                }
                                Value::Complex(complex_value) => {
                                    write!(f, "{name}: {complex_value},");
                                }
                                Value::Array(array) => {
                                    write!(f, "{array},");
                                }
                            }
                        } else {
                            match field {
                                Value::Primitive(primitive) => {
                                    write!(f, "{name}: {primitive}");
                                }
                                Value::Complex(complex_value) => {
                                    write!(f, "{name}: {complex_value}");
                                }
                                Value::Array(array) => {
                                    write!(f, "{array}");
                                }
                            }
                        }
                        last = *lvl;
                    }
                    write!(f, "}}")
                }
                ComplexValue::Enum(enu) => write!(f, "{}", enu.field),
            },
            Value::Array(array) => write!(f, "{array}"),
        }
    }
}

impl Value<'_> {
    pub fn id(&self, typer: Option<&Typer>) -> Option<TypeId> {
        match self {
            Value::Primitive(prim) => Some(TypeId::from(prim.id())),
            Value::Complex(v) => {
                let typer = typer?;

                match v {
                    ComplexValue::Struct(struc) => typer.id(struc.name.name()).map(TypeId::from),

                    ComplexValue::Enum(enu) => Some(TypeId::from(enu.id)),
                }
            }
            Value::Array(array) => Some(array.type_id),
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
pub struct StructAccess<'a> {
    name: &'a str,
    fields: Vec<&'a str>,
}

impl Display for StructAccess<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.", self.name);

        for name in &self.fields {
            write!(f, ".{name}");
        }

        Ok(())
    }
}

impl<'a> StructAccess<'a> {
    pub fn new(name: &'a str, fields: Vec<&'a str>) -> Self {
        Self { name, fields }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn fields(&self) -> &[&str] {
        &self.fields
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableValue<'a> {
    StructAccess(StructAccess<'a>),
    Value(Value<'a>),
    Name(&'a str),
    Expr(Vec<MathItem<'a>>),
    FunctionCall(FunctionCall<'a>),
    Empty,
}

impl Display for VariableValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableValue::StructAccess(struct_access) => write!(f, "{struct_access}"),
            VariableValue::Value(value) => write!(f, "{value}"),
            VariableValue::Name(name) => write!(f, "{name}"),
            VariableValue::Expr(math_items) => {
                for expr in math_items {
                    write!(f, "{expr},");
                }

                Ok(())
            }
            VariableValue::FunctionCall(function_call) => write!(f, "{function_call}"),
            VariableValue::Empty => write!(f, "Empty"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct MathExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum MathItem<'a> {
    Prim(Primitive<'a>),
    Op(Operation),
}

impl Display for MathItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathItem::Prim(primitive) => write!(f, "{primitive}"),
            MathItem::Op(operation) => write!(f, "{operation}"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct VariableUse;

#[derive(Debug)]
pub enum VariableValueReturn<'a> {
    Assignment(VariableValue<'a>),
    ReAssignment(VariableValue<'a>),
    Expr(Vec<MathItem<'a>>),
}

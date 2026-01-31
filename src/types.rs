#![allow(unused)]

use std::collections::{HashMap, HashSet};
use tracing::{info, instrument};

#[derive(Debug)]
pub(crate) struct Typer<'a> {
    named_types: HashMap<&'a str, ComplexTypeID>,
    /* map of id to {<name>: <type_id>..} should there be a special case for an empty struct/enum?*/
    types: HashMap<ComplexTypeID, Vec<(&'a str, TypeID)>>,
    unknown_types: HashSet<&'a str>,
    next_id: ComplexTypeID,
}

impl<'a> Typer<'a> {
    pub fn register_unknown_type(&mut self, type_name: &'a str) {
        assert!(!self.unknown_types.insert(type_name));
    }

    pub fn register(&mut self, type_name: &'a str, value: &[(&'a str, TypeID)]) {
        info!(?self, ?type_name, ?value);
        assert!(self.named_types.insert(type_name, self.next_id).is_none());
        info!(?self, ?type_name, ?value);
        assert!(
            self.types
                .insert(
                    ComplexTypeID {
                        id: self.types.len()
                    },
                    value.to_vec()
                )
                .is_none()
        );
    }

    pub fn get_id(&self, type_name: &'a str) -> Option<&ComplexTypeID> {
        self.named_types.get(type_name)
    }

    #[instrument]
    pub fn get(&self, type_name: &'a str) -> Option<&Vec<(&'a str, TypeID)>> {
        let id = self.named_types.get(type_name)?;
        self.types.get(id)
    }

    pub fn get_type(&self, id: ComplexTypeID) -> Option<&Vec<(&'a str, TypeID)>> {
        self.types.get(&id)
    }
}

impl<'a> Default for Typer<'a> {
    fn default() -> Self {
        Self {
            named_types: HashMap::default(),
            types: HashMap::default(),
            unknown_types: HashSet::default(),
            next_id: ComplexTypeID { id: 0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ComplexTypeID {
    pub(crate) id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum TypeID {
    Primitive(PrimitiveID),
    Complex(ComplexTypeID),
}

impl From<crate::tokenizer::TypeID> for TypeID {
    fn from(value: crate::tokenizer::TypeID) -> Self {
        match value {
            crate::tokenizer::TypeID::I8 => TypeID::Primitive(PrimitiveID::I8),
            crate::tokenizer::TypeID::I16 => TypeID::Primitive(PrimitiveID::I16),
            crate::tokenizer::TypeID::I32 => TypeID::Primitive(PrimitiveID::I32),
            crate::tokenizer::TypeID::I64 => TypeID::Primitive(PrimitiveID::I64),
            crate::tokenizer::TypeID::U8 => TypeID::Primitive(PrimitiveID::U8),
            crate::tokenizer::TypeID::U16 => TypeID::Primitive(PrimitiveID::U16),
            crate::tokenizer::TypeID::U32 => TypeID::Primitive(PrimitiveID::U32),
            crate::tokenizer::TypeID::U64 => TypeID::Primitive(PrimitiveID::U64),
            crate::tokenizer::TypeID::F32 => TypeID::Primitive(PrimitiveID::F32),
            crate::tokenizer::TypeID::F64 => TypeID::Primitive(PrimitiveID::F64),
            crate::tokenizer::TypeID::String => TypeID::Primitive(PrimitiveID::String),
            _ => panic!(),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
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
    pub fn id(&self) -> TypeID {
        TypeID::Primitive(match self {
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
        })
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq)]
pub(crate) enum Primitive<'a> {
    Number(Number),
    String(&'a str),
}

impl<'a> Primitive<'a> {
    pub fn id(&self) -> TypeID {
        match self {
            Primitive::Number(number) => number.id(),
            Primitive::String(_) => TypeID::Primitive(PrimitiveID::String),
        }
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
}

impl PrimitiveID {
    pub fn can_fit(&self, other: Self) -> bool {
        match (self, other) {
            (PrimitiveID::I8, PrimitiveID::I8) => true,
            (PrimitiveID::I16, PrimitiveID::I8) => true,
            (PrimitiveID::I16, PrimitiveID::I16) => true,
            (PrimitiveID::I16, PrimitiveID::U8) => true,
            (PrimitiveID::I16, PrimitiveID::U16) => true,
            (PrimitiveID::I32, PrimitiveID::I8) => true,
            (PrimitiveID::I32, PrimitiveID::I16) => true,
            (PrimitiveID::I32, PrimitiveID::I32) => true,
            (PrimitiveID::I32, PrimitiveID::U8) => true,
            (PrimitiveID::I32, PrimitiveID::U16) => true,
            (PrimitiveID::I32, PrimitiveID::U32) => true,
            (PrimitiveID::I64, PrimitiveID::I8) => true,
            (PrimitiveID::I64, PrimitiveID::I16) => true,
            (PrimitiveID::I64, PrimitiveID::I32) => true,
            (PrimitiveID::I64, PrimitiveID::I64) => true,
            (PrimitiveID::I64, PrimitiveID::U8) => true,
            (PrimitiveID::I64, PrimitiveID::U16) => true,
            (PrimitiveID::I64, PrimitiveID::U32) => true,
            (PrimitiveID::U8, PrimitiveID::I8) => true,
            (PrimitiveID::U8, PrimitiveID::U8) => true,
            (PrimitiveID::U16, PrimitiveID::I8) => true,
            (PrimitiveID::U16, PrimitiveID::I16) => true,
            (PrimitiveID::U16, PrimitiveID::U8) => true,
            (PrimitiveID::U16, PrimitiveID::U16) => true,
            (PrimitiveID::U32, PrimitiveID::I8) => true,
            (PrimitiveID::U32, PrimitiveID::I16) => true,
            (PrimitiveID::U32, PrimitiveID::I32) => true,
            (PrimitiveID::U32, PrimitiveID::U8) => true,
            (PrimitiveID::U32, PrimitiveID::U16) => true,
            (PrimitiveID::U32, PrimitiveID::U32) => true,
            (PrimitiveID::U64, PrimitiveID::I8) => true,
            (PrimitiveID::U64, PrimitiveID::I16) => true,
            (PrimitiveID::U64, PrimitiveID::I32) => true,
            (PrimitiveID::U64, PrimitiveID::I64) => true,
            (PrimitiveID::U64, PrimitiveID::U8) => true,
            (PrimitiveID::U64, PrimitiveID::U16) => true,
            (PrimitiveID::U64, PrimitiveID::U32) => true,
            (PrimitiveID::U64, PrimitiveID::U64) => true,
            (PrimitiveID::F32, PrimitiveID::F32) => true,
            (PrimitiveID::F64, PrimitiveID::F32) => true,
            (PrimitiveID::F64, PrimitiveID::F64) => true,

            _ => false,
        }
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
                if self.type_id != primitive.id() {
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Enum {
    pub field: Number,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ComplexValue<'a> {
    Array(Array<'a>),
    Struct(HashMap<&'a str, Value<'a>>),
    Enum(Enum),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Value<'a> {
    Primitive(Primitive<'a>),
    Complex(ComplexValue<'a>),
}

impl<'a> Value<'a> {
    pub fn id(&self) -> TypeID {
        match self {
            Value::Primitive(prim) => prim.id(),
            _ => todo!(),
        }
    }
}

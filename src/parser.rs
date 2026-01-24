#![allow(unused)]

use std::{fmt::Display, io::Write};

use crate::function_name;
use crate::info;

use crate::tokenizer::{Token, Tokenizer, TypeID};

#[derive(Debug)]
pub struct Parser<'a> {
    data: &'a str,
    ast: Vec<AstNode<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { data, ast: vec![] }
    }

    pub fn parse(mut self) -> Self {
        let tokenizer = Tokenizer::new(self.data).tokenize();
        info!("{tokenizer:?}");

        let mut tokens = tokenizer.tokens.iter();
        let mut idx = 0;

        loop {
            let Some(token) = tokens.next() else {
                break;
            };

            match token {
                Token::Let => {
                    let Some((name, var)) = Parser::parse_var(&tokenizer.tokens[idx..]) else {
                        panic!()
                    };
                    self.ast.push(AstNode::Variable((name, var)));
                }
                Token::Struct => {
                    let Some((name, var)) = Parser::parse_struct(&tokenizer.tokens[idx..]) else {
                        panic!()
                    };
                    self.ast.push(AstNode::StructType((name, var)));
                }
                _ => {}
            };

            idx += 1;
        }

        self
    }

    fn parse_var(tokens: &[Token<'a>]) -> Option<(&'a str, Variable<'a>)> {
        #[derive(Debug, Clone, Copy)]
        enum VarState {
            VarName,
            TypeID,
            VarVal,
            StructVal,
        }

        let mut state = VarState::VarName;

        let mut found_let = false;
        let mut found_space = false;
        let mut var_name: Option<&'a str> = None;
        let mut var_val: Option<Variable> = None;

        let mut found_eq = false;
        let mut type_id: Option<TypeID> = None;
        let mut found_colon = false;

        let mut idx = 0;
        for token in tokens {
            match state {
                VarState::StructVal => {}

                VarState::VarName => match token {
                    Token::Let => {
                        found_let = true;
                    }
                    Token::Colon => continue,
                    Token::WhiteSpace => continue,
                    Token::Str(s) => {
                        if !found_let {
                            panic!("expected let before variable name");
                        }
                        var_name = Some(s);
                        state = VarState::TypeID;
                    }
                    _ => {}
                },
                VarState::TypeID => match token {
                    Token::Colon => {
                        found_colon = true;
                    }
                    Token::Str(s) => {
                        type_id = Some(TypeID::from(*s));
                        state = VarState::VarVal;
                    }
                    Token::Equal => state = VarState::VarVal,
                    _ => {}
                },
                VarState::VarVal => match token {
                    Token::Number(n) => match type_id {
                        Some(id) => match id {
                            TypeID::I8 => var_val = Some(Variable::I8(n.parse::<i8>().unwrap())),
                            TypeID::I16 => var_val = Some(Variable::I16(n.parse::<i16>().unwrap())),
                            TypeID::I32 => var_val = Some(Variable::I32(n.parse::<i32>().unwrap())),
                            TypeID::I64 => var_val = Some(Variable::I64(n.parse::<i64>().unwrap())),
                            TypeID::U8 => var_val = Some(Variable::U8(n.parse::<u8>().unwrap())),
                            TypeID::U16 => var_val = Some(Variable::U16(n.parse::<u16>().unwrap())),
                            TypeID::U32 => var_val = Some(Variable::U32(n.parse::<u32>().unwrap())),
                            TypeID::U64 => var_val = Some(Variable::U64(n.parse::<u64>().unwrap())),
                            _ => panic!("incorrect id"),
                        },
                        None => var_val = Some(Variable::I64(n.parse::<i64>().unwrap())),
                    },
                    Token::Str(n) => var_val = Some(Variable::String(n)),
                    Token::WhiteSpace => continue,
                    Token::Semicolon => return Some((var_name.unwrap(), var_val.unwrap())),
                    Token::LeftAngleBracket => {
                        let struct_var = Parser::parse_struct_assignment(&tokens[idx..]).unwrap();
                        return Some((var_name.unwrap(), Variable::Struct(struct_var.1)));
                    }
                    Token::Comma => panic!(),
                    Token::RightAngleBracket => panic!(),
                    Token::LeftBracket => panic!(),
                    Token::RightBracket => panic!(),
                    tok => {
                        panic!("{tok:?}")
                    }
                },
            }
            idx += 1;
        }

        info!("{var_name:?} {var_val:?}");

        match (var_name, var_val) {
            (Some(name), Some(val)) => Some((name, val)),
            _ => None,
        }
    }

    fn parse_struct_assignment(tokens: &[Token<'a>]) -> Option<(&'a str, StructVar<'a>)> {
        info!("{tokens:?}");
        #[derive(Debug)]
        enum StructState {
            Ident,
            Equal,
            Val,
        }

        let mut name = None;
        let mut ident = None;
        let mut state = StructState::Ident;
        let mut struct_var = StructVar::default();
        for token in tokens {
            info!("{token:?}");
            match state {
                StructState::Ident => match token {
                    Token::Str(s) => match (name, ident) {
                        (None, None) => {
                            name = Some(s);
                        }
                        (Some(_), None) => {
                            ident = Some(s);
                            state = StructState::Equal;
                        }
                        _ => {}
                    },
                    Token::LeftAngleBracket => continue,
                    Token::Equal => state = StructState::Val,
                    t => panic!("{t:?}"),
                },
                StructState::Equal => match token {
                    Token::Equal => {
                        state = StructState::Val;
                    }
                    t => panic!("{t:?}"),
                },
                StructState::Val => match token {
                    Token::Str(s) => {
                        struct_var.push(ident.unwrap(), Variable::String(s));
                        ident = None;
                    }
                    Token::Number(s) => {
                        struct_var.push(ident.unwrap(), Variable::I64(s.parse::<i64>().unwrap()));
                        ident = None;
                    }
                    Token::Comma => state = StructState::Ident,
                    Token::Semicolon => return Some((name.unwrap(), struct_var)),
                    Token::RightAngleBracket => {}
                    _ => {}
                },
            }
        }
        None
    }

    fn parse_struct(tokens: &[Token<'a>]) -> Option<(&'a str, StructType)> {
        #[derive(Debug, Clone, Copy)]
        enum StructState {
            Name,
            Ident,
        }

        let mut state = StructState::Name;
        let mut found_struct = false;
        let mut name = None;
        let mut struct_type = StructType::default();
        let mut field_name: Option<&'a str> = None;
        let mut field_val: Option<TypeID> = None;

        for token in tokens {
            match state {
                StructState::Name => match token {
                    Token::WhiteSpace => continue,
                    Token::Str(s) => {
                        if found_struct {
                            name = Some(s);
                        }
                    }
                    Token::LeftAngleBracket => state = StructState::Ident,
                    Token::Struct => found_struct = true,
                    _ => panic!("{token:?}"),
                },
                StructState::Ident => match token {
                    Token::Number(_) => todo!(),
                    Token::WhiteSpace => continue,
                    Token::Str(s) => {
                        if field_name.is_none() {
                            field_name = Some(s);
                        } else {
                            field_val = Some(TypeID::String);
                        }
                    }
                    // Token::Semicolon => todo!(),
                    // Token::LeftAngleBracket => todo!(),
                    Token::RightAngleBracket => {
                        if let (Some(name), Some(val)) = (field_name, field_val) {
                            struct_type.fields.push((name.to_string(), val));
                        };
                        return Some((name.unwrap(), struct_type));
                    }
                    // Token::Struct => todo!(),
                    // Token::Colon => todo!(),
                    Token::Comma => {
                        log::trace!("{field_name:?} {field_val:?}");
                        struct_type
                            .fields
                            .push((field_name.unwrap().to_string(), field_val.unwrap()));
                        field_name = None;
                        field_val = None;
                    }
                    Token::TypeID(type_id) => field_val = Some(*type_id),

                    _ => {}
                },
            }
            log::trace!("{token:?}");
        }

        Some((name.unwrap(), struct_type))
    }
}

#[derive(Debug, Clone)]
pub enum Variable<'a> {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    String(&'a str),
    Struct(StructVar<'a>),
    Array(Array<'a>),
}

#[derive(Debug, Clone)]
pub struct Array<'a> {
    typ: TypeID,
    fields: Vec<Variable<'a>>,
}

impl<'a> Array<'a> {
    pub fn new(typ: TypeID) -> Self {
        Self {
            typ,
            fields: vec![],
        }
    }

    pub fn push(&mut self, var: Variable<'a>) {
        match self.typ == var.id() {
            true => self.fields.push(var),
            false => panic!(
                "attempted to push variable with incorrect type expected: {:?} found {:?}",
                self.typ,
                var.id()
            ),
        }
    }
}

impl Variable<'_> {
    pub fn id(&self) -> TypeID {
        match self {
            Variable::I8(_) => TypeID::I8,
            Variable::I16(_) => TypeID::I16,
            Variable::I32(_) => TypeID::I32,
            Variable::I64(_) => TypeID::I64,
            Variable::U8(_) => TypeID::U8,
            Variable::U16(_) => TypeID::U16,
            Variable::U32(_) => TypeID::U32,
            Variable::U64(_) => TypeID::U64,
            Variable::String(_) => TypeID::String,
            Variable::Struct(_) => TypeID::Unknown,
            Variable::Array(_) => TypeID::Array,
        }
    }
}

#[derive(Debug, Default)]
pub struct StructType {
    fields: Vec<(String, TypeID)>,
}

/// could be represented as a slice of type ids?

#[derive(Debug, Default, Clone)]
pub struct StructVar<'a> {
    fields: Vec<(&'a str, Variable<'a>)>,
}

impl<'a> StructVar<'a> {
    pub fn push(&mut self, name: &'a str, variable: Variable<'a>) {
        self.fields.push((name, variable));
    }
}

#[derive(Debug)]
pub enum AstNode<'a> {
    Variable((&'a str, Variable<'a>)),
    StructType((&'a str, StructType)),
}

#[cfg(test)]
mod parser_test {
    use std::time::SystemTime;

    use log::info;

    #[tracing::instrument]
    fn setup_logger() -> Result<(), fern::InitError> {
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{} {} {}] {}",
                    humantime::format_rfc3339_seconds(SystemTime::now()),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Debug)
            .chain(std::io::stdout())
            .chain(fern::log_file("output.log")?)
            .apply()?;
        Ok(())
    }

    #[test]
    fn array() {
        setup_logger().unwrap();
        info!("foo");
    }
}

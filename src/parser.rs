#![allow(unused)]

use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    io::Write,
    str::FromStr,
};

use tracing::{Subscriber, info, instrument};

use crate::{
    tokenizer::{Span, Token, Tokenizer},
    types::{
        self, Array, ComplexValue, Enum, Number, Primitive, PrimitiveID, TypeID, Typer, Value,
    },
};

#[derive(Debug, Default)]
enum Visibility {
    #[default]
    Private,
    Pub,
}

#[derive(Debug, Default)]
struct StructDecl<'a> {
    visiblity: Visibility,

    fields: HashMap<&'a str, types::TypeID>,
}

impl<'a> ParseableCtx<'a> for StructDecl<'a> {
    type Output = (&'a str, Self, usize);
    type Context = Typer<'a>;

    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        panic!("use parse_ctx_mut");
    }

    #[instrument(skip_all)]
    fn parse_ctx_mut(ctx: &mut Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        info!("");
        enum State {
            Struct,
            Name,
            Ident,
            Type,
        }

        let mut state = State::Struct;
        let mut decl = StructDecl::default();
        let mut name = None;
        let mut ident = None;
        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Struct => match token {
                    Token::Struct => {
                        state = State::Name;
                    }

                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(name.is_none());
                        name = Some(s);
                    }
                    Token::WhiteSpace => {}
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
                    Token::WhiteSpace => {}
                    Token::Equal => {
                        state = State::Type;
                    }
                    Token::RightAngleBracket => return Ok((name.unwrap(), decl, i + 1)),
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
                            _ => {
                                info!(?s, ?ctx);
                                match ctx.get_id(s) {
                                    Some(id) => {
                                        assert!(
                                            decl.fields
                                                .insert(ident.unwrap(), TypeID::Complex(*id))
                                                .is_none()
                                        );
                                    }
                                    None => panic!(),
                                }
                            }
                        };
                        state = State::Type;
                    }
                    Token::RightAngleBracket => {
                        info!("returning {:?}", &tokens[i..]);
                        let mut slice =
                            decl.fields.iter().map(|f| (*f.0, *f.1)).collect::<Vec<_>>();
                        slice.sort();
                        ctx.register(name.unwrap(), &slice);
                        return Ok((name.unwrap(), decl, i + 1));
                    }
                    Token::WhiteSpace => {}
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

#[derive(Debug, Default)]
struct Struct<'a> {
    fields: HashMap<&'a str, Value<'a>>,
}

impl<'a> Parseable<'a> for Struct<'a> {
    type Output = (&'a str, Self, usize);

    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        todo!()
    }
}

impl<'a> ParseableCtx<'a> for Struct<'a> {
    type Output = (&'a str, Self, usize);
    type Context = Typer<'a>;

    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        #[derive(Debug)]
        enum State {
            Struct,
            Name,
            Ident,
            Type,
        }

        let mut state = State::Struct;
        let mut decl = Struct::default();
        let mut name = None;
        let mut ident = None;
        for (i, token) in tokens.iter().enumerate() {
            info!(?state, ?token, ?ident);
            match state {
                State::Struct => match token {
                    Token::Struct => {
                        state = State::Name;
                    }

                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(name.is_none());
                        name = Some(s);
                    }
                    Token::WhiteSpace => {}
                    Token::LeftAngleBracket => state = State::Ident,

                    t => panic!("{t:?}"),
                },
                State::Ident => match token {
                    Token::Str(s) => {
                        assert!(ident.is_none());
                        ident = Some(s);
                    }
                    Token::Equal => state = State::Type,
                    Token::WhiteSpace => {}
                    Token::LeftAngleBracket => {}
                    t => panic!("{t:?}"),
                },
                State::Type => match token {
                    Token::Str(s) => {
                        let vec = ctx.get(s).unwrap();
                        info!(?s);
                        for (name, val) in vec {
                            match val {
                                TypeID::Primitive(primitive_id) => {
                                    info!("found primitive {primitive_id:?}")
                                }
                                TypeID::Complex(complex_type_id) => {
                                    info!(
                                        "struct name: {s} field name: {name} fields {:?}",
                                        ctx.get_type(*complex_type_id)
                                    );
                                }
                            }
                        }
                        panic!("{s:?} {vec:?}");
                    }
                    Token::QuotedString(s) => {
                        assert!(
                            decl.fields
                                .insert(ident.unwrap(), Value::Primitive(Primitive::String(s)))
                                .is_none()
                        );
                        state = State::Ident;
                    }
                    Token::Number(n) => {
                        let (prim, i) = Primitive::parse(&tokens[i..])?;
                        assert!(
                            decl.fields
                                .insert(ident.unwrap(), Value::Primitive(prim))
                                .is_none()
                        );

                        state = State::Ident;
                    }
                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
            }
        }
        Err(ParseError::IncorrectType)
    }
}

#[derive(Debug, Default)]
struct EnumDecl<'a> {
    visibility: Visibility,

    fields: HashMap<&'a str, Number>,
}

impl<'a> Parseable<'a> for EnumDecl<'a> {
    type Output = (&'a str, Self, usize);

    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        #[derive(Debug)]
        enum State {
            Enum,
            Name,
            Ident,
            Value,
        }
        let mut name: Option<&str> = None;
        let mut state = State::Enum;
        let mut found_enum = false;
        let mut ident = None;
        let mut decl = EnumDecl::default();

        for (i, token) in tokens.iter().enumerate() {
            info!(?state, ?token);
            match state {
                State::Enum => match token {
                    Token::Pub => decl.visibility = Visibility::Pub,
                    Token::Enum => found_enum = true,
                    Token::WhiteSpace => {
                        if found_enum {
                            state = State::Name;
                        }
                    }

                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::Str(s) => match name {
                        None => name = Some(s),
                        Some(_) => panic!(),
                    },
                    Token::WhiteSpace => {}
                    Token::LeftAngleBracket => {
                        assert!(name.is_some());
                        state = State::Ident;
                    }

                    t => panic!("{t:?}"),
                },
                State::Ident => {
                    info!(?ident);
                    match token {
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
                        Token::WhiteSpace => {}
                        Token::RightAngleBracket => {
                            decl.fields
                                .insert(ident.unwrap(), Number::I64(decl.fields.len() as i64));
                            return Ok((name.unwrap(), decl, i + 1));
                        }

                        t => panic!("{t:?}"),
                    }
                }
                State::Value => match token {
                    Token::WhiteSpace => {}
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

impl<'a> ParseableCtx<'a> for Enum {
    type Output = (Self, usize);
    type Context = Typer<'a>;

    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        enum State {
            Name,
            Ident,
            Value,
        }

        let mut state = State::Name;
        let mut left = None;

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Name => match token {
                    Token::Str(s) => {
                        assert!(left.is_none());
                        left = Some(s);
                    }
                    Token::WhiteSpace => {}
                    Token::Comma => left = None,
                    Token::Colon => state = State::Ident,
                    t => panic!("{t:?}"),
                },
                State::Ident => match token {
                    Token::Colon => state = State::Value,
                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
                State::Value => match token {
                    Token::Str(s) => {
                        let vals = ctx.get(left.unwrap()).unwrap();

                        for (idx, (name, _)) in vals.iter().enumerate() {
                            if s == name {
                                return Ok((
                                    Enum {
                                        field: Number::I64(idx as i64),
                                    },
                                    i,
                                ));
                            }
                        }
                    }
                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
            }
        }
        panic!("{tokens:?}");
    }
}

#[derive(Debug)]
struct Variable<'a> {
    typeid: TypeID,
    mutable: bool,
    val: Value<'a>,
}

impl<'a> Variable<'a> {
    fn new(typeid: TypeID, mutable: bool, val: Value<'a>) -> Self {
        Self {
            typeid,
            mutable,
            val,
        }
    }
}

#[derive(Debug)]
struct VariableType {
    typ: types::TypeID,
}

#[derive(Debug, Default)]
struct FunctionDecl<'a> {
    visibility: Visibility,
    name: &'a str,

    args: Vec<(bool, (&'a str, types::TypeID))>,

    return_type: Option<types::TypeID>,
}

impl<'a> Parseable<'a> for FunctionDecl<'a> {
    type Output = (Self, usize);

    #[instrument]
    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        info!("");
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
        let mut found_fn = false;

        let mut found_arg = None;
        let mut mutable = false;

        for (i, token) in tokens.iter().enumerate() {
            match state {
                State::Fn => match token {
                    Token::WhiteSpace => {
                        if found_fn {
                            state = State::Name;
                        }
                    }
                    Token::Pub => decl.visibility = Visibility::Pub,
                    Token::Function => found_fn = true,
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::WhiteSpace => {}
                    Token::Str(s) => {
                        assert!(decl.name.is_empty());
                        decl.name = s;
                    }
                    Token::LeftParen => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::Arg => match token {
                    Token::WhiteSpace => {}
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
                    t => panic!("{t:?}"),
                },
                State::TypeID => match token {
                    Token::WhiteSpace => {}
                    Token::TypeID(t) => {
                        decl.args
                            .push((mutable, (found_arg.unwrap(), TypeID::from(*t))));
                        found_arg = None;
                        mutable = false;
                    }
                    Token::Str(s) => match *s {
                        "string" => {
                            decl.args.push((
                                mutable,
                                (found_arg.unwrap(), TypeID::Primitive(PrimitiveID::String)),
                            ));
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
                    Token::WhiteSpace => {}
                    Token::TypeID(t) => decl.return_type = Some(TypeID::from(*t)),
                    Token::LeftAngleBracket => state = State::Block,
                    t => panic!("{t:?}"),
                },

                State::Block => match token {
                    Token::WhiteSpace => {}
                    Token::RightAngleBracket => return Ok((decl, i)),
                    Token::Return => state = State::Return,
                    t => panic!("{t:?}"),
                },
                State::Return => {
                    if decl.return_type.is_none() {
                        panic!()
                    }
                    match token {
                        Token::Str(s) => {
                            for (mutable, val) in &decl.args {
                                if val.1 == TypeID::Primitive(PrimitiveID::String) {
                                    panic!()
                                }
                            }
                        }
                        Token::Number(n) => {
                            let (prim, i) = Primitive::parse_ctx(&decl.return_type, &tokens[i..])?;
                            let prim_id = prim.id();
                            match (decl.return_type, prim_id) {
                                (Some(TypeID::Primitive(val_id)), TypeID::Primitive(prim_id)) => {
                                    if !val_id.can_fit(prim_id) {
                                        panic!("{val_id:?} {prim_id:?}");
                                    }
                                }
                                t => panic!("{t:?}"),
                            }
                        }
                        Token::WhiteSpace => {}
                        Token::Semicolon => return Ok((decl, i)),
                        t => panic!("{t:?}"),
                    }
                }
            }
        }

        panic!()
    }
}

#[derive(Debug)]
struct FunctionCall<'a> {
    args: Vec<Value<'a>>,
}

impl<'a> ParseableCtx<'a> for FunctionCall<'a> {
    type Output = (&'a str, Self, usize);
    type Context = Option<()>;

    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        #[derive(Debug)]
        enum State {
            Fn,
            Name,
            Arg,
            FnEnd,
        }
        let mut fn_name: Option<&str> = None;
        let mut args: Vec<Value> = vec![];
        let mut state = State::Fn;
        let mut is_fn = false;
        let mut needs_comma = false;

        for (i, token) in tokens.iter().enumerate() {
            info!(?state, ?token);
            match state {
                State::Fn => match token {
                    Token::WhiteSpace => {
                        if is_fn {
                            state = State::Name
                        }
                    }
                    Token::Function => is_fn = true,
                    t => panic!("{t:?}"),
                },
                State::Name => match token {
                    Token::WhiteSpace => {}
                    Token::Str(s) => {
                        assert!(fn_name.is_none());
                        fn_name = Some(s);
                    }
                    Token::LeftParen => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::Arg => match token {
                    Token::WhiteSpace => {}
                    Token::Number(s) | Token::QuotedString(s) => {
                        info!("needs comma {needs_comma}");
                        if needs_comma {
                            panic!()
                        }
                        let (val, idx) = Primitive::parse(&tokens[i..])?;
                        args.push(Value::Primitive(val));
                        needs_comma = true;
                    }
                    Token::Comma => needs_comma = false,
                    Token::Str(s) => todo!("{s:?}"),
                    Token::RightParen => state = State::FnEnd,
                    t => panic!("{t:?}"),
                },
                State::FnEnd => match token {
                    Token::WhiteSpace => {}
                    Token::Semicolon => return Ok((fn_name.unwrap(), FunctionCall { args }, i)),
                    t => panic!("{t:?}"),
                },
            }
        }

        panic!()
    }
}

#[derive(Debug)]
enum Operation {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
}

#[derive(Debug)]
enum AstNode<'a> {
    ConditionOp(Condition),
    Operation(Operation),
    Assign,
    IfExpr(IfExpr<'a>),
    Block(Expression<'a>),
    Variable(Variable<'a>),
    Type(Value<'a>),
    FunctionDecl(FunctionDecl<'a>),
}

#[derive(Debug)]
enum Expression<'a> {
    VarDecl(Variable<'a>),
    VarAssignment(Variable<'a>, Operation, Value<'a>),
    StructDecl(StructDecl<'a>),
    Struct(Struct<'a>),
    EnumDecl(EnumDecl<'a>),
    Enum(Enum),
}

#[derive(Debug)]
enum Condition {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug)]
struct IfExpr<'a> {
    left: Value<'a>,
    cond: Condition,
    right: Value<'a>,

    exprs: Expression<'a>,
}

pub type ParseResult<T> = Result<T, ParseError>;

trait Parseable<'a>
where
    Self: Sized,
{
    type Output;

    #[instrument]
    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        todo!()
    }
}

trait ParseableCtx<'a>
where
    Self: Sized,
{
    type Output;
    type Context;
    /// for types that may require more information such as Number's
    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output>;

    fn parse_ctx_mut(ctx: &mut Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        todo!()
    }
}

impl<'a> Parseable<'a> for Primitive<'a> {
    type Output = (Self, usize);

    /// if its a number it defaults to I64
    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        for (i, token) in tokens.iter().enumerate() {
            match token {
                Token::WhiteSpace => {}
                Token::Number(n) => match n.contains('.') {
                    true => match n.parse::<f64>() {
                        Ok(f) => return Ok((Primitive::Number(Number::F64(f)), i)),
                        Err(e) => return Err(ParseError::IncorrectType),
                    },

                    false => match n.parse::<i64>() {
                        Ok(n) => return Ok((Primitive::Number(Number::I64(n)), i)),
                        Err(e) => return Err(ParseError::IncorrectType),
                    },
                },
                Token::Str(s) | Token::QuotedString(s) => return Ok((Primitive::String(s), i)),
                t => panic!("{t:?}"),
            }
        }

        Err(ParseError::IncorrectType)
    }
}

impl<'a> ParseableCtx<'a> for Primitive<'a> {
    type Output = (Self, usize);
    type Context = Option<TypeID>;
    /// if ctx is None and its a number it defaults to I64
    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        for (i, token) in tokens.iter().enumerate() {
            match token {
                Token::WhiteSpace => {
                    continue;
                }
                Token::Number(n) => {
                    return match ctx {
                        Some(id) => {
                            return match n.contains('.') {
                                true => {
                                    if *id != TypeID::Primitive(PrimitiveID::F32)
                                        || *id != TypeID::Primitive(PrimitiveID::F64)
                                    {
                                        panic!()
                                    }

                                    return match n.parse::<f64>() {
                                        Ok(f) => Ok((Primitive::Number(Number::F64(f)), i)),
                                        Err(e) => Err(ParseError::IncorrectType),
                                    };
                                }

                                false => match id {
                                    TypeID::Primitive(primitive_id) => match primitive_id {
                                        PrimitiveID::I8 => match n.parse::<i8>() {
                                            Ok(val) => {
                                                return Ok((Primitive::Number(Number::I8(val)), i));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::U8 => match n.parse::<u8>() {
                                            Ok(val) => {
                                                return Ok((Primitive::Number(Number::U8(val)), i));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::I16 => match n.parse::<i16>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::I16(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::U16 => match n.parse::<u16>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::U16(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::I32 => match n.parse::<i32>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::I32(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::U32 => match n.parse::<u32>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::U32(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::I64 => match n.parse::<i64>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::I64(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::U64 => match n.parse::<u64>() {
                                            Ok(val) => {
                                                return Ok((
                                                    Primitive::Number(Number::U64(val)),
                                                    i,
                                                ));
                                            }
                                            Err(e) => Err(ParseError::IncorrectType),
                                        },
                                        PrimitiveID::F32 => panic!(),
                                        PrimitiveID::F64 => panic!(),
                                        PrimitiveID::String => panic!(),
                                    },
                                    TypeID::Complex(complex_type_id) => todo!(),
                                },
                            };
                        }
                        None => Self::parse(tokens),
                    };
                }
                Token::Str(s) => return Ok((Primitive::String(s), i)),
                _ => panic!(),
            }
        }

        Err(ParseError::IncorrectType)
    }
}

impl<'a> Parseable<'a> for Variable<'a> {
    type Output = (Self, usize);

    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        #[derive(Debug)]
        enum State {
            Mut,
            Let,
            VarName,
            TypeID,
            Value,
            Semicolon,
        }

        let mut state = State::Mut;
        let mut name: &'a str;
        let mut found_let = false;
        let mut value: Option<Value> = None;
        let mut var = Variable {
            typeid: TypeID::Primitive(PrimitiveID::I64),
            mutable: false,
            val: Value::Primitive(Primitive::Number(Number::I64(0))),
        };

        let iter = tokens.iter().enumerate().peekable();
        for (i, token) in iter {
            match state {
                State::Mut => match token {
                    Token::Mutable => {
                        var.mutable = true;
                    }

                    Token::Let => {
                        found_let = true;
                        state = State::Let;
                    }
                    Token::WhiteSpace => {}
                    _ => panic!(),
                },
                State::Let => match token {
                    Token::WhiteSpace => {
                        // ensures that it is [Let, Whitespace, ...]
                        if found_let {
                            state = State::VarName;
                        }
                    }
                    Token::Let => {
                        found_let = true;
                    }
                    t => panic!("{t:?}"),
                },
                State::VarName => match token {
                    Token::WhiteSpace => {}
                    Token::Str(s) => {
                        name = s;
                    }
                    Token::Colon => state = State::TypeID,
                    Token::Equal => state = State::Value,
                    t => panic!("{t:?}"),
                },
                State::TypeID => match token {
                    Token::TypeID(id) => {
                        var.typeid = types::TypeID::from(*id);
                    }
                    Token::Equal => state = State::Value,
                    Token::WhiteSpace => {}
                    Token::Str(s) => match *s {
                        "string" => var.typeid = TypeID::Primitive(PrimitiveID::String),
                        _ => {
                            panic!()
                        }
                    },
                    t => panic!("{t:?}"),
                },
                State::Value => match token {
                    Token::WhiteSpace => {}
                    Token::Number(s) => {
                        let (var, idx) = Primitive::parse(&tokens[i..])?;
                        value = Some(Value::Primitive(var));
                        info!("{i} {idx}");
                        state = State::Semicolon;
                    }
                    // somehow support structs and arrays
                    Token::Str(s) => {
                        let (var, idx) = Primitive::parse(&tokens[i..])?;
                        info!("{i} {idx}");
                        value = Some(Value::Primitive(var));
                        state = State::Semicolon;
                    }
                    Token::QuotedString(s) => {
                        info!("{i}");
                        value = Some(Value::Primitive(Primitive::String(s)));
                        state = State::Semicolon;
                    }

                    Token::LeftAngleBracket => {
                        let (name, struc, idx) = Struct::parse(&tokens[i..])?;
                    }
                    t => panic!("{t:?}"),
                },
                State::Semicolon => match token {
                    Token::Semicolon => {
                        if var.typeid != var.val.id() {
                            return Err(ParseError::IncorrectType);
                        }
                        return Ok((var, i));
                    }
                    Token::WhiteSpace => {}
                    t => panic!("{t:?}"),
                },
            }
        }

        panic!()
    }
}

impl<'a> ParseableCtx<'a> for Variable<'a> {
    type Output = (Self, usize);
    type Context = Typer<'a>;

    fn parse_ctx(ctx: &Self::Context, tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        #[derive(Debug)]
        enum State {
            Mut,
            Let,
            VarName,
            TypeID,
            Value,
            Semicolon,
        }

        let mut state = State::Mut;
        let mut name: &'a str;
        let mut found_let = false;
        let mut value: Option<Value> = None;
        let mut var = Variable {
            typeid: TypeID::Primitive(PrimitiveID::I64),
            mutable: false,
            val: Value::Primitive(Primitive::Number(Number::I64(0))),
        };

        let mut i = 0;
        loop {
            if i >= tokens.len() {
                panic!("{i}");
            }

            let token = &tokens[i];
            info!(?state, ?token);

            match state {
                State::Mut => match token {
                    Token::Mutable => {
                        var.mutable = true;
                    }

                    Token::Let => {
                        found_let = true;
                        state = State::Let;
                    }
                    Token::WhiteSpace => {}
                    _ => panic!(),
                },
                State::Let => match token {
                    Token::WhiteSpace => {
                        // ensures that it is [Let, Whitespace, ...]
                        if found_let {
                            state = State::VarName;
                        }
                    }
                    Token::Let => {
                        found_let = true;
                    }
                    t => panic!("{t:?}"),
                },
                State::VarName => match token {
                    Token::WhiteSpace => {}
                    Token::Str(s) => {
                        name = s;
                    }
                    Token::Colon => state = State::TypeID,
                    Token::Equal => state = State::Value,
                    t => panic!("{t:?}"),
                },
                State::TypeID => match token {
                    Token::TypeID(id) => {
                        var.typeid = types::TypeID::from(*id);
                    }
                    Token::Equal => state = State::Value,
                    Token::WhiteSpace => {}
                    Token::Str(s) => match *s {
                        "string" => var.typeid = TypeID::Primitive(PrimitiveID::String),
                        s => match ctx.get_id(s) {
                            Some(id) => {
                                var.typeid = TypeID::Complex(*id);
                            }
                            None => return Err(ParseError::UnknownType(s.to_string())),
                        },
                    },
                    t => panic!("{t:?}"),
                },
                State::Value => match token {
                    Token::WhiteSpace => {}
                    Token::Number(s) => {
                        let (var, idx) = Primitive::parse(&tokens[i..])?;
                        value = Some(Value::Primitive(var));
                        info!("{i} {idx}");
                        state = State::Semicolon;
                    }
                    // somehow support structs and arrays
                    Token::Str(s) => {
                        let (var, idx) = Primitive::parse(&tokens[i..])?;
                        info!("{i} {idx}");
                        value = Some(Value::Primitive(var));
                        state = State::Semicolon;
                    }
                    Token::QuotedString(s) => {
                        info!("{i}");
                        value = Some(Value::Primitive(Primitive::String(s)));
                        state = State::Semicolon;
                    }

                    Token::LeftAngleBracket => {
                        let (name, struc, idx) = Struct::parse(&tokens[i..])?;
                    }
                    t => panic!("{t:?}"),
                },
                State::Semicolon => match token {
                    Token::Semicolon => {
                        if var.typeid != var.val.id() {
                            return Err(ParseError::IncorrectType);
                        }
                        return Ok((var, i));
                    }
                    Token::WhiteSpace => {}
                    Token::Colon => {
                        if let Token::Colon = tokens[i + 1] {
                            let (val, idx) = Enum::parse_ctx(ctx, &tokens[i.saturating_sub(1)..])?;
                            info!(?val, idx);
                            info!("{i} {idx} {:?}", &tokens[i..]);
                            i += idx;
                            info!("{i} {idx} {:?}", &tokens[i..]);

                            value = Some(Value::Complex(ComplexValue::Enum(val)));
                        }
                    }
                    t => panic!("{t:?}"),
                },
            }
            i += 1;
        }

        panic!()
    }
}

impl<'a> Parseable<'a> for Array<'a> {
    type Output = (Self, usize);

    fn parse(tokens: &[Token<'a>]) -> ParseResult<Self::Output> {
        panic!()
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum ParseError {
    IncorrectType,
    UnknownType(String),
    Unterminated(char),
}

#[derive(Debug)]
pub struct Parser<'a> {
    typer: Typer<'a>,

    ast: Vec<AstNode<'a>>,
}

impl<'a> Parser<'a> {
    pub fn parse(tokenizer: Tokenizer<'a>) -> ParseResult<Self> {
        // info!("{tokenizer:?}");

        let mut iter = tokenizer.tokens();
        let mut ast: Vec<AstNode<'a>> = vec![];
        let mut typer: Typer<'a> = Typer::default();

        info!("{:?}", iter);

        let mut idx = 0;

        loop {
            let token = &iter[idx];
            info!(?token);
            if idx >= iter.len() {
                break;
            }
            match token {
                Token::Let => {
                    let (var, i) = Variable::parse_ctx(&typer, &iter[idx..])?;
                    ast.push(AstNode::Variable(var));
                    idx += i;
                }
                Token::WhiteSpace => {
                    idx += 1;
                }
                Token::Equal => {}
                Token::Semicolon => {}
                Token::LeftAngleBracket => {}
                Token::RightAngleBracket => {}
                Token::Colon => {}
                Token::Comma => {}
                Token::LeftBracket => {}
                Token::RightBracket => {}
                Token::Dot => {}
                Token::Exclamation => {}
                Token::Plus => {}
                Token::Minus => {}
                Token::Multiply => {}
                Token::Divide => {}
                Token::Function => {
                    let (func, i) = FunctionDecl::parse(&iter[idx..])?;
                    info!(?func);

                    idx += i;
                }
                Token::TypeID(type_id) => {
                    info!(?type_id);
                }
                Token::Return => {}
                Token::Pub => match (&iter[idx + 1], &iter[idx + 2]) {
                    (Token::WhiteSpace, Token::Struct) => {
                        let (name, mut struc, i) =
                            StructDecl::parse_ctx_mut(&mut typer, &iter[idx..])?;
                        info!(?struc);

                        idx += i;
                        info!("reading {:?}", &iter[idx..]);
                    }
                    (Token::WhiteSpace, Token::Enum) => {
                        let (name, mut enu, i) = EnumDecl::parse(&iter[idx..])?;
                        info!(?name, ?enu, ?i);
                        let mut fields = enu
                            .fields
                            .iter_mut()
                            .map(|f| (*f.0, f.1.id()))
                            .collect::<Vec<_>>();
                        fields.sort();

                        typer.register(name, &fields);

                        idx += i;
                    }
                    t => panic!("{t:?} {token:?}"),
                },
                Token::Struct => {
                    let (name, decl, inc) = StructDecl::parse_ctx_mut(&mut typer, &iter[idx..])?;
                    info!(?name, ?decl, ?inc);
                    idx += inc;
                    info!("reading {:?}", &iter[idx..]);
                }
                Token::Enum => {
                    let (name, decl, i) = EnumDecl::parse(&iter[idx..])?;

                    let slice = decl
                        .fields
                        .iter()
                        .map(|f| (*f.0, f.1.id()))
                        .collect::<Vec<_>>();
                    typer.register(name, &slice);
                    idx += i;
                }

                // things like var += 1;
                Token::Str(s) => {
                    info!("str tokens: {:?}", &iter[idx..]);
                    todo!("{s:?}");
                }

                t => panic!("{t:?}"),
            }
        }

        Ok(Self { ast, typer })
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        time::SystemTime,
    };

    use tracing::info;

    use crate::{
        parser::{ComplexValue, Number, ParseError, ParseResult, Parseable, Parser},
        tokenizer::{Span, Token, Tokenizer},
        types::{self, ComplexTypeID, Primitive, PrimitiveID, TypeID, Value},
    };

    fn setup_logger() {
        let _guard =
            tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new());
    }

    #[test]
    fn var_quoted() {
        setup_logger();
        let data = "let foo = \"bar\" ;";

        let mut parser = Parser::parse(Tokenizer::tokenize(data));

        info!("{:?}", parser);
    }

    #[test]
    fn var_typed_string() {
        setup_logger();

        let data = "let foo:string= \"bar\";";

        let mut parser = Parser::parse(Tokenizer::tokenize(data));
        info!("{:?}", parser);
    }

    #[test]
    fn mismatch_type_id() {
        setup_logger();

        let data = "let foo:i32=\"bar\";";

        let mut parser = Parser::parse(Tokenizer::tokenize(data));
        assert!(parser.is_err());
        info!(?parser);
    }

    #[test]
    fn decl_enum() -> Result<(), ParseError> {
        setup_logger();

        let data = "pub enum Foo {
            Bar=4,
            Baz=2,
        }";

        let mut parser = Parser::parse(Tokenizer::tokenize(data))?;
        info!(?parser);

        let val = parser.typer.get("Foo").unwrap();
        info!(?val);

        let (bar, bar_id) = val[0];

        info!(?bar);
        let names = vec![
            ("Bar", TypeID::Primitive(PrimitiveID::I64)),
            ("Baz", TypeID::Primitive(PrimitiveID::I64)),
        ];

        assert_eq!(*val, names);

        Ok(())
    }

    #[test]
    fn assign_after_decl() -> Result<(), ParseError> {
        setup_logger();

        let data = "struct Bar {i:i32} let foo = Bar {i:0}";

        let mut parser = Parser::parse(Tokenizer::tokenize(data))?;
        info!(?parser);

        Ok(())
    }

    #[test]
    fn assign_before_decl() -> Result<(), ParseError> {
        setup_logger();

        let data = "let foo = Bar {
        baz = 0,
        };
        
        struct Bar {
        baz:i32
            }";

        let mut parser = Parser::parse(Tokenizer::tokenize(data))?;
        info!(?parser);

        let vec = parser.typer.get("Bar").unwrap();
        let (bar, bar_id) = vec[0];

        info!(?bar);

        // let mut names = HashMap::<&str, Type>::from([("baz", TypeID::I32)]);
        //
        // assert_eq!(bar, names);

        Ok(())
    }
}

#[cfg(test)]
mod parseable {
    use std::collections::HashMap;

    use tracing::info;

    use crate::{
        parser::{
            FunctionCall, FunctionDecl, ParseError, ParseResult, Parseable, ParseableCtx, Parser,
            Struct,
        },
        tokenizer::{Span, Token, Tokenizer},
        types::{ComplexValue, Number, Primitive, PrimitiveID, TypeID, Typer, Value},
    };

    fn setup_logger() {
        let _guard =
            tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new());
    }

    #[test]
    fn parse_primitive_number() -> Result<(), ParseError> {
        setup_logger();

        let (prim, _) = Primitive::parse_ctx(
            &Some(TypeID::Primitive(PrimitiveID::I8)),
            &[Token::Number("10")],
        )?;

        info!(?prim);
        assert_eq!(Primitive::Number(Number::I8(10)), prim);

        Ok(())
    }

    #[test]
    fn parse_primitive_string() -> Result<(), ParseError> {
        setup_logger();

        let (prim, _) = Primitive::parse(&[Token::Str("foo")])?;

        info!(?prim);
        assert_eq!(Primitive::String("foo"), prim);

        Ok(())
    }

    #[test]
    fn function_decl() -> Result<(), ParseError> {
        setup_logger();
        let data = "fn foo(mut i:i32,bar:string)i32{
            return 0;
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens();

        let (decl, _) = FunctionDecl::parse(tokens)?;

        info!(?decl);

        Ok(())
    }

    #[test]
    fn function_call() -> Result<(), ParseError> {
        setup_logger();

        let data = "fn foo(\"baz\",1);";
        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens();

        let (name, call, i) = FunctionCall::parse_ctx(&None, tokens)?;
        assert_eq!(
            call.args.as_slice(),
            &[
                Value::Primitive(Primitive::String("baz")),
                Value::Primitive(Primitive::Number(Number::I64(1))),
            ]
        );

        // i starts from 0 and len starts from 1
        assert_eq!(i, tokens.len() - 1);

        Ok(())
    }

    #[test]
    fn complex_struct() -> ParseResult<()> {
        setup_logger();

        let data = "
        struct BarBaz {
        i: string
        }

        struct Baz {
        baz: BarBaz, 
        }

        struct Foo {
        bar= Baz {
        baz= BarBaz {
            i= \"foo\",
            }
          }
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let parser = Parser::parse(tokenizer)?;
        info!(?parser);
        // let mut typer = Typer::default();
        // let mut baz: HashMap<&str, TypeID> = HashMap::default();
        // let mut barbaz: HashMap<&str, TypeID> = HashMap::default();
        // barbaz.insert("i", TypeID::Primitive(PrimitiveID::String));
        // info!(?barbaz);
        // let id = typer.register(
        //     "BarBaz",
        //     barbaz
        //         .iter()
        //         .map(|f| (*f.0, *f.1))
        //         .collect::<Vec<_>>()
        //         .as_slice(),
        // );
        // baz.insert("baz", TypeID::Complex(id));
        // info!("baz {:?}", typer.get_type(id));
        // typer.register(
        //     "Baz",
        //     baz.iter()
        //         .map(|f| (*f.0, *f.1))
        //         .collect::<Vec<_>>()
        //         .as_slice(),
        // );

        // Struct::parse_ctx(&typer, tokens)?;
        Ok(())
    }

    #[test]
    fn enum_decl() {
        setup_logger();

        let data = "enum Foo {
        Bar 
        }
        
        let foo = Foo::Bar;
        ";

        let tokenizer = Tokenizer::tokenize(data);
        let parser = Parser::parse(tokenizer);

        info!(?parser);
    }
}

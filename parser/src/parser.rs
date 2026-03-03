#![allow(unused)]

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    default,
    error::Error,
    fmt::{Debug, Display},
    io::Write,
    str::FromStr,
};

use tracing::{Subscriber, debug, error, info, instrument, trace, warn};

use crate::types::{
    self, Array, Block, BlockValue, ComplexType, ComplexTypeDecl, ComplexTypeID, ComplexTypeName,
    ComplexValue, Else, ElseIfStatement, Enum, EnumDecl, Frame, FunctionCall, FunctionDecl,
    MathExpr, MathItem, Number, Operation, Primitive, PrimitiveID, Struct, StructDecl, TypeDecl,
    TypeDeclReturn, TypeID, Typer, Value, Variable, VariableUse, VariableValue,
    VariableValueReturn, Visibility,
};
use tokenizer::{Span, Token, Tokenizer};

#[derive(Debug, PartialEq)]
pub enum Type<'a> {
    Known(ComplexTypeID),
    UnknownType(&'a str),
}

#[derive(Debug, PartialEq)]
pub enum AstNode<'a> {
    Str(&'a str),
    Value(Value<'a>),
    Struct(StructDecl<'a>),
    Type(Type<'a>),

    Enum(EnumDecl<'a>),

    Function(FunctionDecl<'a>),
    FunctionCall(FunctionCall<'a>),
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum ParseError {
    IncorrectType,
    DuplicateField,
    UnknownType(String),
    Unterminated(char),
    ImmutableAssignment,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Default)]
pub struct Ast<'a> {
    nodes: Vec<AstNode<'a>>,
}

impl<'a> Ast<'a> {
    pub fn push(&mut self, node: AstNode<'a>) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[AstNode<'a>] {
        &self.nodes
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    typer: Typer<'a>,

    ast: Ast<'a>,
}

impl<'a> Parser<'a> {
    pub fn ast(&self) -> &Ast<'_> {
        &self.ast
    }

    pub fn parse(s: &'a str) -> ParseResult<Self> {
        let tokenizer = Tokenizer::tokenize(s);

        let mut iter = tokenizer.tokens();

        let mut idx = 0;
        let mut needs_inc = true;

        let mut typer = Typer::default();
        let mut ast = Ast::default();

        let mut unknown_types: HashSet<&str> = HashSet::default();

        loop {
            if idx >= iter.len() {
                break;
            }
            let token = &iter[idx];
            match token.1 {
                Token::Str(_) => {
                    if let Some((span, peek)) = iter.get(idx + 1)
                        && peek == &Token::LeftParen
                    {
                        let (call, i) =
                            FunctionCall::parse_ctx(&(), &iter[idx.saturating_sub(1)..])?;
                        ast.push(AstNode::FunctionCall(call));
                    }
                }
                Token::TypeID(type_id) => {
                    info!(?type_id);
                }
                Token::Pub | Token::Function | Token::Struct | Token::Enum => {
                    let (name, decl, i) = TypeDecl::parse_ctx_mut(&mut typer, &iter[idx..])?;
                    idx += i;
                    needs_inc = false;
                    match decl {
                        crate::types::TypeDeclReturn::Enum(enum_decl) => {
                            typer.register(name.unwrap(), ComplexTypeDecl::Enum(enum_decl));
                        }
                        crate::types::TypeDeclReturn::Struct(struct_decl) => {
                            typer.register(name.unwrap(), ComplexTypeDecl::StructDecl(struct_decl));
                        }
                        crate::types::TypeDeclReturn::Function(function_decl) => {
                            ast.push(AstNode::Function(function_decl));
                        }
                    }
                }

                _ => needs_inc = true,

                t => panic!("{t:?}"),
            }
            if needs_inc {
                idx += 1;
            }
        }

        for typ in unknown_types {
            println!("{typ:?}");
        }

        Ok(Self { ast, typer })
    }
}

impl<'a> Primitive<'a> {
    #[instrument(name = "Primitive::parse", skip_all, err)]
    pub fn parse(tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
        match tokens[0].1 {
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
    pub fn parse_ctx(
        ctx: &Option<TypeID>,
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Self, usize)> {
        match ctx {
            None => Self::parse(tokens),
            Some(id) => {
                match (tokens[0].1, id) {
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

impl Enum {
    #[instrument(name = "Enum::parse_ctx", skip_all, err)]
    pub fn parse_ctx<'a>(
        ctx: &Typer<'a>,
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Self, usize)> {
        enum State {
            Name,
            Ident,
            Value,
        }

        let mut state = State::Name;
        let mut left = None;
        let mut field = None;

        for (i, (span, token)) in tokens.iter().enumerate() {
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

impl<'a> StructDecl<'a> {
    #[instrument(name = "StructDecl::parse_ctx_mut", skip_all, err)]
    pub fn parse_ctx_mut(
        ctx: &mut Typer<'a>,
        tokens: &[(Span, Token<'a>)],
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

        for (i, (span, token)) in tokens.iter().enumerate() {
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
                        if ident.is_some() {
                            error!(?ident);
                            assert!(ident.is_none());
                        }
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

impl<'a> Struct<'a> {
    #[instrument(name = "Struct::parse_ctx", skip_all, err)]
    pub fn parse_ctx(ctx: &Typer, tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
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

        while let Some((span, token)) = tokens.get(i) {
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

                        /// finds nested struct through assuming its already read the field name so
                        /// then anything that is :<str> { is a struct
                        if let Some((_, Token::Colon)) = tokens.get(i + 1)
                            && let Some((_, Token::Str(typ))) = tokens.get(i + 2)
                            && let Some((_, Token::LeftAngleBracket)) = tokens.get(i + 3)
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

                        assert!(ctx.get(finished.name).is_some());

                        let parent = stack.last_mut().unwrap();
                        let parent_name = parent.pending_name.take().unwrap();
                        parent.fields.push((
                            parent_name,
                            Value::Complex(crate::types::ComplexValue::Struct(comp)),
                        ));
                        i += 1;
                    }
                    Token::QuotedString(_s) | Token::Number(_s) => {
                        let (val, inc) = Primitive::parse(&[(*span, *token)])?;
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

impl<'a> EnumDecl<'a> {
    #[instrument(name = "EnumDecl::parse", skip_all, err)]
    pub fn parse(tokens: &[(Span, Token<'a>)]) -> ParseResult<(&'a str, Self, usize)> {
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

        for (i, (span, token)) in tokens.iter().enumerate() {
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

impl<'a> Block<'a> {
    fn parse_ctx(ctx: &Typer<'a>, tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
        enum State {
            Block,
            Return,
        }

        let mut state = State::Block;
        let mut block = Block::default();
        let mut i = 0;

        loop {
            let Some((_, token)) = tokens.get(i) else {
                break;
            };
            match state {
                State::Block => match token {
                    Token::Return => state = State::Return,
                    Token::Else => match block.values.last_mut().unwrap() {
                        BlockValue::IfStatement(if_statement) => {
                            if_statement.block.push(BlockValue::Else(Block::default()))
                        }
                        BlockValue::Else(block) => block.push(BlockValue::Else(Block::default())),
                        BlockValue::ElseIf(else_if_statement) => else_if_statement
                            .block
                            .push(BlockValue::Else(Block::default())),
                        BlockValue::Block(block) => match block.last_mut().unwrap() {
                            BlockValue::Else(block) => {
                                block.push(BlockValue::Else(Block::default()))
                            }
                            BlockValue::Block(block) => {
                                block.push(BlockValue::Else(Block::default()))
                            }
                            t => block.push(BlockValue::Else(Block::default())),
                        },
                        t => block.push(BlockValue::Else(Block::default())),
                    },
                    Token::LeftAngleBracket => {}
                    Token::RightAngleBracket => {
                        let completed = block.values.pop().unwrap();
                        if block.values.is_empty() {
                            info!(?completed, "finished");
                            return Ok((
                                Block {
                                    values: vec![completed],
                                },
                                i,
                            ));
                        }

                        info!(?completed);
                        block.push(completed);
                    }
                    Token::Let => {
                        let (var_name, mutable, id, value, inc) =
                            Variable::parse_ctx(ctx, &tokens[i..])?;

                        i += inc;

                        let id = match id {
                            Some(id) => Some(id),
                            None => match &value {
                                VariableValue::Value(value) => value.id(Some(ctx)),
                                VariableValue::Name(var_name) => {
                                    let mut id_ = None;
                                    for b in block.values.iter_mut().rev() {
                                        match b {
                                            BlockValue::VariableDecl(decl) => {
                                                if *var_name == decl.0 {
                                                    match &decl.1.val {
                                                        VariableValue::Value(value) => {
                                                            id_ = value.id(Some(ctx));
                                                        }
                                                        VariableValue::Name(_) => todo!(),
                                                        VariableValue::Expr(math_items) => {
                                                            todo!()
                                                        }
                                                    }
                                                }
                                            }
                                            BlockValue::Else(else_statement) => {
                                                for values in else_statement.iter().rev() {
                                                    match values {
                                                        BlockValue::VariableDecl(decl) => {
                                                            if *var_name == decl.0 {
                                                                match &decl.1.val {
                                                                    VariableValue::Value(value) => {
                                                                        id_ = value.id(Some(ctx));
                                                                    }
                                                                    VariableValue::Name(_) => {
                                                                        todo!()
                                                                    }
                                                                    VariableValue::Expr(
                                                                        math_items,
                                                                    ) => {
                                                                        todo!()
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        BlockValue::Else(else_statement) => {
                                                            for values in
                                                                else_statement.iter().rev()
                                                            {
                                                                match values {
                                                                    BlockValue::VariableDecl(
                                                                        decl,
                                                                    ) => {
                                                                        if *var_name == decl.0 {
                                                                            match &decl.1.val {
                                                                    VariableValue::Value(value) => {
                                                                        id_ = value.id(Some(ctx));
                                                                    }
                                                                    VariableValue::Name(_) => {
                                                                        todo!()
                                                                    }
                                                                    VariableValue::Expr(
                                                                        math_items,
                                                                    ) => {
                                                                        todo!()
                                                                    }
                                                                }
                                                                        }
                                                                    }
                                                                    BlockValue::Else(
                                                                        else_block,
                                                                    ) => continue,
                                                                    t => panic!("{t:?}"),
                                                                }
                                                            }
                                                        }
                                                        t => panic!("{t:?}"),
                                                    }
                                                }
                                            }
                                            BlockValue::Block(block_values) => {
                                                for values in block_values.iter().rev() {
                                                    match values {
                                                        BlockValue::VariableDecl(decl) => {
                                                            if *var_name == decl.0 {
                                                                match &decl.1.val {
                                                                    VariableValue::Value(value) => {
                                                                        id_ = value.id(Some(ctx));
                                                                    }
                                                                    VariableValue::Name(_) => {
                                                                        todo!()
                                                                    }
                                                                    VariableValue::Expr(
                                                                        math_items,
                                                                    ) => {
                                                                        todo!()
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        BlockValue::Else(else_statement) => {
                                                            for values in
                                                                else_statement.iter().rev()
                                                            {
                                                                match values {
                                                                    BlockValue::VariableDecl(
                                                                        decl,
                                                                    ) => {
                                                                        if *var_name == decl.0 {
                                                                            match &decl.1.val {
                                                                    VariableValue::Value(value) => {
                                                                        id_ = value.id(Some(ctx));
                                                                    }
                                                                    VariableValue::Name(_) => {
                                                                        todo!()
                                                                    }
                                                                    VariableValue::Expr(
                                                                        math_items,
                                                                    ) => {
                                                                        todo!()
                                                                    }
                                                                }
                                                                        }
                                                                    }
                                                                    BlockValue::Else(
                                                                        else_block,
                                                                    ) => continue,
                                                                    t => panic!("{t:?}"),
                                                                }
                                                            }
                                                        }
                                                        BlockValue::ElseIf(else_if_statement) => {
                                                            todo!()
                                                        }
                                                        t => panic!("{t:?}"),
                                                    }
                                                }
                                            }
                                            t => panic!("{t:?}"),
                                        }
                                    }
                                    id_
                                }
                                t => panic!("{t:?}"),
                            },
                        };

                        match value {
                            VariableValue::Value(value) => {
                                let var_value = BlockValue::VariableDecl((
                                    var_name,
                                    Variable {
                                        typeid: id.unwrap(),
                                        mutable,
                                        val: VariableValue::Value(value),
                                    },
                                ));
                                info!("{var_value:?}");
                                match block.values.last_mut() {
                                    None => block.push(var_value),
                                    Some(block_value) => match block_value {
                                        BlockValue::Else(block) | BlockValue::Block(block) => {
                                            match block.values.last_mut() {
                                                None => block.push(var_value),
                                                Some(block_value) => match block_value {
                                                    BlockValue::VariableDecl(_) => todo!(),
                                                    BlockValue::VariableReAssignment(_) => todo!(),
                                                    BlockValue::IfStatement(if_statement) => {
                                                        if_statement.block.push(var_value)
                                                    }
                                                    BlockValue::Else(else_block) => {
                                                        else_block.push(var_value)
                                                    }
                                                    BlockValue::ElseIf(else_if_statement) => {
                                                        else_if_statement.block.push(var_value)
                                                    }
                                                    BlockValue::Block(inner_block) => {
                                                        inner_block.push(var_value)
                                                    }
                                                    t => panic!("{t:?}"),
                                                },
                                            }
                                        }
                                        t => warn!("{t:?}"),
                                    },
                                }
                            }
                            VariableValue::Name(name) => {
                                let var_value = BlockValue::VariableDecl((
                                    var_name,
                                    Variable {
                                        typeid: id.unwrap(),
                                        mutable,
                                        val: VariableValue::Name(name),
                                    },
                                ));
                                info!("name {var_value:?}");
                                match block.values.last_mut() {
                                    None => {
                                        block.values.push(var_value);
                                    }
                                    Some(block_value) => match block_value {
                                        BlockValue::Else(block) | BlockValue::Block(block) => {
                                            match block.values.last_mut().unwrap() {
                                                BlockValue::IfStatement(if_statement) => {
                                                    if_statement.block.push(var_value)
                                                }
                                                BlockValue::Else(else_block) => {
                                                    match else_block.last_mut() {
                                                        None => else_block.push(var_value),
                                                        Some(else_block_value) => {
                                                            match else_block_value {
                                                                BlockValue::IfStatement(
                                                                    if_statement,
                                                                ) => if_statement
                                                                    .block
                                                                    .push(var_value),
                                                                BlockValue::Else(
                                                                    inner_else_block,
                                                                ) => {
                                                                    inner_else_block.push(var_value)
                                                                }
                                                                BlockValue::ElseIf(
                                                                    else_if_statement,
                                                                ) => else_if_statement
                                                                    .block
                                                                    .push(var_value),
                                                                BlockValue::Block(inner_block) => {
                                                                    inner_block.push(var_value)
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                }
                                                BlockValue::ElseIf(else_if_statement) => {
                                                    else_if_statement.block.push(var_value)
                                                }
                                                BlockValue::Block(inner_block) => {
                                                    inner_block.push(var_value)
                                                }
                                                t => panic!("{t:?}"),
                                            }
                                        }
                                        t => warn!("{t:?}"),
                                    },
                                }
                            }
                            t => panic!("{t:?} {block:?}"),
                        }

                        trace!(?block);
                    }

                    // Token::Str(s) => {
                    //     let (var_name, inc) = VariableUse::parse_ctx(ctx, &tokens[i..])?;
                    //
                    //     let mut reass = None;
                    //     for b in block.iter_mut().rev() {
                    //         match b {
                    //             BlockValue::Else(else_statement) => {
                    //                 panic!("{else_statement:?}")
                    //             }
                    //             BlockValue::Block(block_values) => {
                    //                 for values in block_values.iter() {
                    //                     match values {
                    //                         BlockValue::VariableDecl(decl) => {
                    //                             reass = match var_name {
                    //                                 VariableValueReturn::Assignment(
                    //                                     ref variable_value,
                    //                                 ) => Some(variable_value.clone()),
                    //                                 VariableValueReturn::ReAssignment(
                    //                                     ref variable_value,
                    //                                 ) => Some(variable_value.clone()),
                    //                                 VariableValueReturn::Expr(ref math_items) => {
                    //                                     Some(VariableValue::Expr(
                    //                                         math_items.to_vec(),
                    //                                     ))
                    //                                 }
                    //                             };
                    //                         }
                    //                         t => panic!("{t:?}"),
                    //                     }
                    //                 }
                    //             }
                    //             t => panic!("{t:?}"),
                    //         }
                    //     }
                    //
                    //     match block.last_mut().unwrap() {
                    //         BlockValue::Block(block_values) => block_values
                    //             .push(BlockValue::VariableReAssignment((s, reass.unwrap()))),
                    //         t => panic!("{t:?}"),
                    //     }
                    //
                    //     i += inc;
                    // }
                    t => panic!("{t:?}"),
                },
                State::Return => {
                    let block_values = match block.values.last_mut().unwrap() {
                        BlockValue::Else(else_statement) => {
                            panic!("{else_statement:?}")
                        }
                        BlockValue::Block(block_values) => block_values,
                        t => panic!("{t:?}"),
                    };
                    match token {
                        Token::Str(s) => {
                            // handle returning a struct being made as the return value? ie
                            // return Foo { ... };
                            block_values.push(BlockValue::Return(VariableValue::Name(s)));
                        }
                        Token::Number(_n) | Token::QuotedString(_n) => {
                            let (prim, _i) = Primitive::parse(&tokens[i..])?;
                            let prim_id = prim.id();
                            block_values.push(BlockValue::Return(VariableValue::Value(
                                Value::Primitive(prim),
                            )));
                            // match (decl.return_type, prim_id) {
                            //     (Some(val_id), prim_id) => match val_id {
                            //         TypeID::Primitive(primitive_id) => {
                            //             if !primitive_id.can_fit(prim_id) {
                            //                 panic!("{val_id:?} {prim_id:?}");
                            //             }
                            //         }
                            //         TypeID::Complex(_complex_type_id) => todo!(),
                            //     },
                            //     t => panic!("{t:?}"),
                            // }
                        }

                        Token::Semicolon => {
                            state = State::Block;
                        }
                        t => panic!("{t:?}"),
                    }
                }
            }
            i += 1;
        }

        Ok((block, i))
    }
}

impl<'a> FunctionDecl<'a> {
    #[instrument(name = "FunctionDecl::parse_ctx", skip_all, err)]
    fn parse_ctx(ctx: &Typer<'a>, tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Fn,
            Name,
            Arg,
            TypeID,
            ReturnType,
        }

        let mut state = State::Fn;
        let mut decl = FunctionDecl::default();

        let mut found_arg = None;
        let mut mutable = false;

        let mut i = 0;

        loop {
            let Some((span, token)) = tokens.get(i) else {
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
                    Token::RightParen => {
                        state = State::ReturnType;
                    }
                    t => panic!("{t:?}"),
                },
                State::TypeID => match token {
                    Token::TypeID(t) => {
                        match &mut decl.args {
                            Some(args) => {
                                args.push((mutable, found_arg.unwrap(), TypeID::from(*t)))
                            }
                            None => {
                                decl.args =
                                    Some(vec![(mutable, found_arg.unwrap(), TypeID::from(*t))])
                            }
                        }
                        found_arg = None;
                        mutable = false;
                    }
                    Token::RightParen => state = State::ReturnType,
                    Token::Comma => state = State::Arg,
                    t => panic!("{t:?}"),
                },
                State::ReturnType => match token {
                    Token::TypeID(t) => decl.return_type = Some(TypeID::from(*t)),
                    Token::LeftAngleBracket => {
                        let (block, inc) = Block::parse_ctx(ctx, &tokens[i..])?;
                        decl.block = block;
                        return Ok((decl, i + inc));
                    }
                    t => panic!("{t:?}"),
                },
            }

            i += 1;
        }

        panic!()
    }
}

impl<'a> FunctionCall<'a> {
    #[instrument(name = "FunctionCall::parse_ctx", skip_all, err)]
    pub fn parse_ctx(_ctx: &(), tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
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

        for (i, (span, token)) in tokens.iter().enumerate() {
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

impl TypeDecl {
    #[instrument(name = "TypeDecl::parse_ctx_mut", skip_all, err)]
    pub fn parse_ctx_mut<'a>(
        ctx: &mut Typer<'a>,
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Option<&'a str>, TypeDeclReturn<'a>, usize)> {
        let mut start = 0;
        let vis = if tokens[0].1 == Token::Pub {
            start = 1;
            Visibility::Pub
        } else {
            Visibility::Private
        };
        match tokens[start] {
            (_, Token::Function) => {
                let (mut decl, i) = FunctionDecl::parse_ctx(ctx, tokens)?;
                decl.visibility = vis;

                Ok((None, TypeDeclReturn::Function(decl), i))
            }
            (_, Token::Struct) => {
                let (name, mut decl, i) = StructDecl::parse_ctx_mut(ctx, tokens)?;
                decl.visibility = vis;

                Ok((Some(name), TypeDeclReturn::Struct(decl), i))
            }
            (_, Token::Enum) => {
                let (name, mut decl, i) = EnumDecl::parse(tokens)?;
                decl.visibility = vis;

                Ok((Some(name), TypeDeclReturn::Enum(decl), i))
            }
            t => panic!("{t:?}"),
        }
    }
}

impl<'a> VariableValue<'a> {
    // #[instrument(name = "VariableValue::parse_ctx", skip_all, ret)]
    pub fn parse_ctx(
        ctx: &(Option<TypeID>, Typer<'a>),
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Self, usize)> {
        #[derive(Debug)]
        enum State {
            Value,
        }

        let mut value: Option<VariableValue> = None;

        let type_id = &ctx.0;
        let typer = &ctx.1;
        let mut i = 0;

        while let Some((span, token)) = tokens.get(i) {
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
                        Some((span, token)) => match token {
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
    // #[instrument(name = "Variable::parse_ctx", skip_all, ret)]
    pub fn parse_ctx(
        ctx: &Typer<'a>,
        tokens: &[(Span, Token<'a>)],
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
            let Some((span, token)) = tokens.get(i) else {
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

impl<'a> MathExpr {
    #[instrument(name = "MathExpr::parse", skip_all, err)]
    pub fn parse(tokens: &[(Span, Token<'a>)]) -> ParseResult<(Vec<MathItem<'a>>, usize)> {
        let mut i = 0;
        let mut items = vec![];
        while let Some((span, token)) = tokens.get(i) {
            match token {
                Token::Plus => items.push(MathItem::Op(Operation::Add)),
                Token::Minus => items.push(MathItem::Op(Operation::Sub)),
                Token::Multiply => items.push(MathItem::Op(Operation::Mult)),
                Token::Divide => items.push(MathItem::Op(Operation::Div)),

                Token::Number(_n) => {
                    items.push(MathItem::Prim(Primitive::parse(&[(*span, *token)])?.0))
                }
                Token::Semicolon => return Ok((items, i.saturating_sub(1))),
                t => panic!("{t:?}"),
            }
            i += 1;
        }

        Err(ParseError::IncorrectType)
    }
}

impl VariableUse {
    // #[instrument(name = "VariableUse::parse_ctx", skip_all, ret)]
    pub fn parse_ctx<'a>(
        _ctx: &Typer<'a>,
        tokens: &[(Span, Token<'a>)],
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

        while let Some((span, tok)) = tokens.get(i) {
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
                        let prim = VariableValue::Value(Value::Primitive(
                            Primitive::parse(&[(*span, *tok)])?.0,
                        ));
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
                        (Token::Exclamation, Some((span, Token::Equal))) => {
                            todo!("should this lang support things like let foo = bar != baz")
                        }
                        (Token::Number(_n), Some((span, Token::Semicolon)))
                        | (Token::QuotedString(_n), Some((span, Token::Semicolon))) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            let prim = VariableValue::Value(Value::Primitive(
                                Primitive::parse(&[(*span, *tok)])?.0,
                            ));
                            return Ok((VariableValueReturn::Assignment(prim), i + 1));
                        }
                        (Token::Str(_s), Some((span, Token::Equal))) => {
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
                        (Token::Number(_n), Some((span, Token::Plus)))
                        | (Token::Number(_n), Some((span, Token::Minus)))
                        | (Token::Number(_n), Some((span, Token::Multiply)))
                        | (Token::Number(_n), Some((span, Token::Divide))) => {
                            let (val, inc) = MathExpr::parse(&tokens[i..])?;
                            return Ok((VariableValueReturn::Expr(val), i + inc + 1));
                        }
                        (Token::True, Some((span, Token::Semicolon))) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            return Ok((
                                VariableValueReturn::Assignment(VariableValue::Value(
                                    Value::Primitive(Primitive::Bool(true)),
                                )),
                                i + 1,
                            ));
                        }
                        (Token::False, Some((span, Token::Semicolon))) => {
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

#[cfg(test)]
mod parseable {
    use std::{collections::HashMap, ops::ControlFlow};

    use tracing::{error, info};

    use crate::{
        parser::{AstNode, FunctionDecl, ParseError, ParseResult, Parser},
        types::{
            Block, BlockValue, ComplexType, ComplexTypeDecl, ComplexTypeID, ComplexTypeName,
            ComplexValue, Enum, FunctionCall, Number, Primitive, PrimitiveID, Struct, StructDecl,
            TypeID, Typer, Value, Variable, Visibility,
        },
    };

    use tokenizer::{Span, Token, Tokenizer};

    #[inline]
    fn setup_logger() {
        let _guard =
            tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new());
    }

    #[test]
    fn parse_primitive_number() -> Result<(), ParseError> {
        setup_logger();

        let data = r#"fn foo() {
        let foo = 10;
        }"#;
        let parser = Parser::parse(data)?;

        let AstNode::Function(ref function) = parser.ast.nodes[0] else {
            panic!()
        };

        for val in function.block.iter() {
            assert_eq!(
                val,
                &BlockValue::VariableDecl((
                    "foo",
                    Variable {
                        typeid: PrimitiveID::I64.into(),
                        mutable: false,
                        val: crate::types::VariableValue::Value(Value::Primitive(
                            Primitive::Number(Number::I64(10))
                        )),
                    }
                ))
            );
        }

        // info!(?parser);
        // assert_eq!(
        //     &[
        //         AstNode::Let,
        //         AstNode::Str("foo"),
        //         AstNode::Equal,
        //         AstNode::Value(Primitive::Number(Number::I64(10)).into()),
        //     ],
        //     &parser.ast.nodes[0..4]
        // );

        Ok(())
    }

    #[test]
    fn parse_primitive_string() -> Result<(), ParseError> {
        setup_logger();

        let data = r#"
        fn foo() {
            let foo = "foo_bar_baz"; 
            };
        }"#;
        let parser = Parser::parse(data)?;
        let AstNode::Function(ref function) = parser.ast.nodes[0] else {
            panic!()
        };

        for val in function.block.iter() {
            assert_eq!(
                val,
                &BlockValue::VariableDecl((
                    "foo",
                    Variable {
                        typeid: PrimitiveID::String.into(),
                        mutable: false,
                        val: crate::types::VariableValue::Value(Value::Primitive(
                            Primitive::String("foo_bar_baz")
                        )),
                    }
                ))
            );
        }

        // info!(?parser);
        // assert_eq!(
        //     &[
        //         AstNode::Let,
        //         AstNode::Str("foo"),
        //         AstNode::Equal,
        //         AstNode::Value(Primitive::String("bar").into()),
        //     ],
        //     &parser.ast.nodes[0..4]
        // );

        Ok(())
    }

    #[test]
    fn function_decl() -> Result<(), ParseError> {
        setup_logger();

        let data = "fn foo(mut i:i32,bar:string)i32{
            return 0;
        }";

        let parser = Parser::parse(data)?;

        let decl = FunctionDecl {
            visibility: Visibility::Private,
            name: "foo",
            args: Some(vec![
                (true, "i", PrimitiveID::I32.into()),
                (false, "bar", PrimitiveID::String.into()),
            ]),
            block: Block::default(),
            return_type: Some(PrimitiveID::I32.into()),
        };

        assert_eq!(AstNode::Function(decl), parser.ast.nodes[0]);

        // info!(?parser);

        Ok(())
    }

    #[test]
    fn function_call() -> Result<(), ParseError> {
        setup_logger();

        let parser = Parser::parse("foo(\"baz\",1);")?;

        let fn_call = FunctionCall {
            name: "foo",
            args: vec![
                Primitive::String("baz").into(),
                Primitive::Number(Number::I64(1)).into(),
            ],
        };
        // info!(?parser);
        assert_eq!(AstNode::FunctionCall(fn_call), parser.ast.nodes[0]);

        Ok(())
    }

    #[test]
    fn simple_struct() -> ParseResult<()> {
        setup_logger();

        let data = "
        pub struct Foo {
        i: i32,
        }
            ";

        let mut parser = Parser::parse(data)?;
        // info!(?parser);
        Ok(())
    }

    #[test]
    fn complex_struct() -> ParseResult<()> {
        setup_logger();

        let data = "
        struct BarBaz {
        i: string,
        }

        struct Baz {
        baz: BarBaz
        }

        struct Foo {
        bar: Baz
        }
        ";

        let tokens = Tokenizer::tokenize(data);
        let tokens = tokens.tokens();

        // for (span, t) in tokens {
        //     info!("source index {:?} {t:?}", &data[span.start..span.end]);
        // }
        // panic!();
        let mut i = 0;
        let mut typer = Typer::default();
        let (name, decl, inc) = StructDecl::parse_ctx_mut(&mut typer, &tokens[i..])?;
        // info!(?name, ?decl,);
        typer.register(name, ComplexTypeDecl::StructDecl(decl));
        i += inc;
        let (name, decl, inc) = StructDecl::parse_ctx_mut(&mut typer, &tokens[i..])?;
        // info!(?name, ?decl,);
        typer.register(name, ComplexTypeDecl::StructDecl(decl));
        i += inc;
        let (name, decl, inc) = StructDecl::parse_ctx_mut(&mut typer, &tokens[i..])?;
        // info!(?name, ?decl,);
        typer.register(name, ComplexTypeDecl::StructDecl(decl));

        if let ComplexType::Known(complex_type_decl) = typer.get("Foo").unwrap()
            && let ComplexTypeDecl::StructDecl(struct_decl) = complex_type_decl
        {
            for (name, val) in &struct_decl.fields {
                let TypeID::Complex(complex_type_id) = val else {
                    panic!()
                };
                match typer.get_id(*complex_type_id).unwrap() {
                    crate::types::ComplexType::Known(complex_type_decl) => {
                        match complex_type_decl {
                            ComplexTypeDecl::StructDecl(struct_decl) => {
                                info!(?struct_decl)
                            }
                            ComplexTypeDecl::Enum(enum_decl) => todo!(),
                        }
                    }
                    crate::types::ComplexType::Unknown(complex_type_decl) => todo!(),
                };
            }
        } else {
            panic!()
        }

        Ok(())
    }

    #[test]
    fn enum_decl() -> ParseResult<()> {
        setup_logger();

        let data = "pub enum Foo {
        Bar = 10,
        }
        ";

        let mut parser = Parser::parse(data)?;

        // info!(?parser);

        Ok(())
    }

    #[test]
    fn enum_use() -> ParseResult<()> {
        setup_logger();

        let data = r#"enum Foo {
    Bar = 10,
    }
    fn foo() {
    let bar = Foo::Bar;
    }
    "#;

        let parser = Parser::parse(data)?;
        info!(?parser);

        Ok(())
    }

    #[test]
    fn float_test() -> ParseResult<()> {
        setup_logger();

        let data = "0.5";

        let tokenizer = Tokenizer::tokenize(data);
        let (prim, inc) = Primitive::parse(tokenizer.tokens())?;

        // info!(?prim);

        match prim {
            Primitive::Number(number) => match number {
                Number::F32(n) => assert_eq!(0.5, n),
                Number::F64(n) => assert_eq!(0.5, n),
                t => panic!("{t:?}"),
            },
            Primitive::String(_) => todo!(),
            Primitive::Bool(_) => todo!(),
        };

        Ok(())
    }

    #[test]
    fn bool_test() -> ParseResult<()> {
        setup_logger();

        let data = r#"fn foo() {
        let bar = true;
        let baz = false;
        }"#;

        // let binding = Tokenizer::tokenize(data);
        // let tokens = binding.tokens();
        // info!(?tokens);
        //
        // for (span, t) in tokens {
        //     info!("source index {:?} {t:?}", &data[span.start..span.end]);
        // }

        let parser = Parser::parse(data)?;
        info!(?parser);

        Ok(())
    }

    #[test]
    fn struct_use() -> ParseResult<()> {
        setup_logger();

        let data = r#"
            struct Baz {
            bar_baz: i32,
            }
            struct Bar {
            bar: i16,
            baz: Baz,
            }

            struct Foo {
            bar: string,
            foo: Bar,
            baz: i8,
            }

            fn foo() {
            let bar = Foo {
            bar: "bar_baz_foo",
            foo: Bar {
                bar: 5,
                baz: Baz {
                bar_baz: 10,
                },
            },
            baz: 15,
            };
            }
            "#;

        let parser = Parser::parse(data)?;
        let f_n = &parser.ast.nodes[0];
        // info!("{f_n:?}");

        Ok(())
    }

    #[test]
    fn multiple_variables() -> ParseResult<()> {
        setup_logger();

        let data = r#"
        struct Foo {
        i: i64,
        }
        fn foo() {
        let bar = "urmom";
        let baz = "foo";
        let bazz = baz;
        let bar_baz = 10;
        let foo = Foo {
        i:15
        };
        }
            "#;

        let parser = Parser::parse(data)?;
        let decl = match &parser.ast.nodes[0] {
            AstNode::Function(function_decl) => function_decl,
            _ => panic!(),
        };

        for val in decl.block.iter() {
            match &val {
                BlockValue::VariableDecl(variable) => {}
                t => panic!("{t:?}"),
            }
            // info!(?val);
        }

        Ok(())
    }

    #[test]
    fn return_variable() -> ParseResult<()> {
        setup_logger();

        let data = r#"fn foo() u64 {
        let foo: u64 = 10;

        return foo;
    }"#;

        let parser = Parser::parse(data)?;
        // info!(?parser);

        Ok(())
    }

    #[test]
    fn var_use() -> ParseResult<()> {
        setup_logger();

        let data = r#"
    fn foo() {
    let mut foo = 0;
    foo = 10;
    }
        "#;
        let parser = Parser::parse(data)?;
        info!(?parser);
        Ok(())
    }

    #[test]
    fn var_add() -> ParseResult<()> {
        setup_logger();
        let data = r#"fn foo(){
    let mut foo = 0 + 10 * 20 / 30;
    }"#;
        let parser = Parser::parse(data)?;
        info!(?parser);
        Ok(())
    }

    #[test]
    fn function_block() -> ParseResult<()> {
        setup_logger();

        let data = r#"fn foo() {
    let mut foo =0;
    foo = 10;
    {
    let bar = foo;
    }
    }"#;
        let parser = Parser::parse(data)?;
        info!(?parser);
        Ok(())
    }
}

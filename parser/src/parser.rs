#![allow(clippy::match_bool)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::ref_option)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::used_underscore_binding)]

use std::{
    collections::HashSet,
    fmt::{Debug, Display},
};

use tracing::{debug, error, info, warn};

use crate::types::{
    Array, Block, BlockValue, ComplexType, ComplexTypeDecl, ComplexTypeID, ComplexTypeName,
    ComplexValue, Condition, ConditionItem, ElseIfStatement, Enum, EnumDecl, Frame, FunctionCall,
    FunctionDecl, IfStatement, MathExpr, MathItem, Number, Operation, Primitive, PrimitiveID,
    Struct, StructAccess, StructDecl, TypeDecl, TypeDeclReturn, TypeID, Typer, Value, Variable,
    VariableUse, VariableValue, VariableValueReturn, Visibility,
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

    #[must_use]
    pub fn nodes(&self) -> &[AstNode<'a>] {
        &self.nodes
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    #[allow(unused)]
    typer: Typer<'a>,

    ast: Ast<'a>,
}

impl<'a> Parser<'a> {
    #[must_use]
    pub fn ast(&self) -> &Ast<'_> {
        &self.ast
    }

    /// # Panics
    ///
    /// # Errors
    pub fn parse(s: &'a str) -> ParseResult<Self> {
        let tokenizer = Tokenizer::tokenize(s);

        let iter = tokenizer.tokens();

        let mut idx = 0;
        let mut needs_inc = true;

        let mut typer = Typer::default();
        let mut ast = Ast::default();

        let unknown_types: HashSet<&str> = HashSet::default();

        loop {
            if idx >= iter.len() {
                break;
            }
            let token = &iter[idx];
            match token.1 {
                Token::Str(_) => {
                    if let Some((_span, peek)) = iter.get(idx + 1)
                        && peek == &Token::LeftParen
                    {
                        let (call, i) =
                            FunctionCall::parse_ctx(&ast, &iter[idx.saturating_sub(1)..])?;
                        ast.push(AstNode::FunctionCall(call));
                        idx += i;
                    }
                }
                Token::TypeID(_type_id) => {
                    // debug!(?type_id);
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
            }
            if needs_inc {
                idx += 1;
            }
        }

        for typ in unknown_types {
            println!("{typ:?}");
        }

        Ok(Self { typer, ast })
    }
}

impl<'a> Primitive<'a> {
    // #[instrument(name = "Primitive::parse", skip_all, err)]
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

    // #[instrument(name = "Primitive::parse_ctx", skip_all, err)]
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
                                PrimitiveID::String | PrimitiveID::Bool => {
                                    return Err(ParseError::IncorrectType);
                                }
                            }),
                            1,
                        ));
                    }
                    (Token::Str(s) | Token::QuotedString(s), TypeID::Primitive(primitive_id)) => {
                        if primitive_id == &PrimitiveID::String {
                            return Ok((Primitive::String(s), 1));
                        }
                        return Err(ParseError::IncorrectType);
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
    // #[instrument(name = "Enum::parse_ctx", skip_all, err)]
    pub fn parse_ctx<'a>(
        ctx: &Typer<'a>,
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Self, usize)> {
        debug!("parsing enum");
        enum State {
            Name,
            Ident,
            Value,
        }

        let mut state = State::Name;
        let mut left = None;
        let mut field = None;

        for (i, (_span, token)) in tokens.iter().enumerate() {
            debug!(?token);
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
                    Token::Comma | Token::RightBracket => {
                        return Ok((
                            Enum {
                                id: ctx.id(left.unwrap()).unwrap(),
                                field: field.unwrap(),
                            },
                            i,
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
    // #[instrument(name = "StructDecl::parse_ctx_mut", skip_all, err)]
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

        for (i, (_span, token)) in tokens.iter().enumerate() {
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
                        }
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
                                                    assert!(nn.can_fit(n));
                                                }
                                                _ => panic!(),
                                            }
                                        }
                                        matching += 1;
                                    }
                                    None => panic!(),
                                }
                            }
                            assert!(matching == unknown.fields.len());
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
    // #[instrument(name = "Struct::parse_ctx", skip_all, err)]
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
        let mut depth: usize = 0;

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

                        // finds nested struct through assuming its already read the field name so
                        // then anything that is :<str> { is a struct
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
                    Token::LeftAngleBracket => {
                        depth += 1;
                        i += 1;
                    }
                    Token::RightAngleBracket => {
                        depth = depth.saturating_sub(1);
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
                            depth,
                            parent_name,
                            Value::Complex(crate::types::ComplexValue::Struct(comp)),
                        ));
                        i += 1;
                    }
                    Token::QuotedString(_s) | Token::Number(_s) => {
                        let (val, inc) = Primitive::parse(&[(*span, *token)])?;
                        stack.last_mut().unwrap().fields.push((
                            depth,
                            field_name.unwrap(),
                            Value::Primitive(val),
                        ));
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
    // #[instrument(name = "EnumDecl::parse", skip_all, err)]
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

        for (i, (_span, token)) in tokens.iter().enumerate() {
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

                    t => panic!("{t:?} {:?}", &tokens[i..]),
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

// #[instrument(name = "parse_condition", skip(_ctx, tokens), ret)]
fn parse_condition<'a>(
    _ctx: &Typer<'a>,
    tokens: &[(Span, Token<'a>)],
) -> ParseResult<(Vec<ConditionItem<'a>>, usize)> {
    let mut value: Vec<MathItem<'a>> = vec![];
    let mut cond = vec![];
    let mut i = 0;

    while let Some((span, token)) = tokens.get(i) {
        match token {
            Token::LeftAngleBracket => {
                cond.push(ConditionItem::Item(VariableValue::Expr(value)));
                return Ok((cond, i));
            }
            Token::Plus => value.push(MathItem::Op(Operation::Add)),
            Token::Minus => value.push(MathItem::Op(Operation::Sub)),
            Token::Multiply => value.push(MathItem::Op(Operation::Mult)),

            Token::Number(_n) | Token::QuotedString(_n) => {
                let prim = Primitive::parse(&[(*span, *token)])?.0;
                value.push(MathItem::Prim(prim));
                // debug!(?value);
            }

            Token::Exclamation => {
                if let (_, Token::Equal) = tokens.get(i + 1).unwrap() {
                    cond.push(ConditionItem::Condition(Condition::NotEqual));
                }
            }
            Token::Equal => {
                if let (_, Token::Equal) = tokens.get(i + 1).unwrap() {
                    cond.push(ConditionItem::Condition(Condition::Equal));
                }
            }
            Token::LeftCarrot => {
                if let (_, Token::Equal) = tokens.get(i + 1).unwrap() {
                    cond.push(ConditionItem::Condition(Condition::LessthanOrEqual));
                } else {
                    cond.push(ConditionItem::Condition(Condition::LessThan));
                }
            }
            Token::RightCarrot => {
                if let (_, Token::Equal) = tokens.get(i + 1).unwrap() {
                    cond.push(ConditionItem::Condition(Condition::GreaterThanOrEqual));
                } else {
                    cond.push(ConditionItem::Condition(Condition::GreaterThan));
                }
            }

            Token::Str(s) => cond.push(ConditionItem::Item(VariableValue::Name(s))),

            t => panic!("{t:?}"),
        }
        i += 1;
    }

    Err(ParseError::IncorrectType)
}

impl<'a> Block<'a> {
    // #[instrument(name = "Block::parse_ctx", skip(ctx, tokens))]
    fn parse_ctx(
        ctx: &Typer<'a>,
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Vec<(usize, BlockValue<'a>)>, usize)> {
        enum State {
            Block,
            Return,
        }

        let mut state = State::Block;
        let mut block: Vec<(usize, BlockValue)> = vec![];
        let mut i = 0;

        let mut level = 0;

        while let Some((_, token)) = tokens.get(i) {
            debug!(?token);
            match state {
                State::Block => match token {
                    Token::Return => state = State::Return,
                    Token::Else => {
                        level += 1;
                        if let Some((_, Token::If)) = tokens.get(i + 1) {
                            let (cond, inc) = parse_condition(ctx, &tokens[i + 2..])?;
                            block.push((level, BlockValue::ElseIf(ElseIfStatement { cond })));
                            i += inc + 3;
                        } else {
                            block.push((level, BlockValue::Else));
                            i += 1;
                        }
                    }
                    Token::If => {
                        let (cond, inc) = parse_condition(ctx, &tokens[i + 1..])?;
                        i += inc + 1;

                        level += 1;
                        block.push((level, BlockValue::IfStatement(IfStatement { cond })));
                    }
                    Token::LeftAngleBracket => {
                        // level += 1;
                        // block.push((level, BlockValue::Block));
                    }
                    Token::RightAngleBracket => {
                        debug!("found closing bracket {level}");
                        match level {
                            0 => return Ok((block, i)),
                            _ => level = level.saturating_sub(1),
                        }
                    }
                    Token::Let => {
                        let (var_name, mutable, id, value, inc) =
                            Variable::parse_ctx(ctx, &tokens[i..])?;

                        i += inc;
                        debug!(?var_name, ?mutable, ?id, ?value);

                        let mut b_val: Option<BlockValue> = None;
                        match &value {
                            VariableValue::StructAccess(struct_access) => {
                                let access_len = struct_access.fields().len();
                                match &block
                                    .iter()
                                    .rfind(|(_level, val)| match val {
                                        BlockValue::VariableDecl(decl) => {
                                            debug!(?struct_access, ?decl);
                                            struct_access.name() == decl.0
                                        }
                                        _ => false,
                                    })
                                    .unwrap()
                                    .1
                                {
                                    BlockValue::VariableDecl(decl) => match &decl.1.val {
                                        VariableValue::Value(Value::Complex(
                                            ComplexValue::Struct(struc),
                                        )) => {
                                            debug!(?struc);
                                            if access_len == 1 {
                                                for (_level, name, v) in &struc.fields {
                                                    if struct_access.fields()[0] == *name {
                                                        let variable = BlockValue::VariableDecl((
                                                            var_name,
                                                            Variable::new(
                                                                v.id(Some(ctx)).unwrap(),
                                                                mutable,
                                                                VariableValue::Value(v.clone()),
                                                            ),
                                                        ));
                                                        b_val = Some(variable);
                                                    }
                                                }
                                            } else {
                                                for (i, (_level, _name, upper_v)) in
                                                    struc.fields.iter().enumerate()
                                                {
                                                    if let Value::Complex(ComplexValue::Struct(
                                                        strukt,
                                                    )) = upper_v
                                                    {
                                                        for (_lvl, name, v) in &strukt.fields {
                                                            if struct_access.fields()[i + 1]
                                                                == *name
                                                                && i + 1
                                                                    >= struct_access
                                                                        .fields()
                                                                        .len()
                                                                        .strict_sub(1)
                                                            {
                                                                let variable =
                                                                    BlockValue::VariableDecl((
                                                                        var_name,
                                                                        Variable::new(
                                                                            v.id(Some(ctx))
                                                                                .unwrap(),
                                                                            mutable,
                                                                            VariableValue::Value(
                                                                                v.clone(),
                                                                            ),
                                                                        ),
                                                                    ));
                                                                b_val = Some(variable);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            assert!(b_val.is_some());
                                        }
                                        t => panic!("{t:?}"),
                                    },
                                    t => panic!("{t:?}"),
                                }
                            }

                            VariableValue::Value(_val) => {
                                b_val = Some(BlockValue::VariableDecl((
                                    var_name,
                                    Variable::new(_val.id(Some(ctx)).unwrap(), mutable, value),
                                )));
                            }
                            VariableValue::Name(name) => {
                                let f = block.iter().rfind(|(_level, val)| match val {
                                    BlockValue::VariableDecl(decl) => {
                                        debug!(?name, ?decl);
                                        *name == decl.0
                                    }
                                    _ => false,
                                });
                                if let Some((_, v)) = f {
                                    match v {
                                        BlockValue::VariableDecl(decl) => {
                                            b_val = Some(BlockValue::VariableDecl((
                                                var_name,
                                                decl.1.clone(),
                                            )));
                                        }
                                        t => panic!("{t:?}"),
                                    }
                                }
                            }
                            VariableValue::Expr(_math_items) => {
                                b_val = Some(BlockValue::VariableDecl((
                                    var_name,
                                    Variable::new(id.unwrap(), mutable, value),
                                )));
                            }
                            VariableValue::FunctionCall(_function_call) => {
                                b_val = Some(BlockValue::VariableDecl((
                                    var_name,
                                    Variable::new(id.unwrap(), mutable, value),
                                )));
                            }
                            VariableValue::Empty => panic!(),
                        }

                        block.push((level, b_val.unwrap()));
                    }

                    Token::Str(s) => {
                        if let Some((_, next)) = tokens.get(i + 1)
                            && *next == Token::LeftParen
                        {
                            let (call, inc) =
                                FunctionCall::parse_ctx(&Ast::default(), &tokens[i..])?;
                            block.push((level, BlockValue::FunctionCall(call)));
                            i += inc;
                            assert_eq!(tokens[i].1, Token::Semicolon);
                            i += 1;
                            continue;
                        }
                        let (var_name, inc) = VariableUse::parse_ctx(ctx, &tokens[i..])?;
                        i += inc;

                        let mut reass = None;
                        for b in block.iter_mut().rev() {
                            match &mut b.1 {
                                BlockValue::Else => {}
                                BlockValue::Block => panic!(), // {
                                //     for values in block_values.iter() {
                                //         match values {
                                //             BlockValue::VariableDecl(_decl) => {
                                //                 reass = match var_name {
                                //                     VariableValueReturn::Assignment(
                                //                         ref variable_value,
                                //                     )
                                //                     | VariableValueReturn::ReAssignment(
                                //                         ref variable_value,
                                //                     ) => Some(variable_value.clone()),
                                //                     VariableValueReturn::Expr(ref math_items) => {
                                //                         Some(VariableValue::Expr(
                                //                             math_items.clone(),
                                //                         ))
                                //                     }
                                //                 };
                                //             }
                                //             t => panic!("{t:?}"),
                                //         }
                                //     }
                                // }
                                BlockValue::VariableDecl(_decl) => match &var_name {
                                    VariableValueReturn::Assignment(variable_value) => {
                                        reass = Some(variable_value.clone());
                                    }
                                    VariableValueReturn::ReAssignment(_variable_value) => todo!(),
                                    VariableValueReturn::Expr(_math_items) => todo!(),
                                },
                                t => panic!("{t:?}"),
                            }
                        }
                        if level == 0 {
                            block.push((
                                level,
                                BlockValue::VariableReAssignment((s, reass.unwrap())),
                            ));
                        } else {
                            match block.last_mut() {
                                None => block.push((
                                    level,
                                    BlockValue::VariableReAssignment((s, reass.unwrap())),
                                )),
                                Some(b) => match &mut b.1 {
                                    BlockValue::Block | BlockValue::VariableDecl(_) => {
                                        block.push((
                                            level,
                                            BlockValue::VariableReAssignment((s, reass.unwrap())),
                                        ));
                                    }
                                    t => panic!("{t:?}"),
                                },
                            }
                        }
                    }
                    _t => {} //panic!("{:?}", &tokens[i..]),
                },
                State::Return => {
                    match token {
                        Token::Str(s) => {
                            // handle returning a struct being made as the return value? ie
                            // return Foo { ... };
                            block.push((0, BlockValue::Return(VariableValue::Name(s))));
                        }
                        Token::Number(_n) | Token::QuotedString(_n) => {
                            let (prim, _i) = Primitive::parse(&tokens[i..])?;
                            block.push((
                                0,
                                BlockValue::Return(VariableValue::Value(Value::Primitive(prim))),
                            ));
                            // let prim_id = prim.id();
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
                        Token::RightAngleBracket => {
                            level = level.strict_sub(1);
                            block.push((level, BlockValue::Return(VariableValue::Empty)));
                            state = State::Block;
                        }
                        t => panic!("{t:?} {:?}", &tokens[i..]),
                    }
                }
            }
            i += 1;
        }

        Ok((block, i))
    }
}

impl<'a> FunctionDecl<'a> {
    // #[instrument(name = "FunctionDecl::parse_ctx", skip_all, err)]
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

        while let Some((_span, token)) = tokens.get(i) {
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
                                args.push((mutable, found_arg.unwrap(), TypeID::from(*t)));
                            }
                            None => {
                                decl.args =
                                    Some(vec![(mutable, found_arg.unwrap(), TypeID::from(*t))]);
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
    // #[instrument(name = "FunctionCall::parse_ctx", skip_all, err)]
    pub fn parse_ctx(ctx: &Ast, tokens: &[(Span, Token<'a>)]) -> ParseResult<(Self, usize)> {
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

        for (i, (_span, token)) in tokens.iter().enumerate() {
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
                        assert!(!needs_comma);
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
                        for node in ctx.nodes() {
                            if let AstNode::Function(function_decl) = node
                                && function_decl.name == fn_name.unwrap()
                            {
                                return Ok((
                                    FunctionCall {
                                        name: fn_name.unwrap(),
                                        args,
                                        return_type: function_decl.return_type,
                                    },
                                    i,
                                ));
                            }
                        }
                        return Ok((
                            FunctionCall {
                                name: fn_name.unwrap(),
                                args,
                                return_type: None,
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
    // #[instrument(name = "TypeDecl::parse_ctx_mut", skip_all, err)]
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
    // #[instrument(name = "VariableValue::parse_ctx", skip_all)]
    pub fn parse_ctx(
        ctx: &(Option<TypeID>, &Typer<'a>),
        tokens: &[(Span, Token<'a>)],
    ) -> ParseResult<(Self, usize)> {
        let value: VariableValue;

        let type_id = &ctx.0;
        let typer = &ctx.1;
        let mut i = 0;

        loop {
            let Some((_span, token)) = tokens.get(i) else {
                panic!()
            };

            match token {
                Token::Str(s) => {
                    if let Some(typ) = typer.get(s) {
                        match typ {
                            ComplexType::Known(complex_type_decl) => match complex_type_decl {
                                ComplexTypeDecl::StructDecl(_struct_decl) => {
                                    let (decl, inc) = Struct::parse_ctx(typer, &tokens[i..])?;
                                    i += inc;
                                    value = VariableValue::Value(Value::Complex(
                                        ComplexValue::Struct(decl),
                                    ));
                                    return Ok((value, i));
                                }
                                ComplexTypeDecl::Enum(_enum_decl) => {
                                    let (decl, inc) = Enum::parse_ctx(ctx.1, &tokens[i..])?;
                                    i += inc;
                                    error!(?decl, "{:?}", &tokens[i..]);
                                    value = VariableValue::Value(Value::Complex(
                                        ComplexValue::Enum(decl),
                                    ));
                                    return Ok((value, i));
                                }
                            },
                            ComplexType::Unknown(_complex_type_decl) => todo!(),
                        }
                    } else {
                        let mut vec = vec![];
                        // if you have foo.bar it skips foo so its just [., "bar"]
                        i += 1;

                        while let (Some((_, Token::Dot)), Some((_, Token::Str(next1)))) =
                            (tokens.get(i), tokens.get(i + 1))
                        {
                            vec.push(*next1);
                            i += 2;
                        }
                        if vec.is_empty() {
                            return Ok((VariableValue::Name(s), i.saturating_sub(1)));
                        }
                        let sp = StructAccess::new(s, vec);

                        value = VariableValue::StructAccess(sp);
                        // caller has to receive the semicolon so it parses it as [...]; where this
                        // function deals with the [...] and lets the caller handle the trailing semicolon
                        return Ok((value, i.saturating_sub(1)));
                    }
                }

                Token::Number(_) | Token::QuotedString(_) | Token::True | Token::False => {
                    let (prim, inc) = Primitive::parse_ctx(type_id, &tokens[i..])?;
                    value = VariableValue::Value(Value::Primitive(prim));
                    match tokens.get(i + 1) {
                        Some((_span, token)) => match token {
                            Token::Plus | Token::Minus | Token::Multiply | Token::Divide => {
                                panic!("{token:?} {:?}", &tokens[i..])
                            }
                            Token::LeftCarrot => todo!("potential less than expr"),
                            Token::RightCarrot => todo!("potential greater than expr"),
                            _ => return Ok((value, i + inc)),
                        },
                        _ => return Ok((value, i)),
                    }
                }
                Token::LeftBracket => {
                    i += 1;
                    let mut vec: Vec<Value> = vec![];

                    loop {
                        match tokens.get(i) {
                            Some((_, Token::RightBracket)) => {
                                return Ok((
                                    VariableValue::Value(Value::Array(Array {
                                        type_id: vec[0].id(Some(ctx.1)).unwrap(),
                                        values: vec,
                                    })),
                                    i + 1,
                                ));
                            }
                            Some((_, Token::Comma)) | Some((_, Token::RightAngleBracket)) => {
                                i += 1;
                            }
                            Some((_, _)) => {
                                let (val, inc) =
                                    VariableValue::parse_ctx(ctx, &tokens[i..]).unwrap();
                                i += inc;
                                info!(?val, "{:?}", &tokens[i..]);
                                match val {
                                    VariableValue::Value(value) => {
                                        vec.push(value);
                                    }
                                    t => todo!("{t:?}"),
                                }
                            }

                            t => todo!("{t:?}"),
                        }
                    }
                }
                t => panic!("{t:?} {:?}", &tokens[i..]),
            }
        }
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

        while let Some((_span, token)) = tokens.get(i) {
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
                        VariableValueReturn::Assignment(variable_value)
                        | VariableValueReturn::ReAssignment(variable_value) => Some(variable_value),
                        VariableValueReturn::Expr(math_items) => {
                            type_id = Some(TypeID::Primitive(match &math_items[0] {
                                MathItem::Prim(primitive) => primitive.id(),
                                MathItem::Op(_operation) => todo!(),
                            }));
                            Some(VariableValue::Expr(math_items))
                        }
                    };
                    state = VariableState::Semicolon;
                    // continue because without it, we finish the loop incrementing i but we are on
                    // the semicolon
                    continue;
                }
                VariableState::Semicolon => match token {
                    Token::Semicolon => {
                        return Ok((name.unwrap(), mutable, type_id, value.unwrap(), i));
                    }
                    _ => panic!("{token} {name:?} {value:?} {:?}", &tokens[i..]),
                },
            }
            i += 1;
        }

        panic!()
    }
}

impl<'a> MathExpr {
    // #[instrument(name = "MathExpr::parse", skip_all, err)]
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
                    items.push(MathItem::Prim(Primitive::parse(&[(*span, *token)])?.0));
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
                        if name.is_none() {
                            name = Some(VariableValue::Name(s));
                        } else {
                            assert!(value.is_none());
                            value = Some(VariableValue::Name(s));
                        }
                        println!("{name:?} {value:?}");
                        state = State::Operator;
                        i += 1;
                    }
                    Token::Number(_n) => {
                        let prim = VariableValue::Value(Value::Primitive(
                            Primitive::parse(&[(*span, *tok)])?.0,
                        ));

                        if name.is_none() {
                            name = Some(prim);
                        } else {
                            assert!(value.is_none());
                            value = Some(prim);
                        }

                        state = State::Operator;
                        i += 1;
                    }
                    Token::Dot => {
                        panic!();
                    }
                    Token::Semicolon => {
                        return Ok((VariableValueReturn::Assignment(name.unwrap()), i));
                    }
                    t => panic!("{t:?} {tokens:?}"),
                },
                State::Operator => {
                    match (tok, tokens.get(i + 1)) {
                        (Token::Equal, _) => {
                            op = Some(Operation::Assign);
                            i += 1;
                        }
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
                        (Token::Exclamation, Some((_span, Token::Equal))) => {
                            todo!("should this lang support things like let foo = bar != baz")
                        }
                        (
                            Token::Number(_n) | Token::QuotedString(_n),
                            Some((span, Token::Semicolon)),
                        ) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            let prim = VariableValue::Value(Value::Primitive(
                                Primitive::parse(&[(*span, *tok)])?.0,
                            ));
                            return Ok((VariableValueReturn::Assignment(prim), i + 1));
                        }
                        (Token::Str(_s), Some((_span, Token::Equal))) => {
                            let (val, inc) =
                                VariableValue::parse_ctx(&(None, _ctx), &tokens[i + 2..])?;
                            // [Token::Str(_), Token::Equal, Val, Token::Semicolon];
                            // ^ start         ^ end         ^ goal
                            // so we do i += inc + 2;
                            i += inc + 2;
                            return Ok((VariableValueReturn::Assignment(val), i));
                        }
                        (Token::Str(_s), _) => {
                            let (val, inc) = VariableValue::parse_ctx(&(None, _ctx), &tokens[i..])?;
                            // [Token::Str(_), Token::Equal, Val, Token::Semicolon];
                            // ^ start         ^ end         ^ goal
                            // so we do i += inc + 2;
                            i += inc + 1;
                            return Ok((VariableValueReturn::Assignment(val), i));
                        }
                        (
                            Token::Number(_n),
                            Some((
                                _span,
                                Token::Plus | Token::Minus | Token::Multiply | Token::Divide,
                            )),
                        ) => {
                            let (val, inc) = MathExpr::parse(&tokens[i..])?;
                            return Ok((VariableValueReturn::Expr(val), i + inc + 1));
                        }
                        (Token::True, Some((_span, Token::Semicolon))) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            return Ok((
                                VariableValueReturn::Assignment(VariableValue::Value(
                                    Value::Primitive(Primitive::Bool(true)),
                                )),
                                i + 1,
                            ));
                        }
                        (Token::False, Some((_span, Token::Semicolon))) => {
                            assert_eq!(op.unwrap(), Operation::Assign);
                            return Ok((
                                VariableValueReturn::Assignment(VariableValue::Value(
                                    Value::Primitive(Primitive::Bool(false)),
                                )),
                                i + 1,
                            ));
                        }
                        (Token::LeftBracket, Some((_span, _t))) => {
                            let (val, inc) = VariableValue::parse_ctx(&(None, _ctx), &tokens[i..])?;
                            i += inc;
                            return Ok((VariableValueReturn::Assignment(val), i));
                        }

                        t => {
                            panic!("{t:?} {op:?}");
                        }
                    }
                    state = State::Name;
                }
            }
        }

        Err(ParseError::IncorrectType)
    }
}

#[cfg(test)]
mod parseable {

    use std::collections::HashMap;

    use tracing::info;
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::fmt::format::FmtSpan;

    use crate::parser::{AstNode, FunctionDecl, ParseError, ParseResult, Parser};
    use crate::types::{
        BlockValue, ComplexType, ComplexTypeDecl, ComplexTypeID, ComplexTypeName, ComplexValue,
        Enum, EnumDecl, FunctionCall, MathItem, Number, Operation, Primitive, PrimitiveID, Struct,
        StructDecl, TypeID, Value, Variable, VariableValue, Visibility,
    };

    use tokenizer::Tokenizer;

    #[inline]
    fn setup_logger() {
        _ = tracing_subscriber::fmt()
            .with_max_level(LevelFilter::DEBUG)
            .with_span_events(FmtSpan::ACTIVE)
            .try_init();
    }

    fn is_function<'a>(node: &'a AstNode<'_>) -> &'a FunctionDecl<'a> {
        match node {
            AstNode::Function(function_decl) => function_decl,
            _ => panic!("{node:?} was not a function"),
        }
    }

    #[test]
    fn parse_primitive_number() -> Result<(), ParseError> {
        setup_logger();

        let data = r#"fn foo() {
        let foo = 10;
        }"#;
        let parser = Parser::parse(data)?;

        let function = is_function(&parser.ast.nodes[0]);

        for (_scope_id, val) in function.block.iter() {
            assert_eq!(
                val,
                &BlockValue::VariableDecl((
                    "foo",
                    Variable::from_value(
                        Value::Primitive(Primitive::Number(Number::I64(10))),
                        false,
                        None
                    )
                ))
            );
        }

        Ok(())
    }

    #[test]
    fn parse_primitive_string() -> Result<(), ParseError> {
        setup_logger();

        let data = r#"
        fn foo() {
            let foo = "foo_bar_baz"; 
        }"#;
        let parser = Parser::parse(data)?;
        let function = is_function(&parser.ast.nodes[0]);

        for (_scope_id, val) in function.block.iter() {
            assert_eq!(
                val,
                &BlockValue::VariableDecl((
                    "foo",
                    Variable::from_value(
                        Value::Primitive(Primitive::String("foo_bar_baz")),
                        false,
                        None
                    )
                ))
            );
        }

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
            block: vec![(
                0,
                BlockValue::Return(crate::types::VariableValue::Value(Value::Primitive(
                    Primitive::Number(Number::I64(0)),
                ))),
            )],
            return_type: Some(PrimitiveID::I32.into()),
        };

        assert_eq!(AstNode::Function(decl), parser.ast.nodes[0]);

        Ok(())
    }

    #[test]
    fn function_call() -> Result<(), ParseError> {
        setup_logger();

        let data = r#"
        fn foo(baz: string, bar: u64) {}

        fn bar() {
        foo("baz",1);
        }
            "#;
        let parser = Parser::parse(data)?;

        let fn_call = FunctionCall {
            name: "foo",
            args: vec![
                Primitive::String("baz").into(),
                Primitive::Number(Number::I64(1)).into(),
            ],
            return_type: None,
        };
        info!(?parser);
        match &parser.ast().nodes()[1] {
            AstNode::Function(function_decl) => {
                for (_scope, val) in &function_decl.block {
                    match val {
                        BlockValue::FunctionCall(function_call) => {
                            assert_eq!(function_call, &fn_call)
                        }
                        _ => panic!(),
                    }
                }
            }
            _ => panic!(),
        }

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

        let parser = Parser::parse(data)?;

        assert_eq!(
            parser.typer.get("Foo").unwrap(),
            &ComplexType::Known(ComplexTypeDecl::StructDecl(StructDecl {
                visibility: Visibility::Pub,
                fields: HashMap::from([("i", TypeID::Primitive(PrimitiveID::I32))])
            }))
        );
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

        let parser = Parser::parse(data)?;

        if let ComplexType::Known(complex_type_decl) = parser.typer.get("Foo").unwrap()
            && let ComplexTypeDecl::StructDecl(struct_decl) = complex_type_decl
        {
            for val in struct_decl.fields.values() {
                let TypeID::Complex(complex_type_id) = val else {
                    panic!()
                };
                match parser.typer.get_id(*complex_type_id).unwrap() {
                    crate::types::ComplexType::Known(complex_type_decl) => {
                        match complex_type_decl {
                            ComplexTypeDecl::StructDecl(struct_decl) => {
                                info!(?struct_decl)
                            }
                            ComplexTypeDecl::Enum(_enum_decl) => todo!(),
                        }
                    }
                    crate::types::ComplexType::Unknown(_complex_type_decl) => todo!(),
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

        let parser = Parser::parse(data)?;

        let ComplexType::Known(ComplexTypeDecl::Enum(decl)) = parser.typer.get("Foo").unwrap()
        else {
            panic!()
        };

        assert_eq!(
            decl,
            &EnumDecl {
                visibility: Visibility::Pub,
                fields: HashMap::from([("Bar", Number::I64(10))])
            }
        );

        info!(?parser);

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

        let AstNode::Function(decl) = parser.ast().nodes().first().unwrap() else {
            panic!();
        };

        match decl.block[0].1 {
            BlockValue::VariableDecl(ref var_decl) => {
                let typeid = parser.typer.id("Foo").unwrap();
                assert_eq!(
                    var_decl,
                    &(
                        "bar",
                        Variable::from_value(
                            Value::Complex(crate::types::ComplexValue::Enum(Enum {
                                id: typeid,
                                field: Number::I64(10)
                            })),
                            false,
                            None
                        )
                    )
                );
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn float_test() -> ParseResult<()> {
        setup_logger();

        let data = "0.5";

        let tokenizer = Tokenizer::tokenize(data);
        let (prim, _inc) = Primitive::parse(tokenizer.tokens())?;

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

        let parser = Parser::parse(data)?;
        let AstNode::Function(decl) = parser.ast().nodes().first().unwrap() else {
            panic!()
        };

        match (
            &decl.block.first().unwrap().1,
            &decl.block.get(1).unwrap().1,
        ) {
            (BlockValue::VariableDecl(first), BlockValue::VariableDecl(second)) => {
                assert_eq!(
                    first,
                    &(
                        "bar",
                        Variable::from_value(Value::Primitive(Primitive::Bool(true)), false, None)
                    )
                );
                assert_eq!(
                    second,
                    &(
                        "baz",
                        Variable::from_value(Value::Primitive(Primitive::Bool(false)), false, None)
                    )
                );
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn simple_struct_use() -> ParseResult<()> {
        setup_logger();

        let data = r#"
        struct BarBaz {
        i: string,
        }

        fn foo() {
        let bar = BarBaz {
        i: "foo",
        };
        }
        "#;

        let parser = Parser::parse(data)?;
        let function = is_function(&parser.ast.nodes[0]);
        assert_eq!(
            function.block[0].1,
            BlockValue::VariableDecl((
                "bar",
                Variable {
                    typeid: TypeID::Complex(ComplexTypeID::new(0)),
                    mutable: false,
                    val: VariableValue::Value(Value::Complex(ComplexValue::Struct(Struct {
                        name: ComplexTypeName::Known("BarBaz"),
                        fields: vec![(0, "i", Value::Primitive(Primitive::String("foo"))),]
                    })))
                }
            ))
        );
        info!("{:#?}", parser.ast());

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
        let AstNode::Function(decl) = &parser.ast.nodes[0] else {
            panic!()
        };

        let BlockValue::VariableDecl(var) = &decl.block[0].1 else {
            panic!()
        };

        assert_eq!(
            var,
            &(
                "bar",
                Variable {
                    typeid: TypeID::Complex(parser.typer.id("Foo").unwrap()),
                    mutable: false,
                    val: VariableValue::Value(Value::Complex(ComplexValue::Struct(Struct {
                        name: ComplexTypeName::Known("Foo"),
                        fields: vec![
                            (0, "bar", Value::Primitive(Primitive::String("bar_baz_foo"))),
                            (
                                0,
                                "foo",
                                Value::Complex(ComplexValue::Struct(Struct {
                                    name: ComplexTypeName::Known("Bar"),
                                    fields: vec![
                                        (
                                            1,
                                            "bar",
                                            Value::Primitive(Primitive::Number(Number::I64(5)))
                                        ),
                                        (
                                            1,
                                            "baz",
                                            Value::Complex(ComplexValue::Struct(Struct {
                                                name: ComplexTypeName::Known("Baz"),
                                                fields: vec![(
                                                    2,
                                                    "bar_baz",
                                                    Value::Primitive(Primitive::Number(
                                                        Number::I64(10)
                                                    ))
                                                )]
                                            }))
                                        )
                                    ]
                                }))
                            ),
                            (
                                0,
                                "baz",
                                Value::Primitive(Primitive::Number(Number::I64(15)))
                            )
                        ]
                    })))
                }
            )
        );

        Ok(())
    }

    #[test]
    fn struct_access() -> ParseResult<()> {
        setup_logger();

        let data = r#"

            struct Baz {
            baz: i64
            }

            struct Bar {
            bar: Baz,    
            }

            struct Foo {
            i: Bar 
            }

            fn foo() {
            let foo = Foo {
            i: Bar {
            bar: Baz {
            baz: 10,
            },
            },
            };
            let bar = foo.i;
            }
        "#;
        let parser = Parser::parse(data)?;
        let decl = match &parser.ast.nodes[0] {
            AstNode::Function(function_decl) => function_decl,
            _ => panic!(),
        };

        for (_scope_id, val) in decl.block.iter() {
            match &val {
                BlockValue::VariableDecl((name, variable)) => {
                    info!(%name, %variable);
                }
                t => panic!("{t:?}"),
            }
        }

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

        for (_scope_id, val) in decl.block.iter() {
            match &val {
                BlockValue::VariableDecl(variable) => {
                    info!(?variable);
                }
                t => panic!("{t:?}"),
            }
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
        let AstNode::Function(decl) = &parser.ast().nodes[0] else {
            panic!()
        };
        let BlockValue::Return(val) = &decl.block[1].1 else {
            panic!();
        };
        assert_eq!(val, &VariableValue::Name("foo"));
        info!(?val);

        Ok(())
    }

    #[test]
    fn var_use() -> ParseResult<()> {
        setup_logger();

        let data = r#"
    fn foo() {
    let mut foo = 0;
    foo = 10;
    }"#;
        let parser = Parser::parse(data)?;

        let AstNode::Function(decl) = &parser.ast().nodes[0] else {
            panic!()
        };
        let BlockValue::VariableDecl(val) = &decl.block[0].1 else {
            panic!();
        };
        let BlockValue::VariableReAssignment(val_assgn) = &decl.block[1].1 else {
            panic!();
        };

        assert_eq!(
            val,
            &(
                "foo",
                Variable::from_value(
                    Value::Primitive(Primitive::Number(Number::I64(0))),
                    true,
                    None
                ),
            )
        );
        assert_eq!(
            val_assgn,
            &(
                "foo",
                VariableValue::Value(Value::Primitive(Primitive::Number(Number::I64(10))))
            )
        );
        Ok(())
    }

    #[test]
    fn var_add() -> ParseResult<()> {
        setup_logger();
        let data = r#"fn foo(){
    let mut foo = 0 + 10 * 20 / 30;
    }"#;
        let parser = Parser::parse(data)?;

        let AstNode::Function(decl) = &parser.ast().nodes[0] else {
            panic!()
        };
        let BlockValue::VariableDecl(val) = &decl.block[0].1 else {
            panic!();
        };
        assert_eq!(
            val,
            &(
                "foo",
                Variable {
                    typeid: TypeID::Primitive(PrimitiveID::I64),
                    mutable: true,
                    val: VariableValue::Expr(vec![
                        MathItem::Prim(Primitive::Number(Number::I64(0))),
                        MathItem::Op(Operation::Add),
                        MathItem::Prim(Primitive::Number(Number::I64(10))),
                        MathItem::Op(Operation::Mult),
                        MathItem::Prim(Primitive::Number(Number::I64(20))),
                        MathItem::Op(Operation::Div),
                        MathItem::Prim(Primitive::Number(Number::I64(30))),
                    ])
                }
            )
        );
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
        info!("{:?}", parser.ast());

        let AstNode::Function(decl) = &parser.ast().nodes[0] else {
            panic!()
        };
        let BlockValue::VariableDecl(val) = &decl.block[0].1 else {
            panic!();
        };
        let BlockValue::VariableReAssignment(val_assgn) = &decl.block[1].1 else {
            panic!();
        };
        // let BlockValue::Block(blok) = &decl.block[2].1 else {
        //     panic!();
        // };
        assert_eq!(
            val,
            &(
                "foo",
                Variable::from_value(
                    Value::Primitive(Primitive::Number(Number::I64(0))),
                    true,
                    None
                ),
            )
        );
        assert_eq!(
            val_assgn,
            &(
                "foo",
                VariableValue::Value(Value::Primitive(Primitive::Number(Number::I64(10))))
            )
        );
        // let BlockValue::VariableDecl(bar) = &blok.values[0] else {
        //     panic!();
        // };
        // info!(?val, ?val_assgn);
        // assert_eq!(
        //     bar,
        //     (
        //         "bar",
        //         Variable {
        //             typeid: TypeID::Primitive(PrimitiveID::I64),
        //             mutable: false,
        //             val: VariableValue::Name("foo")
        //         }
        //     )
        // );
        info!(?parser);
        Ok(())
    }

    #[test]
    fn parse_conditions() -> ParseResult<()> {
        setup_logger();
        let data = r#"
        struct Bar {
        bar: i64,
        }

        struct Foo {
        i: Bar, 
        }
        fn foo() {
        let foo = 10;
        if foo < 10 {
            return
        } else if foo == 10 {
            return
        } else {
            return
        }
        let bar = 20;
        let gar = bar;
        let foo = Foo {
        i: Bar {
        bar: 10
        },
        };

        let b = foo.i;
        }
            "#;

        let parser = Parser::parse(data).unwrap();

        match &parser.ast().nodes()[0] {
            AstNode::Function(decl) => {
                info!("{:?}", decl.block);
                // for (_, val) in &decl.block {
                //     info!(?val);
                // }
            }
            _ => panic!(),
        }
        Ok(())
    }

    #[test]
    fn array_test() {
        setup_logger();

        let data = r#"
        fn foo() {
        let f = [0, 1, 2, 3];
        }
            "#;

        let parser = Parser::parse(data).unwrap();

        info!(?parser);
    }

    #[test]
    fn complex_enum_array_test() {
        setup_logger();
        let data = r#"
    enum Bar {
        I,
        B,
        Z 
        }
    fn foo() {
        let f = [Bar::I, Bar::B, Bar::Z];
    }
    "#;

        let parser = Parser::parse(data).unwrap();

        info!(?parser);
    }

    #[test]
    fn complex_struct_array_test() {
        setup_logger();
        let data = r#"
    struct Foo {
        i: i64
        }
    fn foo() {
        let f = [Foo {i: 0}, Foo {i:10}, Foo{i:20}];
    }
    "#;

        let parser = Parser::parse(data).unwrap();

        info!(?parser);
    }
}

#![allow(unused)]

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{Debug, Display},
    io::Write,
    str::FromStr,
};

use tracing::{Subscriber, info, instrument};

use crate::{
    tokenizer::{Span, Token, Tokenizer},
    types::{
        self, Array, ComplexTypeDecl, ComplexTypeID, ComplexValue, Enum, EnumDecl, FunctionCall,
        FunctionDecl, Number, Primitive, PrimitiveID, StructDecl, TypeDecl, Typer, Value, Variable,
    },
};

#[derive(Debug)]
enum Operation {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
}

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
                    let (name, decl, unknown, i) =
                        TypeDecl::parse_ctx_mut(&mut typer, &iter[idx..])?;
                    if let Some(unknown) = unknown {
                        for item in unknown {
                            unknown_types.insert(item);
                        }
                    }
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

#[cfg(test)]
mod parseable {
    use std::{collections::HashMap, ops::ControlFlow};

    use tracing::{error, info};

    use crate::{
        parser::{AstNode, FunctionDecl, ParseError, ParseResult, Parser},
        tokenizer::{Span, Token, Tokenizer},
        types::{
            BlockValue, ComplexType, ComplexTypeDecl, ComplexTypeID, ComplexTypeName, ComplexValue,
            Enum, FunctionCall, Number, Primitive, PrimitiveID, Struct, StructDecl, TypeID, Typer,
            Value, Variable, Visibility,
        },
    };

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

        for (name, val) in &function.block {
            if *name == "foo" {
                assert_eq!(
                    val,
                    &BlockValue::VariableDecl(Variable {
                        typeid: PrimitiveID::I64.into(),
                        mutable: false,
                        val: crate::types::VariableValue::Value(Value::Primitive(
                            Primitive::Number(Number::I64(10))
                        )),
                    })
                );
            }
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

        for (name, val) in &function.block {
            if *name == "foo" {
                assert_eq!(
                    val,
                    &BlockValue::VariableDecl(Variable {
                        typeid: PrimitiveID::String.into(),
                        mutable: false,
                        val: crate::types::VariableValue::Value(Value::Primitive(
                            Primitive::String("foo_bar_baz")
                        )),
                    })
                );
            }
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
                (true, ("i", PrimitiveID::I32.into())),
                (false, ("bar", PrimitiveID::String.into())),
            ]),
            block: vec![],
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

        for val in &decl.block {
            match &val.1 {
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
}

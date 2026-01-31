#![allow(unused)]
use std::fmt::Display;

use tracing::{debug, error};

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Token<'a> {
    Let,
    WhiteSpace,
    Equal,
    Semicolon,
    LeftAngleBracket,
    RightAngleBracket,
    Colon,
    Comma,
    LeftBracket,
    RightBracket,
    Dot,
    // !
    Exclamation,

    Plus,
    Minus,
    Multiply,
    Divide,
    // should typeid be a type? introspection could be cool but a little useless :3
    TypeID(TypeID),
    Return,

    Pub,
    Mutable,

    // types
    Struct,
    Number(&'a str),
    Str(&'a str),
    QuotedString(&'a str),
    Enum,

    LeftParen,
    RightParen,

    Function,
    If,
    Else,

    #[default]
    Unknown,
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TypeID {
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
    QuotedString,
    /// for structs (? idkkkkkkk)
    Unknown,
    Array,
    Enum,
}

#[derive(Debug)]
pub enum TypeIDError {
    NotTypeID,
}

impl TryFrom<&str> for TypeID {
    type Error = TypeIDError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "i8" => Ok(TypeID::I8),
            "i16" => Ok(TypeID::I16),
            "i32" => Ok(TypeID::I32),
            "i64" => Ok(TypeID::I64),

            "u8" => Ok(TypeID::U8),
            "u16" => Ok(TypeID::U16),
            "U32" => Ok(TypeID::U32),
            "u64" => Ok(TypeID::U64),

            "String" => Ok(TypeID::String),

            _ => Err(TypeIDError::NotTypeID),
        }
    }
}

#[derive(Debug)]
enum Ident {
    Word(usize),
    Number(usize),
}

impl Ident {
    fn into_str<'a>(self, s: &'a str, idx: usize) -> Token<'a> {
        match self {
            Ident::Word(w) => {
                if let Ok(id) = TypeID::try_from(s[w..idx].trim()) {
                    Token::TypeID(id)
                } else {
                    Token::Str(s[w..idx].trim())
                }
            }
            Ident::Number(w) => Token::Number(s[w..idx].trim()),
        }
    }

    fn inner(&self) -> usize {
        match self {
            Ident::Word(n) => *n,
            Ident::Number(n) => *n,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    tokens: Vec<Token<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn tokens(&self) -> &[Token<'a>] {
        &self.tokens
    }
    /// works as an iterator, the number it returns is an increment amount, you can give it a big
    /// string and repeatedly call next() on it and just increment the start of your slice to get
    /// the next word
    fn next_token(source: &str) -> Option<(usize, Token<'_>, Option<Token<'_>>)> {
        // info!("src: {source:?}");

        let mut ident: Option<Ident> = None;
        let mut quoted = false;
        for (i, ch) in source.char_indices() {
            match ch {
                'A'..='Z' | 'a'..='z' => {
                    if ident.is_none() {
                        ident = Some(Ident::Word(i));
                    }
                }
                '+' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Plus)));
                    } else {
                        return Some((i + 1, Token::Plus, None));
                    }
                }
                '(' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::LeftParen)));
                    } else {
                        return Some((i + 1, Token::LeftParen, None));
                    }
                }
                ')' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::RightParen)));
                    } else {
                        return Some((i + 1, Token::RightParen, None));
                    }
                }
                '-' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Minus)));
                    } else {
                        return Some((i + 1, Token::Minus, None));
                    }
                }

                '*' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Multiply)));
                    } else {
                        return Some((i + 1, Token::Multiply, None));
                    }
                }

                '/' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Divide)));
                    } else {
                        return Some((i + 1, Token::Divide, None));
                    }
                }
                '.' => {
                    if let Some(ref iden) = ident
                        && let Ident::Number(_) = iden
                    {
                        continue;
                    }
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Dot)));
                    } else {
                        return Some((i + 1, Token::Dot, None));
                    }
                }

                '0'..='9' => {
                    if ident.is_none() {
                        ident = Some(Ident::Number(i));
                    }
                }
                ':' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Colon)));
                    } else {
                        return Some((i + 1, Token::Colon, None));
                    }
                }
                '!' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Exclamation)));
                    } else {
                        return Some((i + 1, Token::Exclamation, None));
                    }
                }

                ',' => {
                    if let Some(iden) = ident {
                        // i + 2 to skip over the string and then the comma
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Comma)));
                    } else {
                        return Some((i + 1, Token::Comma, None));
                    }
                }

                ' ' => {
                    if ident.is_none() {
                        return Some((i + 1, Token::WhiteSpace, None));
                    }
                    if let Some(iden) = ident {
                        debug!(
                            "found {ch:?}: {:?}",
                            (i, Token::Str(source[iden.inner()..i + 1].trim()),)
                        );

                        return Some((i + 1, iden.into_str(source, i), Some(Token::WhiteSpace)));
                    } else {
                        return Some((i + 1, Token::WhiteSpace, None));
                    }
                }
                '=' => {
                    if let Some(iden) = ident {
                        debug!("found {ch:?} {:?}", (i, source[iden.inner()..i].trim()));
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Equal)));
                    }
                    return Some((i + 1, Token::Equal, None));
                }
                '[' => {
                    if let Some(iden) = ident {
                        debug!("found {ch:?} {:?}", (i, source[iden.inner()..i].trim()));
                        return Some((i, iden.into_str(source, i), None));
                    } else {
                        return Some((i + 1, Token::LeftBracket, None));
                    }
                }
                ']' => {
                    if let Some(iden) = ident {
                        return Some((i + 1, iden.into_str(source, i), Some(Token::RightBracket)));
                    }
                    return Some((i + 2, Token::RightBracket, None));
                }
                '{' => {
                    if let Some(iden) = ident {
                        return Some((
                            i + 1,
                            iden.into_str(source, i),
                            Some(Token::LeftAngleBracket),
                        ));
                    }
                    return Some((i + 1, Token::LeftAngleBracket, None));
                }
                '}' => {
                    if let Some(iden) = ident {
                        return Some((
                            i + 1,
                            iden.into_str(source, i),
                            Some(Token::RightAngleBracket),
                        ));
                    }
                    return Some((i + 2, Token::RightAngleBracket, None));
                }

                ';' => {
                    if let Some(iden) = ident {
                        debug!(
                            "found {ch:?} {:?}",
                            (
                                i,
                                Token::Str(source[iden.inner()..i.saturating_add(1)].trim()),
                            )
                        );
                        return Some((i + 1, iden.into_str(source, i), Some(Token::Semicolon)));
                    } else {
                        return Some((i + 1, Token::Semicolon, None));
                    }
                }
                '"' => match quoted {
                    false => {
                        quoted = true;
                        ident = Some(Ident::Word(i));
                    }
                    true => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Word(w) => {
                                    return Some((
                                        i + 1,
                                        Token::QuotedString(&source[w + 1..i]),
                                        None,
                                    ));
                                }
                                Ident::Number(_) => panic!(),
                            }
                        }
                    }
                },
                '\n' => continue,

                tok => {
                    error!("error: found {tok:?}");
                    return Some((i, Token::Unknown, None));
                }
            }
        }

        if let Some(iden) = ident {
            return Some((source.len(), iden.into_str(source, source.len()), None));
        }

        None
    }

    pub fn tokenize(data: &'a str) -> Self {
        debug!("{data:?}");

        let mut i = 0;
        let mut prev = Token::Unknown;
        let mut tokens: Vec<Token> = vec![];

        loop {
            if i >= data.len() {
                break;
            }
            if let Some((inc, wrd, peek)) = Tokenizer::next_token(&data[i..]) {
                // info!("found {prev} {wrd} {peek:?}");
                if wrd == Token::Unknown {
                    panic!("found unknown token");
                }

                if prev == Token::WhiteSpace && wrd == Token::WhiteSpace {
                    i += 1;
                    continue;
                }
                if let Token::Str(s) = wrd {
                    let t = match s {
                        "let" => Token::Let,
                        "struct" => Token::Struct,
                        "enum" => Token::Enum,
                        "return" => Token::Return,
                        "mut" => Token::Mutable,
                        "pub" => Token::Pub,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "fn" => Token::Function,
                        _ => wrd,
                    };

                    tokens.push(t);
                } else {
                    tokens.push(wrd);
                }
                // info!("word: {wrd:?} {peek:?}");
                if let Some(peek) = peek {
                    tokens.push(peek);
                }
                prev = wrd;
                i += inc;
            }
        }

        Self { tokens }
    }
}

#[cfg(test)]
mod token {
    use tracing::info;

    use crate::tokenizer::{Token, Tokenizer, TypeID};

    fn setup_logger() {
        let _guard =
            tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new());
    }

    #[test]
    fn let_test() {
        setup_logger();

        let data = "let ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;
        assert_eq!([Token::Let, Token::WhiteSpace], tokens.as_slice());

        info!("{tokenizer:?}");
    }

    #[test]
    fn let_var_string() {
        setup_logger();

        let data = "let foo=\"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::QuotedString("bar")
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_string_whitespace() {
        setup_logger();

        let data = "let foo = \"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::QuotedString("bar")
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_num() {
        setup_logger();

        let data = "let foo=10";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Number("10"),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_num_whitespace() {
        setup_logger();

        let data = "let foo = 10";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::Number("10"),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_float() {
        setup_logger();

        let data = "let foo=10.0";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;
        info!("{tokens:?}");

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Number("10.0"),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_float_whitespace() {
        setup_logger();

        let data = "let foo = 10.0";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::Number("10.0"),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_struct() {
        setup_logger();

        let data = "let foo=Foo{baz=0;}";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!("{:?}", tokenizer.tokens);

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Str("Foo"),
                Token::LeftAngleBracket,
                Token::Str("baz"),
                Token::Equal,
                Token::Number("0"),
                Token::Semicolon,
                Token::RightAngleBracket,
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_struct_whitespace() {
        setup_logger();

        let data = "let foo = Foo {
        baz = 0;
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::WhiteSpace,
                Token::LeftAngleBracket,
                Token::WhiteSpace,
                Token::Str("baz"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::Number("0"),
                Token::Semicolon,
                Token::WhiteSpace,
                Token::RightAngleBracket,
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_struct_decl() {
        setup_logger();

        let data = "struct Foo{bar:i32,baz:i32,}";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;
        info!("{tokens:?}");

        assert_eq!(
            [
                Token::Struct,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::LeftAngleBracket,
                Token::Str("bar"),
                Token::Colon,
                Token::TypeID(TypeID::I32),
                Token::Comma,
                Token::Str("baz"),
                Token::Colon,
                Token::TypeID(TypeID::I32),
                Token::Comma,
                Token::RightAngleBracket
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_struct_decl_whitespace() {
        setup_logger();

        let data = "struct Foo {
        bar: i32,
        baz:i32,
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Struct,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::WhiteSpace,
                Token::LeftAngleBracket,
                Token::WhiteSpace,
                Token::Str("bar"),
                Token::Colon,
                Token::WhiteSpace,
                Token::TypeID(TypeID::I32),
                Token::Comma,
                Token::WhiteSpace,
                Token::Str("baz"),
                Token::Colon,
                Token::TypeID(TypeID::I32),
                Token::Comma,
                Token::WhiteSpace,
                Token::RightAngleBracket
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_enum_decl() {
        setup_logger();

        let data = "enum Foo{Bar,Baz,}";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Enum,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::LeftAngleBracket,
                Token::Str("Bar"),
                Token::Comma,
                Token::Str("Baz"),
                Token::Comma,
                Token::RightAngleBracket
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_enum_decl_whitespace() {
        setup_logger();

        let data = "enum Foo {
        Bar,
        Baz,
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                Token::Enum,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::WhiteSpace,
                Token::LeftAngleBracket,
                Token::WhiteSpace,
                Token::Str("Bar"),
                Token::Comma,
                Token::WhiteSpace,
                Token::Str("Baz"),
                Token::Comma,
                Token::WhiteSpace,
                Token::RightAngleBracket
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_enum() {
        setup_logger();

        let data = "let foo=Foo::Urmom;";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!("{:?}", tokenizer.tokens);

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Str("Foo"),
                Token::Colon,
                Token::Colon,
                Token::Str("Urmom"),
                Token::Semicolon
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_enum_whitespace() {
        setup_logger();

        let data = "let foo = Foo::Urmom;";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!("{:?}", tokenizer.tokens);

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::WhiteSpace,
                Token::Equal,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::Colon,
                Token::Colon,
                Token::Str("Urmom"),
                Token::Semicolon
            ],
            tokens.as_slice()
        );
    }
    #[test]
    fn let_var_dot() {
        setup_logger();

        let data = "foo.bar";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        assert_eq!(
            [Token::Str("foo"), Token::Dot, Token::Str("bar")],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_dot_white_space() {
        setup_logger();

        let data = " foo.bar ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        assert_eq!(
            [
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Dot,
                Token::Str("bar"),
                Token::WhiteSpace
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn exclamation_mark() {
        setup_logger();

        let data = "!";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        assert_eq!([Token::Exclamation], tokens.as_slice());
    }

    #[test]
    fn exclamation_mark_whitespace() {
        setup_logger();

        let data = " ! ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [Token::WhiteSpace, Token::Exclamation, Token::WhiteSpace],
            tokens.as_slice()
        );
    }

    #[cfg(test)]
    mod ops {
        use crate::tokenizer::{Token, Tokenizer, token::setup_logger};

        #[test]
        fn add() {
            setup_logger();
            let data = "foo+bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Plus, Token::Str("bar")],
                tokens.as_slice()
            );
        }

        #[test]
        fn add_whitespace() {
            setup_logger();
            let data = "foo+ ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Plus, Token::WhiteSpace],
                tokens.as_slice()
            );
        }

        #[test]
        fn sub() {
            setup_logger();
            let data = "foo-bar";
            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Minus, Token::Str("bar")],
                tokens.as_slice()
            );
        }

        #[test]
        fn sub_whitespace() {
            setup_logger();
            let data = "foo- ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Minus, Token::WhiteSpace],
                tokens.as_slice()
            );
        }

        #[test]
        fn mult() {
            setup_logger();
            let data = "foo*bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Multiply, Token::Str("bar")],
                tokens.as_slice()
            );
        }

        #[test]
        fn mult_whitespace() {
            setup_logger();
            let data = "foo* ";

            let tokenizer = Tokenizer::tokenize(data);

            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Multiply, Token::WhiteSpace],
                tokens.as_slice()
            );
        }

        #[test]
        fn div() {
            setup_logger();
            let data = "foo/bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Divide, Token::Str("bar")],
                tokens.as_slice()
            );
        }

        #[test]
        fn div_whitespace() {
            setup_logger();
            let data = "foo/ ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [Token::Str("foo"), Token::Divide, Token::WhiteSpace],
                tokens.as_slice()
            );
        }
    }
}

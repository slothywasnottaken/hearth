use std::fmt::Display;

use tracing::{debug, error, info, instrument, trace};

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Token<'a> {
    Let,
    Equal,
    Semicolon,
    LeftAngleBracket,
    RightAngleBracket,
    LeftCarrot,
    RightCarrot,
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

    True,
    False,

    #[default]
    Unknown,
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[allow(unused)]
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

    Bool,

    String,
    QuotedString,
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
            "u32" => Ok(TypeID::U32),
            "u64" => Ok(TypeID::U64),

            "string" => Ok(TypeID::String),
            "bool" => Ok(TypeID::Bool),

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
    #[instrument]
    fn into_str<'a>(self, s: &'a str, idx: usize) -> Token<'a> {
        match self {
            Ident::Word(w) => {
                if let Ok(id) = TypeID::try_from(&s[w..idx]) {
                    Token::TypeID(id)
                } else {
                    Token::Str(&s[w..idx])
                }
            }
            Ident::Number(w) => Token::Number(&s[w..idx]),
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

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    tokens: Vec<(Span, Token<'a>)>,
}

#[derive(Debug)]
pub struct TokenIterator<'a> {
    data: &'a str,
    idx: usize,
}

impl<'a> TokenIterator<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { data, idx: 0 }
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = (Span, Token<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.data.len() {
            return None;
        }

        let (idx, tok) = Tokenizer::next_token(&self.data[self.idx..])?;

        // info!("looking at {:?}", &self.data[self.idx..self.idx + idx]);
        let mut span = Span {
            start: self.idx + idx.start,
            end: self.idx + idx.end,
        };

        if let Token::QuotedString(_) = tok {
            span.end -= 1;
        }

        // info!(?idx, ?span);

        self.idx += idx.end;

        Some((span, tok))
    }
}

impl<'a> Tokenizer<'a> {
    pub fn tokenize(data: &'a str) -> Self {
        debug!("{data:?}");

        let mut prev = Token::Unknown;
        let mut tokens: Vec<(Span, Token)> = vec![];
        let iter = TokenIterator::new(data);

        for (span, wrd) in iter {
            trace!("found {prev} {wrd}");
            if wrd == Token::Unknown {
                panic!("found unknown token");
            }

            if let Token::Str(s) = wrd {
                let (new_span, t) = match s {
                    "let" => (Span::new(span.start, span.end), Token::Let),
                    "struct" => (Span::new(span.start, span.end), Token::Struct),
                    "enum" => (Span::new(span.start, span.end), Token::Enum),
                    "return" => (Span::new(span.start, span.end), Token::Return),
                    "mut" => (Span::new(span.start, span.end), Token::Mutable),
                    "pub" => (Span::new(span.start, span.end), Token::Pub),
                    "if" => (Span::new(span.start, span.end), Token::If),
                    "else" => (Span::new(span.start, span.end), Token::Else),
                    "fn" => (Span::new(span.start, span.end), Token::Function),
                    "true" => (Span::new(span.start, span.end), Token::True),
                    "false" => (Span::new(span.start, span.end), Token::False),
                    _ => (Span::new(span.start, span.end), wrd),
                };

                info!(?new_span, ?t);
                tokens.push((new_span, t));
            } else {
                tokens.push((span, wrd));
            }
            prev = wrd;
        }

        Self { tokens }
    }

    /// works as an iterator, the number it returns is an increment amount, you can give it a big
    /// string and repeatedly call next() on it and just increment the start of your slice to get
    /// the next word
    #[instrument(skip_all, ret)]
    fn next_token(source: &str) -> Option<(Span, Token<'_>)> {
        let mut ident: Option<Ident> = None;
        let mut quoted = false;
        for (i, ch) in source.char_indices() {
            match ch {
                'A'..='Z' | 'a'..='z' => {
                    if ident.is_none() {
                        info!(?i, ?ch);
                        ident = Some(Ident::Word(i));
                    }
                }
                '0'..='9' => {
                    if ident.is_none() {
                        ident = Some(Ident::Number(i));
                    }
                }

                ' ' => {
                    if let Some(iden) = ident {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                }

                '=' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Equal)),
                },
                '!' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Exclamation)),
                },
                '{' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::LeftAngleBracket)),
                },

                '}' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::RightAngleBracket)),
                },

                '(' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::LeftParen)),
                },
                ')' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::RightParen)),
                },

                ';' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Semicolon)),
                },

                ':' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Colon)),
                },

                ',' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Comma)),
                },

                '+' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Plus)),
                },
                '-' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Minus)),
                },
                '*' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Multiply)),
                },
                '/' => match ident {
                    Some(iden) => {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                    None => return Some((Span::new(i, i + 1), Token::Divide)),
                },

                '\n' => {
                    if let Some(iden) = ident {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
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
                                        Span::new(w + 1, i + 1),
                                        Token::QuotedString(&source[w + 1..i]),
                                    ));
                                }
                                Ident::Number(_) => panic!(),
                            }
                        }
                    }
                },

                tok => {
                    if ident.is_none() {
                        error!("error: found {tok:?}");
                        return Some((Span::new(i, i), Token::Unknown));
                    }
                }
            }
        }

        if let Some(iden) = ident {
            return Some((
                Span::new(iden.inner(), source.len()),
                iden.into_str(source, source.len()),
            ));
        }

        None
    }

    pub fn tokens(&self) -> &[(Span, Token<'a>)] {
        &self.tokens
    }
}

#[cfg(test)]
mod token {
    use tracing::info;

    use crate::tokenizer::{Span, Token, Tokenizer};

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

        for (span, _) in tokens {
            info!("source index {:?}", &data[span.start..span.end]);
        }

        assert_eq!([(Span::new(0, 3), Token::Let)], tokens.as_slice());
    }

    #[test]
    fn let_var_string() {
        setup_logger();

        let data = "let foo=\"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;
        for (span, _) in tokens {
            info!("source index {:?}", &data[span.start..span.end]);
        }

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(7, 8), Token::Equal),
                (Span::new(9, 12), Token::QuotedString("bar"))
            ],
            tokens.as_slice()
        );
    }

    #[ignore]
    fn let_var_string_whitespace() {
        setup_logger();

        let data = "let foo = \"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 2), Token::Let),
                (Span::new(4, 6), Token::Str("foo")),
                (Span::new(7, 7), Token::Equal),
                (Span::new(10, 12), Token::QuotedString("bar"))
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
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(7, 8), Token::Equal),
                (Span::new(8, 10), Token::Number("10")),
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

        for (span, _) in tokens {
            info!("source index {:?}", &data[span.start..span.end]);
        }

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(8, 9), Token::Equal),
                (Span::new(10, 12), Token::Number("10")),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_float() {
        setup_logger();

        let data = "let foo = 10.0";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);

        for (span, _) in tokens {
            info!("source index {:?}", &data[span.start..span.end]);
        }

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(8, 9), Token::Equal),
                (Span::new(10, 14), Token::Number("10.0")),
            ],
            tokens.as_slice()
        );
    }

    #[ignore]
    fn let_var_float_whitespace() {
        setup_logger();

        let data = "let foo = 10.0";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 2), Token::Let),
                (Span::new(4, 6), Token::Str("foo")),
                (Span::new(7, 7), Token::Equal),
                (Span::new(8, 11), Token::Number("10.0")),
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

        info!(?tokens);

        // assert_eq!(
        //     [
        //         Token::Let,
        //         Token::Str("foo"),
        //         Token::Equal,
        //         Token::Str("Foo"),
        //         Token::LeftAngleBracket,
        //         Token::Str("baz"),
        //         Token::Equal,
        //         Token::Number("0"),
        //         Token::Semicolon,
        //         Token::RightAngleBracket,
        //     ],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn let_var_struct_whitespace() {
        setup_logger();

        let data = "let foo = Foo {
        baz = 0;
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);
        // assert_eq!(
        //     [
        //         Token::Let,
        //         Token::Str("foo"),
        //         Token::Equal,
        //         Token::Str("Foo"),
        //         Token::LeftAngleBracket,
        //         Token::Str("baz"),
        //         Token::Equal,
        //         Token::Number("0"),
        //         Token::Semicolon,
        //         Token::RightAngleBracket,
        //     ],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn let_struct_decl() {
        setup_logger();

        let data = "struct Foo{bar:i32,baz:i32,}";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);
        //     assert_eq!(
        //         [
        //             Token::Struct,
        //             Token::Str("Foo"),
        //             Token::LeftAngleBracket,
        //             Token::Str("bar"),
        //             Token::Colon,
        //             Token::TypeID(TypeID::I32),
        //             Token::Comma,
        //             Token::Str("baz"),
        //             Token::Colon,
        //             Token::TypeID(TypeID::I32),
        //             Token::Comma,
        //             Token::RightAngleBracket
        //         ],
        //         tokens.as_slice()
        //     );
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

        info!(?tokens);
        // assert_eq!(
        //     [
        //         Token::Struct,
        //         Token::Str("Foo"),
        //         Token::LeftAngleBracket,
        //         Token::Str("bar"),
        //         Token::Colon,
        //         Token::TypeID(TypeID::I32),
        //         Token::Comma,
        //         Token::Str("baz"),
        //         Token::Colon,
        //         Token::TypeID(TypeID::I32),
        //         Token::Comma,
        //         Token::RightAngleBracket
        //     ],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn let_enum_decl() {
        setup_logger();

        let data = "enum Foo{Bar,Baz,}";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);
        // assert_eq!(
        //     [
        //         Token::Enum,
        //         Token::Str("Foo"),
        //         Token::LeftAngleBracket,
        //         Token::Str("Bar"),
        //         Token::Comma,
        //         Token::Str("Baz"),
        //         Token::Comma,
        //         Token::RightAngleBracket
        //     ],
        //     tokens.as_slice()
        // );
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

        info!(?tokens);
        // assert_eq!(
        //     [
        //         Token::Enum,
        //         Token::Str("Foo"),
        //         Token::LeftAngleBracket,
        //         Token::Str("Bar"),
        //         Token::Comma,
        //         Token::Str("Baz"),
        //         Token::Comma,
        //         Token::RightAngleBracket
        //     ],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn let_var_enum() {
        setup_logger();

        let data = "let foo=Foo::Urmom;";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;
        info!(?tokens);

        //     assert_eq!(
        //         [
        //             Token::Let,
        //             Token::Str("foo"),
        //             Token::Equal,
        //             Token::Str("Foo"),
        //             Token::Colon,
        //             Token::Colon,
        //             Token::Str("Urmom"),
        //             Token::Semicolon
        //         ],
        //         tokens.as_slice()
        //     );
    }

    #[test]
    fn let_var_enum_whitespace() {
        setup_logger();

        let data = "let foo = Foo::Urmom;";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);
        //     assert_eq!(
        //         [
        //             Token::Let,
        //             Token::Str("foo"),
        //             Token::Equal,
        //             Token::Str("Foo"),
        //             Token::Colon,
        //             Token::Colon,
        //             Token::Str("Urmom"),
        //             Token::Semicolon
        //         ],
        //         tokens.as_slice()
        //     );
    }

    #[test]
    fn let_var_dot() {
        setup_logger();

        let data = "foo.bar";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        info!(?tokens);
        // assert_eq!(
        //     [Token::Str("foo"), Token::Dot, Token::Str("bar")],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn let_var_dot_white_space() {
        setup_logger();

        let data = " foo.bar ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        info!(?tokens);
        // assert_eq!(
        //     [Token::Str("foo"), Token::Dot, Token::Str("bar"),],
        //     tokens.as_slice()
        // );
    }

    #[test]
    fn exclamation_mark() {
        setup_logger();

        let data = "!";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        info!(?tokens);
        // assert_eq!([Token::Exclamation], tokens.as_slice());
    }

    #[test]
    fn exclamation_mark_whitespace() {
        setup_logger();

        let data = " ! ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        info!(?tokens);
        // assert_eq!([Token::Exclamation], tokens.as_slice());
    }

    #[cfg(test)]
    mod ops {
        use tracing::info;

        use crate::tokenizer::{Token, Tokenizer, token::setup_logger};

        #[test]
        fn add() {
            setup_logger();
            let data = "foo+bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!(
            //     [Token::Str("foo"), Token::Plus, Token::Str("bar")],
            //     tokens.as_slice()
            // );
        }

        #[test]
        fn add_whitespace() {
            setup_logger();
            let data = "foo+ ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!([Token::Str("foo"), Token::Plus,], tokens.as_slice());
        }

        #[test]
        fn sub() {
            setup_logger();
            let data = "foo-bar";
            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!(
            //     [Token::Str("foo"), Token::Minus, Token::Str("bar")],
            //     tokens.as_slice()
            // );
        }

        #[test]
        fn sub_whitespace() {
            setup_logger();
            let data = "foo- ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!([Token::Str("foo"), Token::Minus,], tokens.as_slice());
        }

        #[test]
        fn mult() {
            setup_logger();
            let data = "foo*bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!(
            //     [Token::Str("foo"), Token::Multiply, Token::Str("bar")],
            //     tokens.as_slice()
            // );
        }

        #[test]
        fn mult_whitespace() {
            setup_logger();
            let data = "foo* ";

            let tokenizer = Tokenizer::tokenize(data);

            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!([Token::Str("foo"), Token::Multiply], tokens.as_slice());
        }

        #[test]
        fn div() {
            setup_logger();
            let data = "foo/bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            info!(?tokens);
            // assert_eq!(
            //     [Token::Str("foo"), Token::Divide, Token::Str("bar")],
            //     tokens.as_slice()
            // );
        }

        #[test]
        fn div_whitespace() {
            setup_logger();
            let data = "foo/ ";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;
            info!(?tokens);

            // assert_eq!([Token::Str("foo"), Token::Divide], tokens.as_slice());
        }
    }
}

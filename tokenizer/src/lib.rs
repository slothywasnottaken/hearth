#![allow(clippy::match_bool)]
use std::fmt::Display;

use tracing::{error, trace};

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

    For,
    Switch,
    Loop,

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

#[derive(Debug, Clone, Copy)]
enum Ident {
    Word(usize),
    Number(usize),
}

impl Ident {
    fn into_str(self, s: &'_ str, idx: usize) -> Token<'_> {
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
            Ident::Word(n) | Ident::Number(n) => *n,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
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
    #[must_use]
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

        let mut span = Span {
            start: self.idx + idx.start,
            end: self.idx + idx.end,
        };

        if let Token::QuotedString(_) = tok {
            span.end -= 1;
        }

        self.idx += idx.end;

        Some((span, tok))
    }
}

impl<'a> Tokenizer<'a> {
    /// # Panics
    /// if there is an unknown token
    pub fn tokenize(data: &'a str) -> Self {
        trace!("{data:?}");

        let mut prev = Token::Unknown;
        let mut tokens: Vec<(Span, Token)> = vec![];
        let iter = TokenIterator::new(data);

        for (span, wrd) in iter {
            trace!("found {prev} {wrd}");

            assert!(wrd != Token::Unknown, "found unknown token");

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
                    "true" => Token::True,
                    "false" => Token::False,
                    "for" => Token::For,
                    "switch" => Token::Switch,
                    "loop" => Token::Loop,
                    _ => wrd,
                };

                trace!(?span, ?t);
                tokens.push((span, t));
            } else {
                tokens.push((span, wrd));
            }
            prev = wrd;
        }

        Self { tokens }
    }

    fn return_ident_or_token(
        ident: Option<Ident>,
        source: &'a str,
        i: usize,
        ch: char,
    ) -> (Span, Token<'a>) {
        if let Some(iden) = ident {
            return (Span::new(iden.inner(), i), iden.into_str(source, i));
        }

        (
            Span::new(i, i + 1),
            match ch {
                '=' => Token::Equal,
                '!' => Token::Exclamation,
                '{' => Token::LeftAngleBracket,
                '}' => Token::RightAngleBracket,
                '(' => Token::LeftParen,
                ')' => Token::RightParen,
                ';' => Token::Semicolon,
                ':' => Token::Colon,
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Multiply,
                '/' => Token::Divide,
                '[' => Token::LeftBracket,
                ']' => Token::RightBracket,
                '<' => Token::LeftCarrot,
                '>' => Token::RightCarrot,
                ',' => Token::Comma,

                t => {
                    error!("found unknown {t:?}");
                    Token::Unknown
                }
            },
        )
    }

    /// works as an iterator, the number it returns is an increment amount, you can give it a big
    /// string and repeatedly call `next()` on it and just increment the start of your slice to get
    /// the next word
    fn next_token(source: &'a str) -> Option<(Span, Token<'a>)> {
        let mut ident: Option<Ident> = None;
        let mut quoted = false;
        for (i, ch) in source.char_indices() {
            match ch {
                'A'..='Z' | 'a'..='z' => {
                    if ident.is_none() {
                        trace!(?i, ?ch);
                        ident = Some(Ident::Word(i));
                    }
                }
                '0'..='9' => {
                    if ident.is_none() {
                        ident = Some(Ident::Number(i));
                    }
                }

                ' ' | '\n' => {
                    if let Some(iden) = ident {
                        return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                    }
                }

                '.' => match ident {
                    Some(iden) => match iden {
                        // if number dont return here because if its 10.0 it would return 10
                        Ident::Number(_n) => {}
                        Ident::Word(_w) => {
                            return Some((Span::new(iden.inner(), i), iden.into_str(source, i)));
                        }
                    },
                    None => return Some((Span::new(i, i + 1), Token::Dot)),
                },

                '_' => {}

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

                tok => return Some(Self::return_ident_or_token(ident, source, i, tok)),
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

    #[must_use]
    pub fn tokens(&self) -> &[(Span, Token<'a>)] {
        &self.tokens
    }
}

#[cfg(test)]
mod token {
    use crate::{Span, Token, Tokenizer, TypeID};

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

        assert_eq!([(Span::new(0, 3), Token::Let)], tokens.as_slice());
    }

    #[test]
    fn let_var_string() {
        setup_logger();

        let data = "let foo=\"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

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

    #[test]
    fn let_var_string_whitespace() {
        setup_logger();

        let data = "let foo = \"bar\"";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(8, 9), Token::Equal),
                (Span::new(11, 14), Token::QuotedString("bar"))
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

    #[test]
    fn let_var_float_whitespace() {
        setup_logger();

        let data = "let foo = 10.0";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

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

    #[test]
    fn let_var_struct() {
        setup_logger();

        let data = "let foo=Foo{baz=0,};";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(7, 8), Token::Equal),
                (Span::new(8, 11), Token::Str("Foo")),
                (Span::new(11, 12), Token::LeftAngleBracket),
                (Span::new(12, 15), Token::Str("baz")),
                (Span::new(15, 16), Token::Equal),
                (Span::new(16, 17), Token::Number("0")),
                (Span::new(17, 18), Token::Comma),
                (Span::new(18, 19), Token::RightAngleBracket),
                (Span::new(19, 20), Token::Semicolon),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_var_struct_whitespace() {
        setup_logger();

        let data = "let foo = Foo {
        baz = 0,
        };";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(8, 9), Token::Equal),
                (Span::new(10, 13), Token::Str("Foo")),
                (Span::new(14, 15), Token::LeftAngleBracket),
                (Span::new(24, 27), Token::Str("baz")),
                (Span::new(28, 29), Token::Equal),
                (Span::new(30, 31), Token::Number("0")),
                (Span::new(31, 32), Token::Comma),
                (Span::new(41, 42), Token::RightAngleBracket),
                (Span::new(42, 43), Token::Semicolon),
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

        assert_eq!(
            [
                (Span::new(0, 6), Token::Struct),
                (Span::new(7, 10), Token::Str("Foo")),
                (Span::new(10, 11), Token::LeftAngleBracket),
                (Span::new(11, 14), Token::Str("bar")),
                (Span::new(14, 15), Token::Colon),
                (Span::new(15, 18), Token::TypeID(TypeID::I32)),
                (Span::new(18, 19), Token::Comma),
                (Span::new(19, 22), Token::Str("baz")),
                (Span::new(22, 23), Token::Colon),
                (Span::new(23, 26), Token::TypeID(TypeID::I32)),
                (Span::new(26, 27), Token::Comma),
                (Span::new(27, 28), Token::RightAngleBracket),
            ],
            tokens.as_slice()
        );
    }

    #[test]
    fn let_struct_decl_whitespace() {
        setup_logger();

        let data = "struct Foo {
        bar: i32,
        baz: i32,
        }";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!(
            [
                (Span::new(0, 6), Token::Struct),
                (Span::new(7, 10), Token::Str("Foo")),
                (Span::new(11, 12), Token::LeftAngleBracket),
                (Span::new(21, 24), Token::Str("bar")),
                (Span::new(24, 25), Token::Colon),
                (Span::new(26, 29), Token::TypeID(TypeID::I32)),
                (Span::new(29, 30), Token::Comma),
                (Span::new(39, 42), Token::Str("baz")),
                (Span::new(42, 43), Token::Colon),
                (Span::new(44, 47), Token::TypeID(TypeID::I32)),
                (Span::new(47, 48), Token::Comma),
                (Span::new(57, 58), Token::RightAngleBracket),
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
                (Span::new(0, 4), Token::Enum),
                (Span::new(5, 8), Token::Str("Foo")),
                (Span::new(8, 9), Token::LeftAngleBracket),
                (Span::new(9, 12), Token::Str("Bar")),
                (Span::new(12, 13), Token::Comma),
                (Span::new(13, 16), Token::Str("Baz")),
                (Span::new(16, 17), Token::Comma),
                (Span::new(17, 18), Token::RightAngleBracket)
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
                (Span::new(0, 4), Token::Enum),
                (Span::new(5, 8), Token::Str("Foo")),
                (Span::new(9, 10), Token::LeftAngleBracket),
                (Span::new(19, 22), Token::Str("Bar")),
                (Span::new(22, 23), Token::Comma),
                (Span::new(32, 35), Token::Str("Baz")),
                (Span::new(35, 36), Token::Comma),
                (Span::new(45, 46), Token::RightAngleBracket)
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
        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(7, 8), Token::Equal),
                (Span::new(8, 11), Token::Str("Foo")),
                (Span::new(11, 12), Token::Colon),
                (Span::new(12, 13), Token::Colon),
                (Span::new(13, 18), Token::Str("Urmom")),
                (Span::new(18, 19), Token::Semicolon),
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

        assert_eq!(
            [
                (Span::new(0, 3), Token::Let),
                (Span::new(4, 7), Token::Str("foo")),
                (Span::new(8, 9), Token::Equal),
                (Span::new(10, 13), Token::Str("Foo")),
                (Span::new(13, 14), Token::Colon),
                (Span::new(14, 15), Token::Colon),
                (Span::new(15, 20), Token::Str("Urmom")),
                (Span::new(20, 21), Token::Semicolon),
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
            [
                (Span::new(0, 3), Token::Str("foo")),
                (Span::new(3, 4), Token::Dot),
                (Span::new(4, 7), Token::Str("bar"))
            ],
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
                (Span::new(1, 4), Token::Str("foo")),
                (Span::new(4, 5), Token::Dot),
                (Span::new(5, 8), Token::Str("bar"))
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

        assert_eq!([(Span::new(0, 1), Token::Exclamation)], tokens.as_slice());
    }

    #[test]
    fn exclamation_mark_whitespace() {
        setup_logger();

        let data = " ! ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!([(Span::new(1, 2), Token::Exclamation)], tokens.as_slice());
    }

    #[test]
    fn left_carrot() {
        setup_logger();

        let data = "<";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        assert_eq!([(Span::new(0, 1), Token::LeftCarrot)], tokens.as_slice());
    }

    #[test]
    fn left_carrot_whitespace() {
        setup_logger();

        let data = " < ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!([(Span::new(1, 2), Token::LeftCarrot)], tokens.as_slice());
    }

    #[test]
    fn right_carrot() {
        setup_logger();

        let data = ">";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens;

        assert_eq!([(Span::new(0, 1), Token::RightCarrot)], tokens.as_slice());
    }

    #[test]
    fn right_carrot_whitespace() {
        setup_logger();

        let data = " > ";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = &tokenizer.tokens;

        assert_eq!([(Span::new(1, 2), Token::RightCarrot)], tokens.as_slice());
    }

    #[test]
    fn r#for() {
        let data = "for";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens();

        assert_eq!([(Span::new(0, 3), Token::For)], tokens);
    }

    #[test]
    fn switch() {
        let data = "switch";

        let tokenizer = Tokenizer::tokenize(data);
        let tokens = tokenizer.tokens();

        assert_eq!([(Span::new(0, data.len()), Token::Switch)], tokens);
    }

    #[cfg(test)]
    mod ops {
        use crate::{Span, Token, Tokenizer, token::setup_logger};

        #[test]
        fn add() {
            setup_logger();
            let data = "foo+bar";

            let tokenizer = Tokenizer::tokenize(data);
            let tokens = tokenizer.tokens;

            assert_eq!(
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Plus),
                    (Span::new(4, 7), Token::Str("bar"))
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Plus)
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Minus),
                    (Span::new(4, 7), Token::Str("bar"))
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Minus)
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Multiply),
                    (Span::new(4, 7), Token::Str("bar"))
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Multiply)
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Divide),
                    (Span::new(4, 7), Token::Str("bar"))
                ],
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
                [
                    (Span::new(0, 3), Token::Str("foo")),
                    (Span::new(3, 4), Token::Divide)
                ],
                tokens.as_slice()
            );
        }
    }
}

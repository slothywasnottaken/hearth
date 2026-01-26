use std::fmt::Display;

#[macro_export]
macro_rules! function_name {
    () => {{
        // Okay, this is ugly, I get it. However, this is the best we can get on a stable rust.
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            let name = std::any::type_name::<T>();
            if let Some(stripped) = name.strip_prefix(module_path!()) {
                // cursed way of skipping the following :: from module_path!
                return &stripped[2..];
            }
            name
        }
        let name = type_name_of(f);
        // `3` is the length of the `::f`.
        &name[..name.len() - 3]
    }};
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        log::trace!("[{}] {}", function_name!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log::debug!("[{}] {}", function_name!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log::info!("[{}] {}", function_name!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log::warn!("[{}] {}", function_name!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log::warn!("[{}] {}", function_name!(), format_args!($($arg)*));
    };
}

#[derive(Debug, PartialEq, Clone, Copy)]
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

    Plus,
    Minus,
    Multiply,
    Divide,
    // should typeid be a type? introspection could be cool but a little useless :3
    TypeID(TypeID),
    Return,

    Pub,

    // types
    Struct,
    Array,
    Number(&'a str),
    Str(&'a str),
    QuotedString(&'a str),
    Float,
    Enum,

    Function,
    Module,
    If,
    Else,
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

    String,
    QuotedString,
    /// for structs (? idkkkkkkk)
    Unknown,
    Array,
}

impl From<&str> for TypeID {
    fn from(value: &str) -> Self {
        match value {
            "i8" => TypeID::I8,
            "i16" => TypeID::I16,
            "i32" => TypeID::I32,
            "i64" => TypeID::I64,

            "u8" => TypeID::U8,
            "u16" => TypeID::U16,
            "U32" => TypeID::U32,
            "u64" => TypeID::U64,

            "String" => TypeID::String,

            _ => TypeID::Unknown,
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
            Ident::Word(w) => Token::Str(s[w..idx].trim()),
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

#[derive(Debug)]
pub struct Tokenizer<'a> {
    pub(crate) data: &'a str,
    pub(crate) tokens: Vec<Token<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            tokens: vec![],
        }
    }

    fn push(&mut self, token: Token<'a>) {
        if token == Token::WhiteSpace {
            info!("adding white space");
        }
        trace!("{token:?}");
        self.tokens.push(token);
    }

    /// works as an iterator, the number it returns is an increment amount, you can give it a big
    /// string and repeatedly call next() on it and just increment the start of your slice to get
    /// the next word
    fn next_token(source: &str) -> Option<(usize, Token<'_>, Option<Token<'_>>)> {
        info!("src: {source:?}");

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

                '0'..='9' => {
                    if ident.is_none() {
                        ident = Some(Ident::Number(i));
                    }
                }
                ':' => {
                    if let Some(iden) = ident {
                        return Some((i, iden.into_str(source, i), Some(Token::Colon)));
                    } else {
                        return Some((i + 1, Token::Colon, None));
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
                    return Some((i + 2, Token::LeftAngleBracket, None));
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

                _ => continue,
            }
        }

        if let Some(iden) = ident {
            return Some((source.len(), iden.into_str(source, source.len()), None));
        }

        None
    }

    pub fn tokenize(mut self) -> Self {
        debug!("{:?}", self.data);

        let mut i = 0;
        let mut prev = Token::WhiteSpace;

        loop {
            if i >= self.data.len() {
                break;
            }
            if let Some((inc, wrd, peek)) = Tokenizer::next_token(&self.data[i..]) {
                i += inc;
                if prev == Token::WhiteSpace && wrd == Token::WhiteSpace {
                    continue;
                }
                if wrd == prev {
                    continue;
                }
                if let Token::Str(s) = wrd {
                    match s {
                        "let" => self.push(Token::Let),
                        "struct" => self.push(Token::Struct),
                        "enum" => self.push(Token::Enum),
                        "return" => self.push(Token::Return),
                        "pub" => self.push(Token::Pub),
                        "if" => self.push(Token::If),
                        "else" => self.push(Token::Else),
                        _ => {
                            self.push(wrd);
                        }
                    }
                } else {
                    self.push(wrd);
                }
                info!("word: {wrd:?} {peek:?}");
                if let Some(peek) = peek {
                    self.push(peek);
                }
                prev = wrd;
            }
        }

        self
    }
}

#[cfg(test)]
mod token {
    use crate::tokenizer::{Token, Tokenizer};

    fn setup_logger() {
        _ = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{} {}] {}",
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Info)
            .chain(std::io::stdout())
            .apply();
    }

    #[test]
    fn let_test() {
        setup_logger();

        let data = "let ";

        let tokenizer = Tokenizer::new(data).tokenize();
        assert_eq!([Token::Let, Token::WhiteSpace], tokenizer.tokens.as_slice());

        info!("{tokenizer:?}");
    }

    #[test]
    fn let_var_string() {
        setup_logger();

        let data = "let foo=\"bar\"";

        let tokenizer = Tokenizer::new(data).tokenize();

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::QuotedString("bar")
            ],
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_string_whitespace() {
        setup_logger();

        let data = "let foo = \"bar\"";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_num() {
        setup_logger();

        let data = "let foo=10";

        let tokenizer = Tokenizer::new(data).tokenize();

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Number("10"),
            ],
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_num_whitespace() {
        setup_logger();

        let data = "let foo = 10";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_float() {
        setup_logger();

        let data = "let foo=10.0";

        let tokenizer = Tokenizer::new(data).tokenize();

        assert_eq!(
            [
                Token::Let,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::Equal,
                Token::Number("10.0"),
            ],
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_float_whitespace() {
        setup_logger();

        let data = "let foo = 10.0";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_struct() {
        setup_logger();

        let data = "let foo=Foo{baz=0;}";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_struct_whitespace() {
        setup_logger();

        let data = "let foo = Foo {
        baz = 0;
        }";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_struct_decl() {
        setup_logger();

        let data = "struct Foo{bar:i32,baz:i32,}";

        let tokenizer = Tokenizer::new(data).tokenize();

        assert_eq!(
            [
                Token::Struct,
                Token::WhiteSpace,
                Token::Str("Foo"),
                Token::LeftAngleBracket,
                Token::Str("bar"),
                Token::Colon,
                Token::Str("i32"),
                Token::Comma,
                Token::Str("baz"),
                Token::Colon,
                Token::Str("i32"),
                Token::Comma,
                Token::RightAngleBracket
            ],
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_struct_decl_whitespace() {
        setup_logger();

        let data = "struct Foo {
        bar: i32,
        baz:i32,
        }";

        let tokenizer = Tokenizer::new(data).tokenize();

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
                Token::Str("i32"),
                Token::Comma,
                Token::WhiteSpace,
                Token::Str("baz"),
                Token::Colon,
                Token::Str("i32"),
                Token::Comma,
                Token::WhiteSpace,
                Token::RightAngleBracket
            ],
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_enum_decl() {
        setup_logger();

        let data = "enum Foo{Bar,Baz,}";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_enum_decl_whitespace() {
        setup_logger();

        let data = "enum Foo {
        Bar,
        Baz,
        }";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_enum() {
        setup_logger();

        let data = "let foo=Foo::Urmom;";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[test]
    fn let_var_enum_whitespace() {
        setup_logger();

        let data = "let foo = Foo::Urmom;";

        let tokenizer = Tokenizer::new(data).tokenize();

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
            tokenizer.tokens.as_slice()
        );
    }

    #[cfg(test)]
    mod ops {
        use crate::tokenizer::{Token, Tokenizer, token::setup_logger};

        #[test]
        fn add() {
            setup_logger();
            let data = "foo+bar";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Plus, Token::Str("bar")],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn add_whitespace() {
            setup_logger();
            let data = "foo+ ";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Plus, Token::WhiteSpace],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn sub() {
            setup_logger();
            let data = "foo-bar";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Minus, Token::Str("bar")],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn sub_whitespace() {
            setup_logger();
            let data = "foo- ";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Minus, Token::WhiteSpace],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn mult() {
            setup_logger();
            let data = "foo*bar";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Multiply, Token::Str("bar")],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn mult_whitespace() {
            setup_logger();
            let data = "foo* ";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Multiply, Token::WhiteSpace],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn div() {
            setup_logger();
            let data = "foo/bar";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Divide, Token::Str("bar")],
                tokenizer.tokens.as_slice()
            );
        }

        #[test]
        fn div_whitespace() {
            setup_logger();
            let data = "foo/ ";

            let tokenizer = Tokenizer::new(data).tokenize();

            assert_eq!(
                [Token::Str("foo"), Token::Divide, Token::WhiteSpace],
                tokenizer.tokens.as_slice()
            );
        }
    }
}

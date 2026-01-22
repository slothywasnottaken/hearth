use std::fmt::Display;

use tracing::{info, instrument};

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
    // should typeid be a type? introspection could be cool but a little useless :3
    TypeID(TypeID),
    Return,

    // types
    Struct,
    Array,
    Number(&'a str),
    Str(&'a str),
    Float,

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

    #[instrument(skip_all)]
    pub fn push(&mut self, token: Token<'a>) {
        tracing::trace!(?token);
        self.tokens.push(token);
    }

    #[instrument(skip(self))]
    pub fn tokenize(mut self) -> Self {
        let mut word = None;

        self.data = self.data.trim();
        tracing::trace!(?self.data);

        let mut chs = self.data.chars();
        let mut i = 0;

        let mut need_space = false;

        loop {
            let Some(c) = chs.next() else {
                break;
            };
            if i >= self.data.trim().len() {
                break;
            }
            match c {
                'a'..='z' | 'A'..='Z' => {
                    if word.is_none() {
                        word = Some(i);
                    }
                }

                ' ' => {
                    if let Some(idx) = word {
                        tracing::trace!("found word {:?}", &self.data[idx..i].trim());
                    }
                    if need_space {
                        self.push(Token::WhiteSpace);
                        need_space = false;
                    }
                    if let Some(s) = word {
                        let w = self.data[s..i].trim();
                        let mut idx = None;
                        for (y, c) in w.chars().enumerate() {
                            if c == ' ' {
                                idx = Some(y)
                            }
                        }
                        let wrd = match idx {
                            Some(id) => self.data[s..s + id].trim(),
                            None => self.data[s..i].trim(),
                        };
                        info!(?wrd);
                        match wrd {
                            "let" => {
                                self.push(Token::Let);
                                word = None;
                                need_space = true;

                                info!("tokenizing variable");
                                let res = match idx {
                                    Some(id) => self.variable_state_machine(s + id),
                                    None => self.variable_state_machine(i),
                                };
                                tracing::trace!(?res);
                                i += res;
                            }
                            "struct" => {
                                self.push(Token::Struct);
                                word = None;
                                need_space = true;

                                info!("tokenizing struct");
                                let res = match idx {
                                    Some(id) => self.struct_state_machine(s + id),
                                    None => self.struct_state_machine(i),
                                };

                                info!(?res);
                                info!("{:?}", self.data[i..i + res].trim());
                                info!("{:?}", self.data[i + res..].trim());
                                i += res;
                                info!("finished tokenizing struct");
                            }
                            "fn" => {
                                self.push(Token::Function);
                            }
                            "if" => {
                                self.push(Token::If);
                            }
                            "else" => {
                                self.push(Token::Else);
                            }
                            _ => {}
                        }
                    }
                }

                _ => {}
            }
            i += 1;
        }

        self
    }

    #[instrument(skip(self))]
    fn variable_state_machine(&mut self, idx: usize) -> usize {
        #[derive(Debug)]
        enum VariableState {
            VarIdent,
            VarValue,
            Semicolon,
            Number,
            Str,
            Struct,
        }

        #[derive(Debug)]
        enum Ident {
            Str(usize),
            Number(usize),
        }

        let mut state = VariableState::VarIdent;

        let mut ident: Option<Ident> = None;

        let mut quote: Option<usize> = None;
        tracing::trace!("tokenizing var");
        info!("src {:?}", self.data[idx..].trim());
        let mut found_space = false;

        for (i, ch) in self.data[idx..].chars().enumerate() {
            info!(?state, ?ch);
            match state {
                VariableState::VarIdent => match ch {
                    '=' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(id) => {
                                    self.push(Token::Str(
                                        &self.data[idx + id..idx + i.saturating_sub(1)],
                                    ));
                                }
                                Ident::Number(id) => self.push(Token::Number(
                                    &self.data[idx + id..idx + i.saturating_sub(1)],
                                )),
                            }
                            self.push(Token::Equal);
                            state = VariableState::VarValue;
                            ident = None;
                        }
                    }
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(Ident::Str(i));
                        }
                    }
                    ' ' => {
                        if !found_space {
                            self.push(Token::WhiteSpace);
                            found_space = true;
                        }
                    }
                    _ => {}
                },
                VariableState::VarValue => match ch {
                    'a'..='z' | 'A'..='Z' => {
                        ident = Some(Ident::Str(i));
                        state = VariableState::Str;
                    }
                    '0'..='9' => {
                        ident = Some(Ident::Number(i));
                        state = VariableState::Number;
                    }
                    _ => {}
                },

                VariableState::Number => match ch {
                    ' ' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(_) => panic!("incorrect ident type"),

                                Ident::Number(id) => self.push(Token::Number(
                                    self.data[idx + id..idx + i.saturating_sub(1)].trim(),
                                )),
                            }

                            ident = None;
                        }
                    }
                    ';' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(_) => panic!("incorrect ident type"),

                                Ident::Number(id) => {
                                    self.push(Token::Number(self.data[idx + id..idx + i].trim()))
                                }
                            }

                            ident = None;
                        }
                        state = VariableState::Semicolon;
                    }

                    'a'..='z' | 'A'..='Z' => {
                        panic!("expected number found string");
                    }

                    '0'..='9' => continue,
                    '\n' => continue,

                    c => panic!("found {c:?}"),
                },

                VariableState::Str => match ch {
                    ' ' => {
                        if let Some(iden) = ident {
                            info!(?iden);
                            match iden {
                                Ident::Str(id) => {
                                    self.push(Token::Str(self.data[idx + id..idx + i].trim()))
                                }

                                Ident::Number(_) => panic!("incorrect ident type"),
                            }

                            ident = None;
                        }
                    }
                    ';' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(id) => self.push(Token::Str(
                                    self.data[idx + id..idx + i.saturating_sub(1)].trim(),
                                )),

                                Ident::Number(_) => panic!("incorrect ident type"),
                            }

                            ident = None;
                        }
                        state = VariableState::Semicolon;
                    }

                    '"' => match quote {
                        None => quote = Some(i),
                        Some(ix) => {
                            let val = self.data[idx + ix..idx + i.saturating_sub(1)].trim();
                            info!(?val);
                            self.push(Token::Str(val));
                            ident = None;
                        }
                    },
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(Ident::Str(i));
                        }
                        continue;
                    }
                    '0'..='9' => {
                        panic!(
                            "trying to tokenize string: number or number in a string is unsupported"
                        );
                    }
                    '{' => {
                        state = VariableState::Struct;
                        self.push(Token::LeftAngleBracket);
                    }
                    '\n' => continue,
                    _ => panic!("found ch: {ch:?}"),
                },

                VariableState::Struct => {
                    match ch {
                        'a'..='z' | 'A'..='Z' => {
                            if ident.is_none() {
                                ident = Some(Ident::Str(i));
                            }
                            continue;
                        }
                        '0'..='9' => {
                            if ident.is_none() {
                                ident = Some(Ident::Number(i));
                            }
                        }
                        ' ' | '\n' => {
                            if ch == ' ' && !found_space {
                                self.push(Token::WhiteSpace);
                                found_space = true;
                            }

                            if let Some(ref iden) = ident {
                                match iden {
                                    Ident::Str(id) => {
                                        self.push(Token::Str(self.data[idx + id..idx + i].trim()));
                                        ident = None;
                                    }

                                    Ident::Number(id) => {
                                        self.push(Token::Number(
                                            self.data[idx + id..idx + i].trim(),
                                        ));
                                        ident = None;
                                    }
                                }
                            }
                        }

                        ',' => {
                            if let Some(ref iden) = ident {
                                match iden {
                                    Ident::Str(id) => {
                                        self.push(Token::Str(self.data[idx + id..idx + i].trim()));
                                        ident = None;
                                    }

                                    Ident::Number(id) => {
                                        self.push(Token::Number(
                                            self.data[idx + id..idx + i].trim(),
                                        ));
                                        ident = None;
                                    }
                                }
                            }
                            self.push(Token::Comma);
                        }
                        /* for if the struct is let <ident> = Foo {
                         * urmom: 0
                         * } */
                        '}' => {
                            if let Some(ref iden) = ident {
                                match iden {
                                    Ident::Str(id) => {
                                        self.push(Token::Str(
                                            self.data[idx + id..idx + i + 1].trim(),
                                        ));
                                        ident = None;
                                    }

                                    Ident::Number(id) => {
                                        self.push(Token::Number(
                                            self.data[idx + id..idx + i].trim(),
                                        ));
                                        ident = None;
                                    }
                                }
                            }
                            self.push(Token::RightAngleBracket);
                        }

                        '=' => {
                            if let Some(ref iden) = ident {
                                match iden {
                                    Ident::Str(id) => {
                                        self.push(Token::Str(self.data[idx + id..idx + i].trim()))
                                    }

                                    Ident::Number(_) => panic!(),
                                }
                            }
                            self.push(Token::Equal);
                            ident = None;
                        }
                        ';' => {
                            self.push(Token::Semicolon);
                            return i + 1;
                        }

                        _ => {}
                    }
                }

                VariableState::Semicolon => {
                    self.push(Token::Semicolon);

                    return i;
                }
            }
        }

        0
    }

    #[instrument(skip(self))]
    fn struct_state_machine(&mut self, idx: usize) -> usize {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum StructState {
            Name,
            LeftBracket,
            VarIdent,
            TypeID,
        }

        let mut state = StructState::Name;

        let mut ident = None;

        tracing::trace!("src {:?}", self.data[idx..].trim());

        let mut found_space = false;

        for (i, ch) in self.data[idx..].chars().enumerate() {
            tracing::trace!(?state);
            match state {
                StructState::Name => match ch {
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    ' ' | '\n' => {
                        if ch == ' ' && !found_space {
                            self.push(Token::WhiteSpace);
                            found_space = true;
                        }
                        if let Some(iden) = ident {
                            let var = self.data[idx + iden..idx + i].trim();
                            self.push(Token::Str(var));
                            state = StructState::LeftBracket;
                            ident = None;
                            found_space = false;
                        }
                    }
                    c => panic!("{c:?}"),
                },
                StructState::LeftBracket => match ch {
                    '{' => {
                        if let Some(iden) = ident {
                            self.push(Token::Str(&self.data[idx + iden..idx + i]));
                        }

                        self.push(Token::LeftAngleBracket);
                        state = StructState::VarIdent;
                    }
                    ' ' | '\n' => continue,
                    c => panic!("{c:?}"),
                },
                StructState::VarIdent => match ch {
                    ':' => {
                        match ident {
                            Some(iden) => {
                                let var = self.data[idx + iden..idx + i].trim();
                                info!(?var);
                                self.push(Token::Str(var));
                                ident = None;
                                state = StructState::TypeID;
                            }
                            None => {
                                panic!("expect <name>:");
                            }
                        }
                        self.push(Token::Colon);
                        // state = StructState::Colon;
                    }
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    '}' => {
                        self.push(Token::RightAngleBracket);
                        return i + 1;
                    }
                    ' ' => {
                        if !found_space {
                            self.push(Token::WhiteSpace);
                            found_space = true;
                        }
                    }
                    _ => {}
                },
                StructState::TypeID => match ch {
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    ' ' | '\n' | ',' | '}' => {
                        if let Some(iden) = ident {
                            self.push(Token::TypeID(TypeID::from(
                                self.data[idx + iden..idx + i].trim(),
                            )));
                        }

                        if !found_space && ch == ' ' {
                            self.push(Token::WhiteSpace);
                            state = StructState::VarIdent;
                        }
                        if ch == ',' {
                            self.push(Token::Comma);
                            state = StructState::VarIdent;
                        }
                        if ch == '}' {
                            self.push(Token::RightAngleBracket);
                        }
                    }
                    _ => {}
                },
            }
        }

        0
    }
}

#![allow(unused)]

#[derive(Debug)]
pub struct Parser<'a> {
    data: &'a str,
    ast: Vec<AstNode<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { data, ast: vec![] }
    }

    pub fn parse(self) -> Self {
        let tokenizer = Tokenizer::new(self.data).tokenize();
        println!("{tokenizer:?}");
        //
        // let mut found_let = false;
        // let mut is_var_name: Option<&'a str> = None;
        // let mut found_equal = false;
        //
        // for token in tokenizer.tokens {
        //     match token {
        //         Token::Let => found_let = true,
        //         Token::Str(s) => {
        //             if let Some(v) = is_var_name
        //                 && found_equal
        //             {
        //                 let var = Variable::String(v.to_string());
        //                 self.ast
        //                     .push(AstNode::Variable((is_var_name.unwrap(), var)));
        //             }
        //             if found_let {
        //                 is_var_name = Some(s);
        //             }
        //         }
        //         Token::Equal => found_equal = true,
        //         Token::Number(s) => {
        //             if let Some(v) = is_var_name
        //                 && found_equal
        //             {
        //                 let var = Variable::U64(v.parse::<u64>().unwrap());
        //                 self.ast
        //                     .push(AstNode::Variable((is_var_name.unwrap(), var)));
        //             }
        //             if found_let {
        //                 is_var_name = Some(s);
        //             }
        //         }
        //         Token::Semicolon => {
        //             found_let = false;
        //             found_equal = false;
        //             is_var_name = None;
        //         }
        //     }
        // }

        self
    }
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    data: &'a str,
    tokens: Vec<Token<'a>>,
}

#[derive(Debug)]
pub enum TokenizerState {
    Searching,
    Variable,
    Struct,
}

impl<'a> Tokenizer<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            tokens: vec![],
        }
    }

    pub fn push(&mut self, token: Token<'a>) {
        // println!("pushing {token:?}");
        self.tokens.push(token);
    }

    pub fn tokenize(mut self) -> Self {
        let mut word = None;

        let mut state = TokenizerState::Searching;
        let src = self.data.trim();
        println!("src {src:?}");

        for (i, c) in src.chars().enumerate() {
            match state {
                TokenizerState::Searching => match c {
                    'a'..='z' | 'A'..='Z' => {
                        if word.is_none() {
                            word = Some(i);
                        }
                    }
                    ' ' => {
                        if let Some(s) = word {
                            let w = self.data[s..i].trim();
                            match w {
                                "let" => {
                                    self.push(Token::Let);
                                    state = TokenizerState::Variable;
                                    word = None;
                                }
                                "struct" => {
                                    self.push(Token::Struct);
                                    state = TokenizerState::Struct;
                                    word = None;
                                }
                                _ => {}
                            }
                        }
                    }

                    c => println!("found: {c:?}"),
                },
                // found let
                TokenizerState::Variable => {
                    println!("tokenizing variable");
                    self.variable_state_machine(i);
                    state = TokenizerState::Searching;
                }
                TokenizerState::Struct => {
                    println!("tokenizing struct");
                    self.struct_state_machine(i);
                    state = TokenizerState::Searching;
                    println!("finished tokenizing struct");
                }
            }
        }

        self
    }

    fn variable_state_machine(&mut self, idx: usize) {
        #[derive(Debug)]
        enum VariableState {
            Equal,
            VarIdent,
            VarValue,
            Semicolon,
            Number,
            Str,
        }

        #[derive(Debug)]
        enum Ident {
            Str(usize),
            Number(usize),
        }

        let mut state = VariableState::VarIdent;

        let mut ident = None;

        let mut quote = None;
        println!("src {:?}", self.data[idx..].trim());

        for (i, ch) in self.data[idx..].chars().enumerate() {
            println!("{state:?}");
            match state {
                VariableState::VarIdent => match ch {
                    '=' => {
                        state = VariableState::Equal;
                    }
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(Ident::Str(i));
                        }
                    }
                    _ => {}
                },
                VariableState::Equal => {
                    if let Some(iden) = ident {
                        match iden {
                            Ident::Str(id) => {
                                self.push(Token::Str(
                                    self.data[idx + id..idx + i.saturating_sub(1)].trim(),
                                ));
                            }
                            Ident::Number(id) => self.push(Token::Number(
                                self.data[idx + id..idx + i.saturating_sub(1)].trim(),
                            )),
                        }
                        self.push(Token::Equal);
                        state = VariableState::VarValue;
                        ident = None;
                    }
                }
                VariableState::Semicolon => {
                    self.push(Token::Semicolon);
                    return;
                }
                VariableState::Number => match ch {
                    ' ' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(id) => panic!("incorrect ident type"),

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
                                Ident::Str(id) => panic!("incorrect ident type"),

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

                    c => panic!("found {c}"),
                },
                VariableState::Str => match ch {
                    ' ' => {
                        if let Some(iden) = ident {
                            match iden {
                                Ident::Str(id) => self.push(Token::Str(
                                    self.data[idx + id..idx + i.saturating_sub(1)].trim(),
                                )),

                                Ident::Number(id) => panic!("incorrect ident type"),
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

                                Ident::Number(id) => panic!("incorrect ident type"),
                            }

                            ident = None;
                        }
                        state = VariableState::Semicolon;
                    }

                    '"' => match quote {
                        None => quote = Some(i),
                        Some(ix) => {
                            let val = self.data[idx + ix..idx + i.saturating_sub(1)].trim();
                            println!("val {val:?}");
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
                    _ => panic!("{} {ch}", &self.data[..idx + i]),
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
            }
        }
    }

    fn struct_state_machine(&mut self, idx: usize) {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum StructState {
            Name,
            LeftBracket,
            VarIdent,
            Colon,
        }

        let mut state = StructState::Name;

        let mut found_white_space = false;
        let mut ident = None;

        println!("src {:?}", self.data[idx..].trim());

        for (i, ch) in self.data[idx..].chars().enumerate() {
            println!("state {state:?}");
            match state {
                StructState::Name => match ch {
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    ' ' | '\n' => {
                        self.push(Token::WhiteSpace);
                        if let Some(iden) = ident {
                            let var = self.data[idx + iden..idx + i].trim();
                            println!("pushing var {var}");
                            self.push(Token::Str(var));
                            state = StructState::LeftBracket;
                            ident = None;
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
                    '}' => self.push(Token::RightAngleBracket),
                    ' ' => {
                        if !found_white_space {
                            self.push(Token::WhiteSpace);
                            found_white_space = true;
                        }
                    }

                    ':' => {
                        match ident {
                            Some(iden) => {
                                let var = self.data[idx + iden..idx + i].trim();
                                println!("var {var:?}");
                                self.push(Token::Str(var));
                                ident = None;
                            }
                            None => {
                                panic!("expect <name>:");
                            }
                        }
                        self.push(Token::Colon);
                        state = StructState::Colon;
                        found_white_space = false;
                    }
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    _ => {}
                },
                StructState::Colon => match ch {
                    'a'..='z' | 'A'..='Z' => {
                        if ident.is_none() {
                            ident = Some(i);
                        }
                    }
                    ',' => match ident {
                        Some(iden) => {
                            self.push(Token::Str(self.data[idx + iden..idx + i].trim()));
                            ident = None;
                            found_white_space = false;
                        }
                        None => {
                            continue;
                        }
                    },
                    '}' => {
                        println!("ident {ident:?}");
                        if let Some(iden) = ident {
                            self.push(Token::Str(self.data[idx + iden..idx + i].trim()));
                            ident = None;
                        }
                        self.push(Token::RightAngleBracket);
                        return;
                    }

                    _ => {}
                },
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Let,
    Equal,
    Number(&'a str),
    WhiteSpace,
    Str(&'a str),
    Semicolon,
    LeftAngleBracket,
    RightAngleBracket,
    Struct,
    Colon,
    TypeID(TypeID),
}

#[derive(Debug)]
pub enum Declaration {
    Variable,
    Struct,
}

#[derive(Debug)]
pub enum Builtin {
    Link,
    Move,
    Exec,
}

#[derive(Debug)]
pub enum Variable {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    String(String),
}

impl Variable {
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
        }
    }
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug)]
pub enum AstNode<'a> {
    Builtin(Builtin),
    Variable((&'a str, Variable)),
}

#[cfg(test)]
mod test {
    use crate::parser::{Token, Tokenizer};

    #[test]
    fn str_var() {
        let src = r#"let foo="bar";"#;
        let tokenizer = Tokenizer::new(src).tokenize();
        println!("{:?}", tokenizer.tokens);
        assert_eq!(*tokenizer.tokens.get(4).unwrap(), Token::Str("bar"))
    }

    #[test]
    fn num_var() {
        let src = "let foo = 10; ";
        let tokenizer = Tokenizer::new(src).tokenize();
        assert_eq!(*tokenizer.tokens.get(4).unwrap(), Token::Number("10"))
    }

    #[test]
    fn multiple_vars() {
        let src = "let foo = \"bar\"; let foo = 10;";
        let tokenizer = Tokenizer::new(src).tokenize();
        println!("{tokenizer:?}");
    }

    #[test]
    fn parse_struct() {
        let src = "struct foo {
            bar: i32,
        }";

        let tokenizer = Tokenizer::new(src).tokenize();

        assert_eq!(
            *tokenizer.tokens.as_slice(),
            [
                Token::Struct,
                Token::WhiteSpace,
                Token::Str("foo"),
                Token::LeftAngleBracket,
                Token::WhiteSpace,
                Token::Str("bar"),
                Token::Colon,
                Token::Str("i32"),
                Token::RightAngleBracket
            ]
        );

        println!("{tokenizer:?}");
    }
}

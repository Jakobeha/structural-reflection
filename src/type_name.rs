use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::iter::{empty, once};
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use join_lazy_fmt::Join;
use derive_more::{Display, Error};
use logos::{Lexer, Logos};

/// A detailed rust type name which lets you extract components like generic args if identifier, or underlying type if a reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RustTypeName {
    /// Identifier. Primitive type names are also identifiers, and they have no qualifier or generic_args
    Ident {
        qualifier: Qualifier,
        simple_name: String,
        generic_args: Vec<RustTypeName>
    },
    /// Anonymous types include names closures, [RustType::unknown], and [RustType::bottom].
    Anonymous {
        desc: Cow<'static, str>
    },
    /// Constant expression (usually as a generic arg)
    ConstExpr {
        code_as_string: String
    },
    /// "pointer" encompasses both references and raw pointers.
    Pointer {
        refd: Box<RustTypeName>,
        ptr_kind: RustPointerKind
    },
    /// Tuple or c-tuple (see `structural_rust_type::c_tuple`)
    Tuple {
        elems: Vec<RustTypeName>
    },
    /// Array (specified length)
    Array {
        elem: Box<RustTypeName>,
        length: usize
    },
    /// Slice (unspecified length)
    Slice {
        elem: Box<RustTypeName>
    },
}

/// Module qualifier
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "{}", "\"::\".join(self.0.iter())")]
pub struct Qualifier(Vec<String>);

/// Macro to create literal qualifier
pub macro qualifier {
    [$($qualifier:expr),*] => {
        Qualifier::from(vec![$($qualifier.to_string()),*])
    }
}

/// "pointer" encompasses both references and raw pointers, and this enum contains their immutable (shared) and mutable variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RustPointerKind {
    ImmRef,
    MutRef,
    ImmRaw,
    MutRaw
}

#[doc(hidden)]
pub struct RustTypeNameQualified<'a>(&'a RustTypeName);

#[doc(hidden)]
pub struct RustTypeNameUnqualified<'a>(&'a RustTypeName);

#[doc(hidden)]
pub struct RustTypeNameDisplay<'a, 'b> {
    type_name: &'a RustTypeName,
    qualify: RustTypeNameDisplayQualify<'b>
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub enum RustTypeNameDisplayQualify<'b> {
    Never,
    Always,
    OnlyAmbiguous {
        dnis: &'b DuplicateNamesInScope
    }
}

/// Struct to track duplicate names, if you want to print names unqualified when allowed.
pub struct DuplicateNamesInScope {
    counts: HashMap<String, usize>
}

impl DuplicateNamesInScope {
    pub fn new() -> Self {
        DuplicateNamesInScope {
            counts: HashMap::new()
        }
    }

    /// Check if the name has more than 1 occurrence: if so, it must be qualified.
    pub fn is_ambiguous(&self, name: &str) -> bool {
        self.counts.get(name).map_or(false, |count| *count > 1)
    }
}

impl<'a> Extend<&'a str> for DuplicateNamesInScope {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item = &'a str> {
        for name in iter {
            let count = self.counts.entry(name.to_string()).or_insert(0);
            *count += 1;
        }
    }
}

impl<'a> FromIterator<&'a str> for DuplicateNamesInScope {
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item = &'a str> {
        let mut scope = DuplicateNamesInScope::new();
        scope.extend(iter);
        scope
    }
}

impl RustTypeName {
    /// Identifier with no generic arguments
    pub fn scoped_simple(qualifier: Qualifier, simple_name: String) -> RustTypeName {
        RustTypeName::Ident {
            qualifier,
            simple_name,
            generic_args: Vec::new()
        }
    }

    /// Identifier with no qualifier or generic arguments
    pub fn simple(simple_name: String) -> RustTypeName {
        RustTypeName::Ident {
            qualifier: Qualifier::local(),
            simple_name,
            generic_args: Vec::new()
        }
    }

    /// Primitive type with the given name (e.g. `usize`)
    pub(crate) fn primitive(name: &str) -> RustTypeName {
        Self::simple(String::from(name))
    }

    /// Unknown anonymous type
    pub fn unknown() -> RustTypeName {
        RustTypeName::Anonymous {
            desc: Cow::Borrowed("unknown")
        }
    }

    /// Bottom type. A special anonymous type which is a structural subtype of everything including itself.
    pub fn bottom() -> RustTypeName {
        RustTypeName::Anonymous {
            desc: Cow::Borrowed("bottom")
        }
    }

    /// Is [RustTypeName::unknown]?
    pub fn is_unknown(&self) -> bool {
        match self {
            RustTypeName::Anonymous { desc } => desc == "unknown",
            _ => false
        }
    }

    /// Is [RustTypeName::bottom]?
    pub fn is_bottom(&self) -> bool {
        match self {
            RustTypeName::Anonymous { desc } => desc == "bottom",
            _ => false
        }
    }

    /// Is a [RustTypeName::Anonymous]?
    pub fn is_anonymous(&self) -> bool {
        matches!(self, RustTypeName::Anonymous { .. })
    }

    /// Convert generic parameters in idents to `{unknown}`, ignore everything else.
    /// Useful e.g. so you can register types like `Box<{unknown}>` if you know the size and alignment.
    pub fn erase_generics(&mut self) {
        if let RustTypeName::Ident { qualifier: _, simple_name: _, generic_args } = self {
            for generic_arg in generic_args.iter_mut() {
                *generic_arg = RustTypeName::unknown();
            }
        }
    }

    /// Remove a qualifier if is is the same as the given qualifier
    pub fn remove_qualifier(&mut self, qualifier_to_remove: &Qualifier) {
        match self {
            RustTypeName::Ident { qualifier, simple_name: _, generic_args } => {
                if qualifier == qualifier_to_remove {
                    qualifier.0.clear();
                }
                for generic_arg in generic_args {
                    generic_arg.remove_qualifier(qualifier_to_remove);
                }
            }
            RustTypeName::Anonymous { .. } => {}
            RustTypeName::ConstExpr { .. } => {}
            RustTypeName::Pointer { .. } => {}
            RustTypeName::Tuple { elems } => {
                for elem in elems {
                    elem.remove_qualifier(qualifier_to_remove);
                }
            }
            RustTypeName::Array { elem, length: _ } => {
                elem.remove_qualifier(qualifier_to_remove);
            }
            RustTypeName::Slice { elem} => {
                elem.remove_qualifier(qualifier_to_remove);
            }
        }
    }

    /// Iterate the type's own (if identifier) and nested simple names
    pub fn iter_simple_names(&self) -> impl Iterator<Item=&str> {
        match self {
            RustTypeName::Ident {
                qualifier: _,
                simple_name,
                generic_args
            } => {
                Box::new(once(simple_name.as_str()).chain(
                    generic_args.iter().flat_map(|arg| arg.iter_simple_names())
                )) as Box<dyn Iterator<Item=&str>>
            }
            RustTypeName::Anonymous { .. } => Box::new(empty()) as Box<dyn Iterator<Item=&str>>,
            RustTypeName::ConstExpr { .. } => Box::new(empty()) as Box<dyn Iterator<Item=&str>>,
            RustTypeName::Pointer { ptr_kind: _, refd } => refd.iter_simple_names(),
            RustTypeName::Tuple { elems } => Box::new(
                elems.iter().flat_map(|elem| elem.iter_simple_names())
            ) as Box<dyn Iterator<Item=&str>>,
            RustTypeName::Array { elem, length: _ } => elem.iter_simple_names(),
            RustTypeName::Slice { elem } => elem.iter_simple_names()
        }
    }

    /// Display the type name qualified
    #[must_use = "this does not display the type name, it returns an object that can be displayed"]
    pub fn qualified(&self) -> RustTypeNameDisplay<'_, 'static> {
        RustTypeNameDisplay {
            type_name: self,
            qualify: RustTypeNameDisplayQualify::Always
        }
    }

    /// Display the type name unqualified
    #[must_use = "this does not display the type name, it returns an object that can be displayed"]
    pub fn unqualified(&self) -> RustTypeNameDisplay<'_, 'static> {
        RustTypeNameDisplay {
            type_name: self,
            qualify: RustTypeNameDisplayQualify::Never
        }
    }

    /// Displays the type name, qualifying its own and nested simple names if they are ambiguous
    #[must_use = "this does not display the type name, it returns an object that can be displayed"]
    pub fn display<'a, 'b>(&'a self, dnis: &'b DuplicateNamesInScope) -> RustTypeNameDisplay<'a, 'b> {
        RustTypeNameDisplay {
            type_name: self,
            qualify: RustTypeNameDisplayQualify::OnlyAmbiguous { dnis }
        }
    }
}


impl Qualifier {
    /// Local = no qualifier
    pub fn local() -> Self {
        Qualifier(Vec::new())
    }

    pub fn is_local(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterator over the qualifier's components
    pub fn iter(&self) -> impl Iterator<Item=&str> {
        self.0.iter().map(|s| s.as_str())
    }

    /// Mutable iterator over the qualifier's components
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut String> {
        self.0.iter_mut()
    }
}

impl From<Vec<String>> for Qualifier {
    fn from(v: Vec<String>) -> Self {
        Qualifier(v)
    }
}

// region Qualifier iterator
impl<'a> IntoIterator for &'a Qualifier {
    type Item = &'a str;
    type IntoIter = std::iter::Map<std::slice::Iter<'a, String>, fn(&'a String) -> &'a str>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|s| s.as_str())
    }
}

impl<'a> IntoIterator for &'a mut Qualifier {
    type Item = &'a mut String;
    type IntoIter = std::slice::IterMut<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl IntoIterator for Qualifier {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
// endregion

// region printing
impl<'a, 'b> Display for RustTypeNameDisplay<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let display = |type_name: &'a RustTypeName| -> RustTypeNameDisplay<'a, 'b> {
            RustTypeNameDisplay {
                type_name,
                qualify: self.qualify
            }
        };
        match &self.type_name {
            RustTypeName::Ident {
                simple_name,
                qualifier,
                generic_args
            } => {
                if self.qualify.do_qualify(simple_name) {
                    for qualifier_item in qualifier {
                        write!(f, "{}::", qualifier_item)?;
                    }
                }
                write!(f, "{}", simple_name)?;
                if !generic_args.is_empty() {
                    write!(f, "<{}>", ", ".join(generic_args.iter().map(display)))?;
                }
                Ok(())
            }
            RustTypeName::Anonymous { desc } => write!(f, "{{{}}}", desc),
            RustTypeName::ConstExpr { code_as_string } => write!(f, "{}", code_as_string),
            RustTypeName::Pointer {
                ptr_kind,
                refd
            } => write!(f, "{}{}", ptr_kind, display(refd)),
            RustTypeName::Tuple { elems } => write!(f, "({})", ", ".join(elems.iter().map(display))),
            RustTypeName::Array { elem, length } => write!(f, "[{}; {}]", display(elem), length),
            RustTypeName::Slice { elem } => write!(f, "[{}]", display(elem))
        }
    }
}

impl Display for RustPointerKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RustPointerKind::ImmRef => write!(f, "&"),
            RustPointerKind::MutRef => write!(f, "&mut "),
            RustPointerKind::ImmRaw => write!(f, "*const "),
            RustPointerKind::MutRaw => write!(f, "&mut ")
        }
    }
}

impl<'a> RustTypeNameDisplayQualify<'a> {
    fn do_qualify(&self, simple_name: &str) -> bool {
        match self {
            RustTypeNameDisplayQualify::Never => false,
            RustTypeNameDisplayQualify::Always => true,
            RustTypeNameDisplayQualify::OnlyAmbiguous { dnis } => {
                dnis.is_ambiguous(simple_name)
            }
        }
    }
}
// endregion

// region parsing
#[derive(Debug, Display, Error)]
#[display(fmt = "parse error at {}: {}", index, cause)]
pub struct RustTypeNameParseError {
    pub index: usize,
    pub cause: RustTypeNameParseErrorCause
}

#[derive(Debug, Display, Error)]
pub enum RustTypeNameParseErrorCause {
    IntegerParseError(#[error(source)] ParseIntError),
    ArrayIntegerParseError(#[error(source)] TryFromIntError),
    FloatParseError(#[error(source)] ParseFloatError),
    #[display(fmt = "unexpected token: {}", _0)]
    Unexpected(#[error(not(source))] String),
    #[display(fmt = "expected comma or close, got {}", _0)]
    ExpectedCommaOrClose(#[error(not(source))] String),
    #[display(fmt = "expected semicolon or close, got {}", _0)]
    ExpectedSemicolonOrClose(#[error(not(source))] String),
    #[display(fmt = "expected more")]
    ExpectedMore
}

#[derive(Logos)]
pub enum RustTypeNameToken {
    #[token("*const")]
    ImmPtr,
    #[token("*mut")]
    MutPtr,
    #[token("&mut", priority = 2)]
    MutRef,

    #[token("::")]
    DoubleColon,

    #[regex("[~!@#$%^&*-=+|:;,.?/(\\[{<>}\\])]", |lex| lex.slice().chars().next().unwrap())]
    Punct(char),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    #[regex("`[^`]*`")]
    Ident,

    #[regex("-?[0-9]+", priority = 2, callback = |lex| lex.slice().parse::<i64>())]
    Integer(Result<i64, ParseIntError>),
    #[regex("-?[0-9]+\\.[0-9]*", |lex| lex.slice().parse::<f64>())]
    #[regex("-?[0-9]+\\.[0-9]+e[0-9]+", |lex| lex.slice().parse::<f64>())]
    #[regex("-?[0-9]+e[0-9]+", |lex| lex.slice().parse::<f64>())]
    Float(Result<f64, ParseFloatError>),
    #[regex("\"([^\"]|\\\\\")*\"")]
    String,

    #[regex(r"[ \t\n\f]+", logos::skip)]
    #[error]
    Error,
}

#[derive(Debug, Display, Error)]
pub enum QualifierParseError {
    #[display(fmt = "bad qualifier ident: {}", _0)]
    BadIdent(#[error(not(source))] String)
}

impl Qualifier {
    fn check(idents: &[String]) -> Result<(), QualifierParseError> {
        for ident in idents {
            if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(QualifierParseError::BadIdent(ident.clone()));
            }
        }
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for Qualifier {
    type Error = QualifierParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        let idents = str.split("::").map(String::from).collect::<Vec<_>>();
        Qualifier::check(&idents)?;
        Ok(Qualifier::from(idents))
    }
}


impl<'a> TryFrom<&'a str> for RustTypeName {
    type Error = RustTypeNameParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        let mut lexer = Lexer::<RustTypeNameToken>::new(str);
        RustTypeName::parse_from(&mut lexer, true)
    }
}

enum RustTypeNameParseState {
    Init,
    AfterIdent {
        qualifier: Qualifier,
        simple_name: String
    },
    ExpectsIdent {
        qualifier: Qualifier
    },
    Done {
        result: RustTypeName
    }
}

impl RustTypeName {
    pub fn parse_from(lexer: &mut Lexer<'_, RustTypeNameToken>, parse_eof: bool) -> Result<Self, RustTypeNameParseError> {
        let mut state = RustTypeNameParseState::Init;
        let mut ptr_stack = Vec::new();
        fn unexpected(lexer: &Lexer<'_, RustTypeNameToken>) -> RustTypeNameParseError {
            RustTypeNameParseError {
                index: lexer.span().start,
                cause: RustTypeNameParseErrorCause::Unexpected(lexer.slice().to_string())
            }
        }
        fn expected_comma_or_close(lexer: &Lexer<'_, RustTypeNameToken>) -> RustTypeNameParseError {
            RustTypeNameParseError {
                index: lexer.span().start,
                cause: RustTypeNameParseErrorCause::ExpectedCommaOrClose(lexer.slice().to_string())
            }
        }
        fn expected_semicolon_or_close(lexer: &Lexer<'_, RustTypeNameToken>) -> RustTypeNameParseError {
            RustTypeNameParseError {
                index: lexer.span().start,
                cause: RustTypeNameParseErrorCause::ExpectedSemicolonOrClose(lexer.slice().to_string())
            }
        }
        fn unexpected_end(lexer: &Lexer<'_, RustTypeNameToken>) -> RustTypeNameParseError {
            RustTypeNameParseError {
                index: lexer.span().end,
                cause: RustTypeNameParseErrorCause::ExpectedMore
            }
        }
        while let Some(token) = lexer.next() {
            state = match state {
                RustTypeNameParseState::Init => match token {
                    RustTypeNameToken::Ident => RustTypeNameParseState::AfterIdent {
                        qualifier: Qualifier::local(),
                        simple_name: lexer.slice().to_string()
                    },
                    RustTypeNameToken::ImmPtr => {
                        ptr_stack.push(RustPointerKind::ImmRaw);
                        RustTypeNameParseState::Init
                    }
                    RustTypeNameToken::MutPtr => {
                        ptr_stack.push(RustPointerKind::MutRaw);
                        RustTypeNameParseState::Init
                    }
                    RustTypeNameToken::MutRef => {
                        ptr_stack.push(RustPointerKind::MutRef);
                        RustTypeNameParseState::Init
                    }
                    RustTypeNameToken::Punct('&') => {
                        ptr_stack.push(RustPointerKind::ImmRef);
                        RustTypeNameParseState::Init
                    }
                    RustTypeNameToken::Punct('(') => {
                        let mut elems = Vec::new();
                        if lexer.remainder().trim_start().starts_with(')') {
                            let next = lexer.next();
                            debug_assert!(matches!(next, Some(RustTypeNameToken::Punct(')'))));
                        } else {
                            loop {
                                elems.push(RustTypeName::parse_from(lexer, false)?);
                                match lexer.next() {
                                    Some(RustTypeNameToken::Punct(')')) => break,
                                    Some(RustTypeNameToken::Punct(',')) => {},
                                    Some(_) => return Err(expected_comma_or_close(lexer)),
                                    None => return Err(unexpected_end(lexer))
                                }
                            }
                        }
                        RustTypeNameParseState::Done {
                            result: RustTypeName::Tuple { elems }
                        }
                    }
                    RustTypeNameToken::Punct('[') => {
                        let elem = Box::new(RustTypeName::parse_from(lexer, false)?);
                        match lexer.next() {
                            Some(RustTypeNameToken::Punct(']')) => RustTypeNameParseState::Done {
                                result: RustTypeName::Slice { elem }
                            },
                            Some(RustTypeNameToken::Punct(';')) => match lexer.next() {
                                Some(RustTypeNameToken::Integer(integer)) => match integer {
                                    Ok(integer) => match usize::try_from(integer) {
                                        Ok(length) => match lexer.next() {
                                            Some(RustTypeNameToken::Punct(']')) => RustTypeNameParseState::Done {
                                                result: RustTypeName::Array { elem, length }
                                            },
                                            Some(_) => return Err(unexpected(lexer)),
                                            None => return Err(unexpected_end(lexer))
                                        },
                                        Err(error) => return Err(RustTypeNameParseError {
                                            index: lexer.span().start,
                                            cause: RustTypeNameParseErrorCause::ArrayIntegerParseError(error)
                                        })
                                    },
                                    Err(err) => return Err(RustTypeNameParseError {
                                        index: lexer.span().start,
                                        cause: RustTypeNameParseErrorCause::IntegerParseError(err)
                                    })
                                },
                                Some(_) => return Err(unexpected(lexer)),
                                None => return Err(unexpected_end(lexer))
                            },
                            Some(_) => return Err(expected_semicolon_or_close(lexer)),
                            None => return Err(unexpected_end(lexer))
                        }
                    },
                    RustTypeNameToken::Punct('{') => match lexer.next() {
                        Some(RustTypeNameToken::Ident) => {
                            let desc = lexer.slice().to_string();
                            match lexer.next() {
                                Some(RustTypeNameToken::Punct('}')) => RustTypeNameParseState::Done {
                                    result: RustTypeName::Anonymous { desc: Cow::Owned(desc) }
                                },
                                Some(_) => return Err(unexpected(lexer)),
                                None => return Err(unexpected_end(lexer))
                            }
                        },
                        _ => return Err(unexpected(lexer))
                    },
                    RustTypeNameToken::Integer(integer) => match integer {
                        Ok(integer) => RustTypeNameParseState::Done {
                            result: RustTypeName::ConstExpr {
                                code_as_string: integer.to_string()
                            }
                        },
                        Err(error) => return Err(RustTypeNameParseError {
                            index: lexer.span().end,
                            cause: RustTypeNameParseErrorCause::IntegerParseError(error)
                        })
                    },
                    RustTypeNameToken::Float(float) => match float {
                        Ok(float) => RustTypeNameParseState::Done {
                            result: RustTypeName::ConstExpr {
                                code_as_string: float.to_string()
                            }
                        },
                        Err(error) => return Err(RustTypeNameParseError {
                            index: lexer.span().end,
                            cause: RustTypeNameParseErrorCause::FloatParseError(error)
                        })
                    },
                    RustTypeNameToken::String => {
                        RustTypeNameParseState::Done {
                            result: RustTypeName::ConstExpr {
                                code_as_string: lexer.slice().to_string()
                            }
                        }
                    }
                    _ => return Err(unexpected(lexer))
                }
                RustTypeNameParseState::AfterIdent {
                    mut qualifier,
                    simple_name
                } => match token {
                    RustTypeNameToken::DoubleColon => {
                        qualifier.0.push(simple_name);
                        RustTypeNameParseState::ExpectsIdent {
                            qualifier
                        }
                    }
                    RustTypeNameToken::Punct('<') => {
                        let mut elems = Vec::new();
                        loop {
                            elems.push(RustTypeName::parse_from(lexer, false)?);
                            match lexer.next() {
                                Some(RustTypeNameToken::Punct('>')) => break,
                                Some(RustTypeNameToken::Punct(',')) => {},
                                Some(_) => return Err(expected_comma_or_close(lexer)),
                                None => return Err(unexpected_end(lexer))
                            }
                        }
                        RustTypeNameParseState::Done {
                            result: RustTypeName::Ident {
                                qualifier,
                                simple_name,
                                generic_args: elems
                            }
                        }
                    }
                    _ => return Err(unexpected(lexer))
                }
                RustTypeNameParseState::ExpectsIdent {
                    qualifier
                } => match token {
                    RustTypeNameToken::Ident => RustTypeNameParseState::AfterIdent {
                        qualifier,
                        simple_name: lexer.slice().to_string()
                    },
                    _ => return Err(unexpected(lexer))
                }
                RustTypeNameParseState::Done { result: _ } => return Err(unexpected(lexer))
            };
            if !parse_eof {
                let mut remaining_chars = lexer.remainder().trim_start().chars();
                let peek_char = remaining_chars.next();
                let next_peek_char = peek_char.as_ref().and_then(|_| remaining_chars.next());
                if let Some(peek_char) = peek_char {
                    match &peek_char {
                        // Characters which will not be in a type at this position
                        '+' | '-' | '*' | '/' | '=' | '.' | ',' | ')' | ']' | '>' | '}' => break,
                        ':' if next_peek_char != Some(':') => break,
                        _ => {}
                    }
                }
            }
        }
        let mut result = match state {
            RustTypeNameParseState::AfterIdent {
                qualifier,
                simple_name
            } => RustTypeName::Ident {
                qualifier,
                simple_name,
                generic_args: Vec::new()
            },
            RustTypeNameParseState::Init |
            RustTypeNameParseState::ExpectsIdent { .. } => return Err(unexpected_end(lexer)),
            RustTypeNameParseState::Done { result } => result
        };
        for ptr_kind in ptr_stack {
            result = RustTypeName::Pointer {
                ptr_kind,
                refd: Box::new(result)
            }
        }
        Ok(result)
    }
}
// endregion
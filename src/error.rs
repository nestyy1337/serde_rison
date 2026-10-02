use std::fmt::{self, Debug, Display};

/// An error that occurred while serializing or deserializing RISON.
pub struct Error {
    inner: Box<ErrorImpl>,
}

struct ErrorImpl {
    code: ErrorCode,
    offset: Option<usize>,
}

/// Broad classification of an [`Error`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Category {
    /// The input is not syntactically valid RISON.
    Syntax,
    /// The input is valid RISON but does not match the target type, or the
    /// value cannot be represented in RISON.
    Data,
    /// The input ended before a complete value was read.
    Eof,
}

pub(crate) enum ErrorCode {
    Message(Box<str>),
    EofWhileParsing,
    ExpectedChar(char),
    UnexpectedChar(char),
    InvalidEscape(char),
    InvalidNumber,
    TrailingCharacters,
    RecursionLimitExceeded,
}

impl Error {
    pub(crate) fn syntax(code: ErrorCode, offset: usize) -> Self {
        Error {
            inner: Box::new(ErrorImpl {
                code,
                offset: Some(offset),
            }),
        }
    }

    pub(crate) fn data(code: ErrorCode) -> Self {
        Error {
            inner: Box::new(ErrorImpl { code, offset: None }),
        }
    }

    pub(crate) fn fix_offset(mut self, offset: usize) -> Self {
        self.inner.offset.get_or_insert(offset);
        self
    }

    /// Byte offset into the input where the error was detected.
    ///
    /// `None` for errors that are not tied to a position, such as
    /// serialization errors or errors from [`from_value`](crate::from_value).
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        self.inner.offset
    }

    #[must_use]
    pub fn classify(&self) -> Category {
        match self.inner.code {
            ErrorCode::Message(_) => Category::Data,
            ErrorCode::EofWhileParsing => Category::Eof,
            ErrorCode::ExpectedChar(_)
            | ErrorCode::UnexpectedChar(_)
            | ErrorCode::InvalidEscape(_)
            | ErrorCode::InvalidNumber
            | ErrorCode::TrailingCharacters
            | ErrorCode::RecursionLimitExceeded => Category::Syntax,
        }
    }

    #[must_use]
    pub fn is_syntax(&self) -> bool {
        self.classify() == Category::Syntax
    }

    #[must_use]
    pub fn is_data(&self) -> bool {
        self.classify() == Category::Data
    }

    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.classify() == Category::Eof
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::Message(msg) => f.write_str(msg),
            ErrorCode::EofWhileParsing => f.write_str("unexpected end of input"),
            ErrorCode::ExpectedChar(c) => write!(f, "expected `{c}`"),
            ErrorCode::UnexpectedChar(c) => write!(f, "unexpected character `{c}`"),
            ErrorCode::InvalidEscape(c) => write!(f, "invalid escape `!{c}`"),
            ErrorCode::InvalidNumber => f.write_str("invalid number"),
            ErrorCode::TrailingCharacters => f.write_str("trailing characters"),
            ErrorCode::RecursionLimitExceeded => f.write_str("recursion limit exceeded"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.offset {
            Some(offset) => write!(f, "{} at offset {offset}", self.inner.code),
            None => Display::fmt(&self.inner.code, f),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.offset {
            Some(offset) => write!(
                f,
                "Error({:?}, offset: {offset})",
                self.inner.code.to_string()
            ),
            None => write!(f, "Error({:?})", self.inner.code.to_string()),
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::data(ErrorCode::Message(msg.to_string().into_boxed_str()))
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::data(ErrorCode::Message(msg.to_string().into_boxed_str()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("unexpected end of input")]
    UnexpectedEndOfInput,
    #[error("unexpected character: {0}")]
    UnexpectedCharacter(char),
    #[error("invalid number: {0}")]
    InvalidNumber(String),
    #[error("invalid string: {0}")]
    InvalidString(String),
    #[error("invalid boolean: {0}")]
    InvalidBoolean(String),
    #[error("invalid struct: {0}")]
    InvalidStruct(String),
    #[error("invalid null value: {0}")]
    InvalidNull(String),
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

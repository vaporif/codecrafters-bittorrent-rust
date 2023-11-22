use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected end")]
    UnexpectedEnd,
    #[error("generic erroor")]
    Other(#[from] anyhow::Error),
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Other(anyhow::Error::msg(msg.to_string()))
    }
}

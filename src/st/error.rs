use thiserror::Error;

/// Errors for the ST lib
#[derive(Error, Debug)]
pub enum Error {
    /// std::io::Error wrapper
    #[error("IO error")]
    Io(#[from] std::io::Error),
    /// openssl error wrapper
    #[error("openssl error")]
    Ssl(#[from] openssl::error::ErrorStack),
    /// non-base32 character in string
    #[error("Invalid character")]
    Codepoint,
}

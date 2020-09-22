use crate::constants::NOT_FOUND;
use crate::responder::Response;
use crate::responder::StatusCode;
use crate::Octane;
use std::error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::marker::Unpin;
use tokio::io::AsyncWrite;

/// The Error structure holds the kind of error code and
/// the location of the custom 404 file, if given by the user
/// and manages sending errors on internal http errors and other instances
/// will send with blank content if no 404 file is found or will send the
/// 404 file directly if given.
///
/// You will not have to use this manually, to send errors on your own, you
/// can do so by just specifying the error code and the content like
/// `res.status(status_code).send("")`.
pub struct Error {
    kind: StatusCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Custom error type for invalid paths
pub struct InvalidPathError;
#[derive(Debug, Clone, PartialEq, Eq)]
// Custom error type for invalid SSL certificates
pub struct InvalidCertError;

/// Takes in a http stream and a error code and sends to the client
#[macro_export]
#[doc(hidden)]
macro_rules! declare_error {
    ($stream : expr, $error_type : expr) => {
        Error::err($error_type, $stream).await?;
        return Ok(());
    };
}

impl Error {
    pub async fn err<S>(status_code: StatusCode, stream: S) -> Result<(), Box<dyn error::Error>>
    where
        S: AsyncWrite + Unpin,
    {
        Error { kind: status_code }.send(stream).await
    }
    async fn send<S>(self, stream: S) -> Result<(), Box<dyn error::Error>>
    where
        S: AsyncWrite + Unpin,
    {
        let mut res = Response::new_from_slice(NOT_FOUND.as_bytes());
        res.status(self.kind)
            .default_headers()
            .set("Content-Type", "text/html");

        let response = res.get_data();
        Octane::send(response, stream).await?;
        Ok(())
    }
}

impl Display for InvalidPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: make more informative
        write!(f, "Invalid path error")
    }
}

impl error::Error for InvalidPathError {}

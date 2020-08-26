use crate::config::OctaneConfig;
use crate::file_handler::FileHandler;
use crate::responder::StatusCode;
use crate::responder::{BoxReader, Response};
use crate::server::Octane;
use std::error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::marker::Unpin;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};

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
    file_404: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Custom error type for invalid paths
pub struct InvalidPathError;
#[derive(Debug, Clone, PartialEq, Eq)]
// Custom error type for invalid SSL certificates
pub struct InvalidCertError;

impl Error {
    pub async fn err<S>(
        status_code: StatusCode,
        config: &OctaneConfig,
        stream: S,
    ) -> Result<(), Box<dyn error::Error>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        Error {
            kind: status_code,
            file_404: config.file_404.as_ref().map(|e| e.to_path_buf()),
        }
        .send(stream)
        .await
    }
    async fn send<S>(self, stream: S) -> Result<(), Box<dyn error::Error>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let mut res = Response::new_from_slice(b"");
        if self.kind == StatusCode::NotFound {
            if let Some(file_404) = self.file_404 {
                let file = FileHandler::handle_file(&file_404)?;
                res = Response::new(
                    Box::new(file.file) as BoxReader,
                    Some(file.meta.len() as usize),
                );
                res.status(self.kind)
                    .default_headers()
                    .set("Content-Type", "text/html");
            } else {
                res.status(self.kind);
            }
        }
        Octane::send_data(res.get_data(), stream).await
    }
}

impl Display for InvalidPathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: make more informative
        write!(f, "Invalid path error")
    }
}

impl error::Error for InvalidPathError {}

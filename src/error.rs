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

pub struct Error {
    kind: StatusCode,
    file_404: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPathError;
pub struct InvalidCertError;

impl Display for InvalidPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "Invalid path error")
    }
}

impl error::Error for InvalidPathError {}

impl Error {
    pub fn err(status_code: StatusCode, config: &OctaneConfig) -> Self {
        Error {
            kind: status_code,
            file_404: config.file_404.as_ref().map(|e| e.to_path_buf()),
        }
    }
    pub async fn send<S>(self, stream: S) -> Result<(), Box<dyn error::Error>>
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

use crate::config::OctaneConfig;
use crate::constants::*;
use crate::file_handler::FileHandler;
use crate::responder::Response;
use crate::server::Octane;
use std::io::Result;
use std::marker::Unpin;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct Error {
    kind: StatusCode,
    file_404: PathBuf,
}

impl Error {
    pub fn err(status_code: StatusCode, config: &OctaneConfig) -> Self {
        Error {
            kind: status_code,
            file_404: (*config.file_404).to_path_buf(),
        }
    }
    pub async fn send<S>(self, stream: S) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let file = FileHandler::handle_file(&self.file_404)?;
        let mut res = Response::new(b"");
        if let Some(file_404) = file {
            if self.kind == StatusCode::NotFound {
                res = Response::new(&file_404.contents[..]);
                res.status(self.kind)
                    .default_headers()
                    .set("Content-Type", "text/html");
            }
        } else {
            res = Response::new(b"");
            res.status(self.kind).default_headers();
        }

        Octane::send_data(res.get_data(), stream).await?;

        Ok(())
    }
}

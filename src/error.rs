use crate::constants::*;
use crate::responder::Response;
use crate::server::Octane;
use std::io::Result;
use std::marker::Unpin;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader};

pub struct Error {
    pub kind: StatusCode,
}

impl Error {
    pub fn err(status_code: StatusCode) -> Self {
        Error { kind: status_code }
    }
    pub async fn send<S>(self, stream: S) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let file = File::open("templates/error.html").await?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = Vec::new();
        buf_reader.read_to_end(&mut contents).await?;
        let mut res = Response::new(&contents[..]);
        res.status(self.kind)
            .default_headers()
            .with_header("Content-Type", "text/html".to_string());

        Octane::send_data(res.get_data(), stream).await?;

        Ok(())
    }
}

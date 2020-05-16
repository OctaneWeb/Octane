use crate::responder::{Response, StatusCode};
use futures::io::copy;
use smol::Async;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::net::TcpStream;

pub struct Error {
    pub kind: StatusCode,
}

impl Error {
    pub fn err(status_code: StatusCode) -> Self {
        Error { kind: status_code }
    }
    pub async fn send(self, mut stream: Async<TcpStream>) -> std::io::Result<()> {
        let file = File::open("templates/error.html")?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let res = Response::new(&contents)
            .with_status(self.kind)
            .default_headers()
            .with_header("Content-Type", "text/html")
            .get_string();
        copy(&mut res.as_bytes(), &mut stream).await?;
        Ok(())
    }
}

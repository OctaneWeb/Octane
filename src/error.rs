use crate::responder::Response;
use tokio::fs::File;
use tokio::io::copy;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use crate::constants::*;

pub struct Error {
    pub kind: StatusCode,
}

impl Error {
    pub fn err(status_code: StatusCode) -> Self {
        Error { kind: status_code }
    }
    pub async fn send(self, mut stream: TcpStream) -> std::io::Result<()> {
        let file = File::open("templates/error.html").await?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = Vec::new();
        buf_reader.read_to_end(&mut contents).await?;
        let res = Response::new(&contents[..])
            .with_status(self.kind)
            .default_headers()
            .with_header("Content-Type", "text/html")
            .get_string();
        copy(&mut &res[..], &mut stream).await?;
        Ok(())
    }
}

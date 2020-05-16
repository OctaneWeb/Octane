use crate::error::Error;
use crate::file_handler::FileHandler;
use crate::http::Request;
use crate::responder::{Response, StatusCode};
use futures::prelude::*;
use smol::{Async, Task};
use std::collections::HashSet;
use std::net::{TcpListener, TcpStream};

#[derive(Clone)]
pub struct Server {
    static_dir: Option<String>,
    get_paths: HashSet<String>,
    post_paths: HashSet<String>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            static_dir: None,
            get_paths: HashSet::new(),
            post_paths: HashSet::new(),
        }
    }
    pub fn listen(self, port: u16) -> std::io::Result<()> {
        smol::run(async {
            let listener = Async::<TcpListener>::bind(format!("0.0.0.0:{}", port))?;

            loop {
                let (stream, _addr) = listener.with(|l| l.accept()).await?;
                Task::spawn(self.clone().catch_request(stream)).await?;
            }
        })
    }
    pub fn static_dir<'a>(&mut self, dir_name: &'a str) -> &mut Self {
        if !dir_name.trim().is_empty() {
            self.static_dir = Some(dir_name.to_owned())
        }
        self
    }
    pub fn get<'a>(mut self, path: &'a str, clouse: fn(Request, Server)) {
        self.get_paths.insert(path.to_owned());
        clouse(Request::parse(b"").unwrap(), self);
    }
    async fn catch_request(self, stream: TcpStream) -> std::io::Result<()> {
        let mut data = [0; 512];
        let mut stream_async = Async::new(stream)?;
        stream_async.read(&mut data).await?;
        if let Ok(mut request) = std::str::from_utf8(&data) {
            request = request.trim_matches(char::from(0));
            if let Some(parsed_request) = Request::parse(request.as_bytes()) {
                if let Some(dir) = &self.static_dir {
                    if let Some(file) =
                        self.server_static_dir(parsed_request.path.to_owned(), dir.to_owned())?
                    {
                        let response = Response::new(&file.contents)
                            .with_header("Content-Type", &file.get_mime_type())
                            .get_string();
                        Self::send(response, stream_async).await?;
                    } else {
                        Error::err(StatusCode::NotFound).send(stream_async).await?;
                    }
                }
            } else {
                Error::err(StatusCode::BadRequest)
                    .send(stream_async)
                    .await?;
            }
        } else {
            Error::err(StatusCode::BadRequest)
                .send(stream_async)
                .await?
        }
        Ok(())
    }
    async fn send(response: String, stream: Async<TcpStream>) -> std::io::Result<()> {
        futures::io::copy(response.as_bytes(), &mut &stream).await?;
        Ok(())
    }
    fn server_static_dir(&self, path: String, dir: String) -> std::io::Result<Option<FileHandler>> {
        let final_path = format!("{}/{}", dir, path);
        FileHandler::handle_file(&final_path)
    }
}

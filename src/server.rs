use crate::error::Error;
use crate::file_handler::FileHandler;
use crate::http::Request;
use crate::responder::{Response, StatusCode};
use futures::prelude::*;
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::io::copy;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::prelude::*;
use tokio::stream::StreamExt;

const BUF_SIZE: usize = 512;

#[derive(Clone)]
struct ServerConfig {
    static_dir: String,
}

pub struct Server {
    static_dir: Option<String>,
    get_paths: HashSet<String>,
    post_paths: HashSet<String>,
    stream: Option<TcpStream>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            stream: None,
            static_dir: None,
            get_paths: HashSet::new(),
            post_paths: HashSet::new(),
        }
    }
    pub async fn listen(self, port: u16) -> std::io::Result<()> {
        let mut listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
            .await
            .unwrap();
        let static_dir = Arc::new(self.static_dir.unwrap());
        while let Some(stream) = StreamExt::next(&mut listener).await {
            let clone = Arc::clone(&static_dir);
            tokio::spawn(async move {
                match stream {
                    Ok(value) => {
                        let _res = Self::catch_request(
                            value,
                            ServerConfig {
                                static_dir: clone.to_string(),
                            },
                        )
                        .await;
                    }
                    Err(_e) => (),
                }
            });
        }
        Ok(())
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
    async fn catch_request(
        mut stream_async: TcpStream,
        config: ServerConfig,
    ) -> std::io::Result<()> {
        let mut data = Vec::<u8>::new();
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        loop {
            let read = stream_async.read(&mut buf).await?;
            data.extend_from_slice(&buf);
            if read < BUF_SIZE {
                break;
            }
        }
        if let Some(parsed_request) = Request::parse(&data[..]) {
            if let Some(file) = Self::server_static_dir(
                parsed_request.path.to_owned(),
                config.static_dir.to_owned(),
            )? {
                let response = Response::new(&file.contents)
                    .with_header("Content-Type", &file.get_mime_type())
                    .get_string();
                Self::send_data(response, stream_async).await?;
            } else {
                Error::err(StatusCode::NotFound).send(stream_async).await?;
            }
        } else {
            println!("bad request wtf");
            Error::err(StatusCode::BadRequest)
                .send(stream_async)
                .await?;
        }
        Ok(())
    }
    async fn send_data(response: Vec<u8>, mut stream: TcpStream) -> std::io::Result<()> {
        copy(&mut &response[..], &mut stream).await?;
        Ok(())
    }
    fn server_static_dir(path: String, dir: String) -> std::io::Result<Option<FileHandler>> {
        let final_path = format!("{}/{}", dir, path);
        FileHandler::handle_file(&final_path)
    }
}

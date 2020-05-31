use crate::constants::*;
use crate::error::Error;
use crate::request::Request;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::stream::StreamExt;

#[derive(Clone)]
struct ServerConfig {
    static_dir: Option<String>,
    get_paths: Option<Vec<(String, fn(Request) -> ())>>,
}

impl fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerConfig")
            .field("STATIC_DIR", &self.static_dir)
            .field("GET_PATHS_LEN", &self.get_paths.as_ref().unwrap().len())
            .finish()
    }
}

#[derive(Clone)]
pub struct Octane {
    meta_data: ServerConfig,
    keep_alive: Option<Duration>,
}

impl Octane {
    pub fn new() -> Self {
        Octane {
            meta_data: ServerConfig {
                static_dir: None,
                get_paths: None,
            },
            keep_alive: None,
        }
    }
    pub async fn listen(self, port: u16) -> std::io::Result<()> {
        let mut listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
            .await
            .unwrap();
        let server_config = Arc::new(self.meta_data);
        while let Some(stream) = StreamExt::next(&mut listener).await {
            let config_clone = Arc::clone(&server_config);
            tokio::spawn(async move {
                match stream {
                    Ok(value) => {
                        let _res = Self::catch_request(value, config_clone).await;
                    }
                    Err(_e) => (),
                };
            });
        }
        Ok(())
    }

    pub fn get<'a>(&mut self, path: &'a str, closure: fn(Request) -> ()) {
        if let Some(mut paths) = self.clone().meta_data.get_paths {
            paths.push((path.to_string(), closure));
            self.meta_data.get_paths = Some(paths);
        } else {
            self.meta_data.get_paths = Some(vec![(path.to_string(), closure)]);
        }
    }

    async fn catch_request(
        mut stream_async: TcpStream,
        config: Arc<ServerConfig>,
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
            config.get_paths.as_ref().unwrap().iter().for_each(|data| {
                if parsed_request.path == data.0 {
                    data.1(parsed_request.clone())
                }
            });
        } else {
            Error::err(StatusCode::BadRequest)
                .send(stream_async)
                .await?;
        }
        Ok(())
    }
    async fn send_data(response: Vec<u8>, mut stream_async: TcpStream) -> std::io::Result<()> {
        copy(&mut &response[..], &mut stream_async).await?;
        Ok(())
    }
}

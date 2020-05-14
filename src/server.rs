use crate::http::{Request, RequestMethod};
use crate::responder::Response;
use futures::prelude::*;
use smol::{Async, Task};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::net::TcpStream;

pub struct Server;

impl Server {
    pub fn listen(addr: &str) -> std::io::Result<()> {
        smol::run(async {
            let listener = Async::<TcpListener>::bind(addr)?;
            loop {
                let (stream, _addr) = listener.with(|l| l.accept()).await?;
                Task::spawn(Self::handle_request(stream)).await?;
            }
        })
    }
    async fn handle_request(stream: TcpStream) -> std::io::Result<()> {
        let mut data = [0; 512];
        let mut stream_async = Async::new(stream.try_clone()?)?;
        stream_async.read(&mut data).await?;
        let response = std::str::from_utf8(&data)
            .expect("Found invalid UTF-8")
            .trim_matches(char::from(0));
        if let Some(response) = Request::parse(response.as_bytes()) {
            if let RequestMethod::Get = response.method {
                if response.path.contains("html") {
                    println!("{:?}", response.path);
                    match Task::spawn(Self::handle_file(response.path.to_string())).await {
                        Ok(string) => {
                            let response_back = Response::new(&string)
                                .with_header("Content-Type", "text/html")
                                .get_string();
                            println!("{:?}", response_back);
                            std::io::copy(&mut response_back.as_bytes(), &mut stream.try_clone()?);
                        }
                        Err(e) => {
                            if let ErrorKind::NotFound = e.kind() {
                                match Task::spawn(Self::handle_file("error.html".to_owned())).await
                                {
                                    Ok(string) => {
                                        let response_back = Response::new(&string)
                                            .with_header("Content-Type", "text/html")
                                            .get_string();
                                        println!("{:?}", response_back);
                                        std::io::copy(
                                            &mut response_back.as_bytes(),
                                            &mut stream.try_clone()?,
                                        );
                                    }
                                    Err(e) => println!("{:?}", e),
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    async fn handle_file(path: String) -> std::io::Result<String> {
        let mut templates_dir = "templates/".to_owned();
        templates_dir.push_str(&path);
        println!("{:?}", templates_dir);
        let file = File::open(templates_dir)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        Ok(contents)
    }
}

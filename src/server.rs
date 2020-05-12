use std::net::{TcpListener, TcpStream};

use futures::io;
use smol::{Async, Task};

async fn echo(stream: Async<TcpStream>) -> io::Result<()> {
    let hello_world_text = "HTTP/1.1 200 OK\nContent-Type: text/plain\nContent-Length: 12\n\nHello world!";
    io::copy(hello_world_text.as_bytes(), &mut &stream).await?;
    Ok(())
}

pub struct Server;

impl Server {
    pub fn listen(addr: &str) -> io::Result<()> {
        smol::run(async {
            let listener = Async::<TcpListener>::bind(addr)?;
            loop {
                let (stream, _) = listener.accept().await?;

                Task::spawn(echo(stream)).unwrap().detach();
            }
        })
    }
}


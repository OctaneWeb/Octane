#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod path;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
pub mod time;
pub mod util;

use crate::config::OctaneConfig;
use crate::constants::*;
use crate::error::Error;
use crate::request::{HttpVersion, KeepAlive, Request};
use crate::responder::Response;
use crate::router::{Closure, ClosureFlow, Flow, Route, Router};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::stream::StreamExt;
use std::future::Future;
use std::task::Poll;

fn main() {}

/*
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut listener =
        TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 12345)).await?;
    while let Some(stream) = StreamExt::next(&mut listener).await {
        tokio::spawn(async move {
            match stream {
                Ok(mut s) => {
                    let mut data: Vec<u8> = Vec::new();
                    let mut buf: [u8; 20] = [0; 20];
                    let single_byte = s.read_u8().await.unwrap();
                    data.push(single_byte);
                    loop {
                        let read_future = s.read(&mut buf);
                        match read_future.poll() {

                        }
                        data.extend_from_slice(&buf[..read]);
                        if read < 20 {
                            break;
                        }
                    }
                    println!("we done bois: {:?}", String::from_utf8_lossy(&data));
                }
                Err(_e) => (),
            };
        });
    }
    Ok(())
}
*/

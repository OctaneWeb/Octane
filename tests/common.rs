#![cfg(feature = "std")]
use octane::Octane;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;

/// Serve `octane` on `port` while `client` runs, then shut the server down
pub fn run<F: FnOnce()>(octane: Octane, port: u16, client: F) {
    let mut builder = Builder::new_multi_thread();
    builder.enable_all().thread_name("octane-test");

    let runtime = builder.build().expect("Unable to build tokio runtime");
    runtime.spawn(async move {
        octane.listen(port, || {}).await.expect("server failed");
    });
    wait_for(port);
    client();
}

/// Wait until something accepts connections on `port`
pub fn wait_for(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < deadline, "nothing listening on {}", port);
        thread::sleep(Duration::from_millis(10));
    }
}

/// A response read back from the server
#[allow(dead_code)]
pub struct Response {
    pub status: u16,
    pub head: String,
    pub body: Vec<u8>,
}

#[allow(dead_code)]
impl Response {
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.body).expect("body is not utf-8")
    }
}

#[allow(dead_code)]
impl Response {
    /// The value of the `name` header, matched without regard to case
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.eq_ignore_ascii_case(name) {
                Some(value.trim())
            } else {
                None
            }
        })
    }
}

/// Send `method path` with a Host header plus `headers`, and read the response
#[allow(dead_code)]
pub fn request(port: u16, method: &str, path: &str, headers: &[&str]) -> Response {
    let mut req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        method, path, port
    );
    for header in headers {
        req.push_str(header);
        req.push_str("\r\n");
    }
    req.push_str("\r\n");
    let mut stream = connect(port);
    stream.write_all(req.as_bytes()).unwrap();
    read_response(&mut stream, method == "HEAD")
}

/// Send `bytes` as they are and read the response
#[allow(dead_code)]
pub fn raw_request(port: u16, bytes: &[u8]) -> Response {
    let mut stream = connect(port);
    stream.write_all(bytes).unwrap();
    read_response(&mut stream, false)
}

/// Open a connection to the server on `port`
#[allow(dead_code)]
pub fn connect(port: u16) -> TcpStream {
    let stream = TcpStream::connect(("127.0.0.1", port)).expect("cannot connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
}

/// Read one response off `stream`, and nothing past it: the body is as long
/// as its `Content-Length`, or runs to the end of the stream without one.
/// The answer to a `HEAD` request (`head_only`) has no body.
#[allow(dead_code)]
pub fn read_response(stream: &mut TcpStream, head_only: bool) -> Response {
    // a byte at a time, so the start of the next response stays unread
    let mut raw = Vec::new();
    let mut byte = [0u8; 1];
    while !raw.ends_with(b"\r\n\r\n") {
        let read = stream.read(&mut byte).expect("cannot read response");
        assert_eq!(
            read,
            1,
            "connection closed in the head {:?}",
            String::from_utf8_lossy(&raw)
        );
        raw.push(byte[0]);
    }
    raw.truncate(raw.len() - 4);
    let head = String::from_utf8(raw).expect("head is not utf-8");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("response has no status code");
    let mut response = Response {
        status,
        head,
        body: Vec::new(),
    };
    if !head_only {
        match response.header("content-length") {
            Some(len) => {
                response.body = vec![0; len.parse().expect("bad Content-Length")];
                stream
                    .read_exact(&mut response.body)
                    .expect("cannot read body");
            }
            None => {
                stream
                    .read_to_end(&mut response.body)
                    .expect("cannot read body");
            }
        }
    }
    response
}

/// Whether the server has closed `stream`, after waiting for it to
#[allow(dead_code)]
pub fn is_closed(stream: &mut TcpStream) -> bool {
    let mut byte = [0u8; 1];
    matches!(stream.read(&mut byte), Ok(0))
}

/// GET `path` over https, accepting the self-signed test certificate
#[allow(dead_code)]
pub fn https_get(port: u16, path: &str) -> String {
    let mut body = Vec::new();
    let mut easy = curl::easy::Easy::new();
    easy.url(&format!("https://127.0.0.1:{}{}", port, path))
        .unwrap();
    easy.ssl_verify_peer(false).unwrap();
    easy.ssl_verify_host(false).unwrap();
    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                body.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().expect("https request failed");
    }
    String::from_utf8(body).expect("body is not utf-8")
}

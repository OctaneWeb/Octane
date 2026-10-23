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
    raw_request(port, req.as_bytes())
}

/// Send `bytes` as they are and read the response, the server closes the
/// connection after it
#[allow(dead_code)]
pub fn raw_request(port: u16, bytes: &[u8]) -> Response {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("cannot connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream.write_all(bytes).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("cannot read response");
    let end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("response has no end of head");
    let head = String::from_utf8(raw[..end].to_vec()).expect("head is not utf-8");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("response has no status code");
    Response {
        status,
        head,
        body: raw[end + 4..].to_vec(),
    }
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

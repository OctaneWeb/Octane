use octane::request::HttpVersion;
use octane::responder::StatusCode;
use octane::responder::{BoxReader, Response};
use tokio::io::AsyncReadExt;

async fn data_to_string(mut data: (String, BoxReader)) -> String {
    let mut ret = data.0;
    data.1
        .read_to_string(&mut ret)
        .await
        .expect("cannot read to string");
    ret
}

#[octane::test]
async fn success_standard() {
    // default response should provide OK 200 Code
    let req = data_to_string(Response::new_from_slice(b"").get_data()).await;
    assert_eq!(req, "HTTP/1.1 200 OK\r\n\r\n");
}

#[octane::test]
async fn response_with_status_code_different() {
    // Reponse with different status codes should work
    let mut req = Response::new_from_slice(b"");
    req.status(StatusCode::Created);

    assert_eq!(
        data_to_string(req.get_data()).await,
        "HTTP/1.1 201 CREATED\r\n\r\n"
    );
}

#[octane::test]
async fn response_with_different_http_version() {
    // Reponse with different status codes should work
    let mut req = Response::new_from_slice(b"");

    req.http_version(HttpVersion::Http10)
        .status(StatusCode::Created);
    assert_eq!(
        data_to_string(req.get_data()).await,
        "HTTP/1.0 201 CREATED\r\n\r\n"
    );
}

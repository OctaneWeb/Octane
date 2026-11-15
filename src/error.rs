use crate::constants::NOT_FOUND;
use crate::responder::Response;
use octane_http::StatusCode;

/// The Error structure builds the responses the server sends on internal
/// http errors: when no route answers a request or when a request can't be
/// served at all.
///
/// You will not have to use this manually, to send errors on your own, you
/// can do so by just specifying the error code and the content like
/// `res.status(status_code).send("")`.
pub struct Error;

#[derive(Debug, Clone, PartialEq, Eq)]
// Custom error type for invalid SSL certificates
pub struct InvalidCertError;

impl Error {
    /// The response sent for `status_code` when no route answers a request
    /// or it can't be served
    pub(crate) fn response(status_code: StatusCode) -> Response<'static> {
        let mut res = Response::new_from_static(NOT_FOUND.as_bytes());
        res.status(status_code).default_headers();
        res.headers.set("Content-Type", "text/html");
        res
    }
}

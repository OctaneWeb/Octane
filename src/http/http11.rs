use crate::http::{Http, KeepAliveState};
use octane_http::StatusCode;

pub fn http11_check(validator: &mut Http) {
    if validator.request.headers.get("host").is_none() {
        validator.set(StatusCode::BadRequest)
    }
    if let Some(connection_type) = validator.request.headers.get("connection") {
        if connection_type == "keep-alive" {
            validator.set_keepalive(KeepAliveState::UserDefined);
        } else if connection_type == "close" {
            validator.set_keepalive(KeepAliveState::Close)
        }
    }
}

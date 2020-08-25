use crate::http::{KeepAliveState, Validator};
use crate::responder::StatusCode;

pub fn http11_check(validator: &mut Validator) {
    if let None = validator.request.headers.get("host") {
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

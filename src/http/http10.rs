use crate::http::{Http, KeepAliveState};
use crate::request::KeepAlive;
use std::time::Duration;

pub fn http10_check(validator: &mut Http) {
    if let Some(connection_type) = validator.request.headers.get("connection") {
        if connection_type == "keep-alive" {
            if let Some(keep_alive_header) = validator.request.headers.get("keep-alive") {
                let header_details = KeepAlive::parse(keep_alive_header);

                validator.set_keepalive(KeepAliveState::Particular(Duration::from_secs(
                    header_details.timeout().unwrap_or(0),
                )));
            }
        } else if connection_type == "close" {
            validator.set_keepalive(KeepAliveState::Close)
        }
    }
}

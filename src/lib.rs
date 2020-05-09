mod constants;
mod http;

#[cfg(test)]
mod tests {
    use crate::http;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn request_line_success() {
        let req = http::RequestLine::parse("POST /abc/def HTTP/1.1".to_string()).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/abc/def");
        assert_eq!(req.version, "1.1");
    }

    #[test]
    fn request_line_fail_1() {
        assert!(http::RequestLine::parse("POST /abc/def HTTP/1.1 x".to_string()).is_none());
    }

    #[test]
    fn request_line_fail_2() {
        assert!(http::RequestLine::parse("POST /abc/def HTDP/1.1".to_string()).is_none());
    }

    #[test]
    fn request_line_fail_3() {
        assert!(http::RequestLine::parse("POST /abc/def".to_string()).is_none());
    }
}

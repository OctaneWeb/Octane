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
    #[should_panic]
    fn request_line_fail_1() {
        http::RequestLine::parse("POST /abc/def HTTP/1.1 x".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn request_line_fail_2() {
        http::RequestLine::parse("POST /abc/def HTDP/1.1".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn request_line_fail_3() {
        http::RequestLine::parse("POST /abc/def".to_string()).unwrap();
    }
}

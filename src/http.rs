#[derive(Debug, PartialEq)]
pub enum RequestMethod {
    Get,
    Post,
    Delete,
    Put,
    Options,
    Head,
    Trace,
    Connect,
}

pub struct Request<'a> {
    pub repsonse: &'a str,
    pub version: &'a str,
    pub method: RequestMethod,
    pub path: &'a str,
    pub host: &'a str,
    pub content_length: usize,
    pub content_type: &'a str,
}

impl<'a> Request<'a> {
    pub fn new(repsonse: &'a str) -> Self {
        Request {
            repsonse,
            version: Self::get_version(repsonse),
            host: Self::get_host(repsonse),
            path: Self::get_path(repsonse),
            method: Self::get_method(repsonse),
            content_length: Self::get_content_length(repsonse),
            content_type: Self::get_content_type(repsonse),
        }
    }
    fn get_path(repsonse: &'a str) -> &'a str {
        let mut peekable_chars = repsonse.split_whitespace().peekable();
        let mut path_str: &'a str = "";
        while let Some(chars) = peekable_chars.next() {
            if &chars[..1] == "/" {
                path_str = chars;
                break;
            }
        }
        path_str
    }
    fn get_version(repsonse: &'a str) -> &'a str {
        let mut peekable_chars = repsonse.split_whitespace().peekable();
        let mut version_str: &'a str = "";
        while let Some(chars) = peekable_chars.next() {
            if &chars[..1] == "/" {
                if let Some(version) = peekable_chars.next() {
                    if &version[..4] == "HTTP" {
                        if let Some(check) = peekable_chars.next() {
                            if &check[check.len() - 1..] != ":" {
                                if peekable_chars.next().unwrap() != ":" {
                                    panic!("{:?}", "Invalid headers");
                                }
                            }
                        }
                        version_str = &version[5..];
                        if version_str == "" {
                            panic!("{:?}", "cannot find version number, invalid request format");
                        }
                        break;
                    } else {
                        panic!("{:?}", "Invalid protocol");
                    }
                } else {
                    panic!("{:?}", "invalid request format");
                }
            }
        }
        version_str
    }
    fn get_method(repsonse: &'a str) -> RequestMethod {
        let peekable_chars: Vec<&'a str> = repsonse.split_whitespace().collect();

        match peekable_chars[0].to_uppercase().as_str() {
            "POST" => RequestMethod::Post,
            "GET" => RequestMethod::Get,
            "DELETE" => RequestMethod::Delete,
            "PUT" => RequestMethod::Put,
            "OPTIONS" => RequestMethod::Options,
            "HEAD" => RequestMethod::Head,
            "TRACE" => RequestMethod::Trace,
            "CONNECT" => RequestMethod::Connect,
            _ => panic!("{:?}", "invalid method specified in the request"),
        }
    }
    fn get_host(repsonse: &'a str) -> &'a str {
        let mut peekable_chars = repsonse.split_whitespace().peekable();
        let mut host_str: &'a str = "";
        while let Some(chars) = peekable_chars.next() {
            match chars.to_uppercase().as_str() {
                "HOST" => {
                    if let Some(":") = peekable_chars.next() {
                        if let Some(host) = peekable_chars.next() {
                            host_str = host;
                            break;
                        } else {
                            panic!("{:?}", "invalid format, cannot find HOST")
                        }
                    } else {
                        panic!("{:?}", "invalid format, cannot find HOST")
                    }
                }
                "HOST:" => {
                    if let Some(host) = peekable_chars.next() {
                        host_str = host;
                        break;
                    } else {
                        panic!("{:?}", "invalid format, cannot find HOST");
                    }
                }
                _ => (),
            }
        }
        host_str
    }
    fn get_content_length(repsonse: &'a str) -> usize {
        let mut peekable_chars = repsonse.split_whitespace().peekable();
        let mut content_size: usize = 0;
        while let Some(chars) = peekable_chars.next() {
            match chars.to_uppercase().as_str() {
                "CONTENT-LENGTH" => {
                    if let Some(":") = peekable_chars.next() {
                        if let Some(size) = peekable_chars.next() {
                            content_size = size.parse::<usize>().unwrap();
                            break;
                        } else {
                            panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH")
                        }
                    } else {
                        panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH")
                    }
                }
                "CONTENT-LENGTH:" => {
                    if let Some(size) = peekable_chars.next() {
                        content_size = size.parse::<usize>().unwrap();
                        break;
                    } else {
                        panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH");
                    }
                }
                _ => (),
            }
        }
        content_size
    }
    fn get_content_type(repsonse: &'a str) -> &'a str {
        let mut peekable_chars = repsonse.split_whitespace().peekable();
        let mut content_type: &'a str = "";
        while let Some(chars) = peekable_chars.next() {
            match chars.to_uppercase().as_str() {
                "CONTENT-LENGTH" => {
                    if let Some(":") = peekable_chars.next() {
                        if let Some(c_type) = peekable_chars.next() {
                            content_type = c_type;
                            break;
                        } else {
                            panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH")
                        }
                    } else {
                        panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH")
                    }
                }
                "CONTENT-LENGTH:" => {
                    if let Some(c_type) = peekable_chars.next() {
                        content_type = c_type;
                        break;
                    } else {
                        panic!("{:?}", "invalid format, cannot find CONTENT-LENGTH");
                    }
                }
                _ => (),
            }
        }
        content_type
    }
}

pub struct Header {
    pub name: String,
    pub value: String,
}

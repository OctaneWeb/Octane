use crate::constants::*;
use crate::path::PathBuf;
use crate::util::Spliterator;
use std::cfg;
use std::collections::HashMap;
#[cfg(not(feature = "raw_headers"))]
use std::marker::PhantomData;
use std::ops::Deref;
use std::str;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum RequestMethod {
    Options,
    Get,
    Head,
    Post,
    Put,
    Delete,
    Trace,
    Connect,
    All,
    None,
}

impl RequestMethod {
    pub fn values() -> [Self; 10] {
        use RequestMethod::*;
        [
            Options,
            Get,
            Head,
            Post,
            Put,
            Delete,
            Trace,
            Connect,
            All,
            RequestMethod::None,
        ]
    }
    pub fn is_some(&self) -> bool {
        if let Self::None = self {
            false
        } else {
            true
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HttpVersion {
    Http11,
    Http10,
    Http02,
    Http09,
    HttpInvalid,
}

impl HttpVersion {
    pub fn get_version_string(self) -> String {
        match self {
            Self::Http11 => "1.1",
            Self::Http10 => "1.0",
            Self::Http09 => "0.9",
            Self::Http02 => "0.2",
            _ => "",
        }
        .to_owned()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RequestLine {
    pub method: RequestMethod,
    pub path: PathBuf,
    pub version: HttpVersion,
}

impl RequestLine {
    pub fn parse(request_line: &str) -> Option<Self> {
        let mut toks = request_line.split(SP);
        let method = toks.next()?;
        let path = match PathBuf::parse(toks.next()?) {
            Ok(val) => val,
            Err(e) => panic!("{:?}", e),
        };
        let version = toks.next()?;
        let (first, ver) = version.split_at(5);
        let enum_ver = match ver {
            "1.1" => HttpVersion::Http11,
            "1.0" => HttpVersion::Http10,
            "2.0" => HttpVersion::Http02,
            "0.9" => HttpVersion::Http09,
            _ => HttpVersion::HttpInvalid,
        };

        if cfg!(feature = "faithful") && (first != "HTTP/" || toks.next().is_some()) {
            return None;
        }
        let request_method = match method {
            "POST" => RequestMethod::Post,
            "GET" => RequestMethod::Get,
            "DELETE" => RequestMethod::Delete,
            "PUT" => RequestMethod::Put,
            "OPTIONS" => RequestMethod::Options,
            "HEAD" => RequestMethod::Head,
            "TRACE" => RequestMethod::Trace,
            "CONNECT" => RequestMethod::Connect,
            _ => RequestMethod::None,
        };
        Some(Self {
            method: request_method,
            path,
            version: enum_ver,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> Header<'a> {
    pub fn parse(header: &'a str) -> Option<Self> {
        let mut toks = header.splitn(2, ':');
        let name = toks.next()?;
        if name.is_empty() {
            return None;
        }
        if cfg!(feature = "faithful") {
            for c in name.chars() {
                TOKEN_CHARS.get(&c)?;
            }
        }
        let value = toks.next()?.trim_start_matches(|c| c == SP || c == HT);
        if cfg!(feature = "faithful") && value.chars().any(is_ctl) {
            return None;
        }
        Some(Self { name, value })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Headers<'a> {
    pub parsed: HashMap<String, String>,
    #[cfg(feature = "raw_headers")]
    pub raw: Vec<Header<'a>>,
    #[cfg(not(feature = "raw_headers"))]
    pub raw: PhantomData<&'a ()>,
}

impl<'a> Headers<'a> {
    pub fn parse(request: &'a str) -> Option<Self> {
        let toks = Spliterator::new(request.as_bytes(), B_CRLF);
        let mut headers: HashMap<String, String> = HashMap::new();
        #[cfg(feature = "raw_headers")]
        let mut raw_headers: Vec<Header> = Vec::new();
        for tok in toks {
            let parsed = Header::parse(match str::from_utf8(tok) {
                Ok(s) => s,
                Err(_) => return None,
            })?;
            headers
                .entry(parsed.name.to_ascii_lowercase())
                .and_modify(|v| *v = format!("{}, {}", v, parsed.value))
                .or_insert_with(|| parsed.value.to_owned());
            #[cfg(feature = "raw_headers")]
            raw_headers.push(parsed);
        }
        Some(Self {
            parsed: headers,
            #[cfg(feature = "raw_headers")]
            raw: raw_headers,
            #[cfg(not(feature = "raw_headers"))]
            raw: PhantomData,
        })
    }
}

impl<'a> Deref for Headers<'a> {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

pub fn parse_without_body(data: &str) -> Option<(RequestLine, Headers)> {
    let n = data.find("\r\n")?;
    let (line, rest) = data.split_at(n);
    let request_line = RequestLine::parse(line)?;
    let headers = Headers::parse(&rest[2..])?;
    Some((request_line, headers))
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Request<'a> {
    pub request_line: RequestLine,
    pub headers: Headers<'a>,
    pub body: &'a [u8],
    #[cfg(feature = "cookies")]
    pub cookies: Cookies,
}

impl<'a> Request<'a> {
    pub fn parse(request_line: RequestLine, headers: Headers<'a>, body: &'a [u8]) -> Option<Self> {
        #[cfg(feature = "cookies")]
        let cookies: Cookies;
        #[cfg(feature = "cookies")]
        if let Some(v) = headers.get("cookie") {
            cookies = Cookies::parse(v);
        } else {
            cookies = Default::default();
        }
        Some(Self {
            request_line,
            headers,
            #[cfg(feature = "cookies")]
            cookies,
            body,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct KeepAlive {
    pub timeout: Option<u64>,
    pub max: Option<u64>,
}

impl KeepAlive {
    pub fn parse(header: &str) -> Self {
        let mut ret = Self {
            timeout: None,
            max: None,
        };
        for tok in header.split(',') {
            let trimmed = tok.trim();
            let eq_ind = match trimmed.find('=') {
                Some(v) => v,
                None => continue,
            };
            let (name, val_str) = trimmed.split_at(eq_ind);
            let val: u64 = match (&val_str[1..]).parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match name {
                "timeout" => ret.timeout = Some(val),
                "max" => ret.max = Some(val),
                _ => continue,
            };
        }
        ret
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookies {
    pub cookies: HashMap<String, String>,
}

impl Deref for Cookies {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.cookies
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }
}

impl Cookies {
    pub fn parse(header: &str) -> Self {
        let mut hashmap: HashMap<String, String> = HashMap::new();
        for tok in header.split("; ") {
            let eq_ind = match tok.find('=') {
                Some(v) => v,
                None => continue,
            };
            let (first, second) = tok.split_at(eq_ind);
            hashmap.insert(first.to_owned(), second[1..].to_owned());
        }
        Self { cookies: hashmap }
    }
}

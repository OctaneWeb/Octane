use crate::constants::*;
use crate::{default, deref};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookies {
    pub cookies: HashMap<String, String>,
}

deref!(Cookies, HashMap<String, String>, cookies);

default!(Cookies);

impl Cookies {
    pub fn new() -> Self {
        Cookies {
            cookies: HashMap::new(),
        }
    }
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

    pub fn set(&mut self, key: &str, value: &str) {
        self.cookies.insert(key.to_owned(), value.to_owned());
    }
    pub fn serialise(&self) -> String {
        let mut cookies_str = String::new();
        for cookie in self.iter() {
            cookies_str.push_str(&format!("Set-Cookie:{}={}{}", cookie.0, cookie.1, CRLF))
        }
        cookies_str
    }
}

#![cfg(feature = "query_strings")]
use crate::constants::*;
use std::collections::HashMap;
use std::mem;

struct DoublePeek<'a, T>
where
    T: Default,
{
    iter: &'a mut dyn Iterator<Item = T>,
    cache: [T; 2],
    stored: usize,
    unpeek: bool,
}

impl<'a, T> DoublePeek<'a, T>
where
    T: Default,
{
    fn new(it: &'a mut dyn Iterator<Item = T>) -> Self {
        let cache: [T; 2] = Default::default();
        Self {
            iter: it,
            cache,
            stored: 0,
            unpeek: false,
        }
    }

    fn peek(&mut self) -> Option<&T> {
        if self.unpeek && self.stored > 0 {
            self.unpeek = false;
            return Some(&self.cache[0]);
        }
        self.unpeek = false;
        if self.stored == 2 {
            return None;
        }
        let item = self.iter.next();
        match item {
            None => None,
            Some(v) => {
                self.cache[self.stored] = v;
                self.stored += 1;
                Some(&self.cache[self.stored - 1])
            }
        }
    }
}

impl<'a, T> Iterator for DoublePeek<'a, T>
where
    T: Default,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.stored == 1 {
            self.stored = 0;
            return Some(mem::take(&mut self.cache[0]));
        } else if self.stored == 2 {
            self.stored = 1;
            let dat1 = mem::take(&mut self.cache[1]);
            return Some(mem::replace(&mut self.cache[0], dat1));
        }
        self.iter.next()
    }
}

pub fn unescape_hex(string: &str) -> String {
    let mut ret = "".to_string();
    let mut chars = string.chars();
    let mut peekable = DoublePeek::new(&mut chars);
    while let Some(val) = peekable.next() {
        if val != '%' {
            ret.push(val);
        } else {
            peekable.unpeek = true;
            let c1 = match peekable.peek() {
                Some(&v) => v,
                None => {
                    ret.push('%');
                    continue;
                }
            };
            let c2 = match peekable.peek() {
                Some(&v) => v,
                None => {
                    ret.push('%');
                    continue;
                }
            };
            let (h1, h2) = (from_hex(c1), from_hex(c2));
            if h1.is_none() || h2.is_none() {
                ret.push('%');
                continue;
            }
            ret.push((h1.unwrap() * 16 + h2.unwrap()) as char);
            peekable.next();
            peekable.next();
        }
    }
    ret
}

#[cfg(feature = "query_strings")]
pub fn parse_query(query: &str) -> HashMap<String, String> {
    let toks = query.split('&');
    let mut ret: HashMap<String, String> = HashMap::new();
    for tok in toks {
        if cfg!(feature = "faithful") && tok.is_empty() {
            continue;
        }
        match tok.find('=') {
            Some(v) => {
                let (name, val) = tok.split_at(v);
                if cfg!(feature = "faithful") && name.is_empty() {
                    continue;
                }
                ret.insert(unescape_hex(name), unescape_hex(&val[1..]));
            }
            None => {
                ret.insert(unescape_hex(tok), "".to_string());
            }
        }
    }
    ret
}

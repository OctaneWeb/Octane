use proc_macro::{TokenStream, TokenTree};
use std::iter;

pub fn pascal_case(dat: &str) -> String {
    let mut ret = String::new();
    let mut uppercase = true;
    for c in dat.chars() {
        if c == ' ' {
            uppercase = true;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            continue;
        }
        if uppercase {
            ret.push(c.to_ascii_uppercase());
        } else {
            ret.push(c.to_ascii_lowercase());
        }
        uppercase = false;
    }
    ret
}

pub fn extend<T: Into<TokenTree>>(ts: &mut TokenStream, tt: T) {
    ts.extend(iter::once(tt.into()));
}

use crate::util::*;
use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

pub fn status_codes(toks: TokenStream) -> TokenStream {
    let mut ret = TokenStream::new();
    let mut enum_stream = TokenStream::new();
    let mut is_string = false;
    let mut prev_num: i32 = -1;
    let mut entries: Vec<(i32, String, String)> = Vec::new();
    for tt in toks {
        match tt {
            TokenTree::Literal(lit) => {
                let repr = lit.to_string();
                if !is_string {
                    let val: i32 = repr
                        .parse()
                        .expect("Expected number but did not receive one.");
                    prev_num = val;
                } else {
                    let name: &str = &repr[1..repr.len() - 1];
                    let cased = pascal_case(name);
                    entries.push((prev_num, name.to_string(), cased.clone()));
                    extend(&mut enum_stream, Ident::new(&cased, Span::call_site()));
                    extend(&mut enum_stream, Punct::new(',', Spacing::Alone));
                }
                is_string = !is_string;
            }
            _ => continue,
        }
    }
    enum_stream.extend::<TokenStream>("Other(i32, &'static str)".parse().unwrap());
    let enum_group = Group::new(Delimiter::Brace, enum_stream);
    let mut enum_tot = TokenStream::new();
    enum_tot.extend::<TokenStream>(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum StatusCode"
            .parse()
            .unwrap(),
    );
    extend(&mut enum_tot, enum_group);
    ret.extend(enum_tot);
    let mut match_stream = TokenStream::new();
    for (code, name, cased) in entries {
        match_stream.extend::<TokenStream>(
            format!("StatusCode::{} => ({:?}, {:?}),", cased, code, name)
                .parse()
                .unwrap(),
        );
    }
    match_stream.extend::<TokenStream>("StatusCode::Other(n, s) => (*n, s)".parse().unwrap());
    let mut function_body = TokenStream::new();
    function_body.extend::<TokenStream>("match self".parse().unwrap());
    extend(
        &mut function_body,
        Group::new(Delimiter::Brace, match_stream),
    );
    let mut impl_body = TokenStream::new();
    impl_body.extend::<TokenStream>(
        "pub fn fetch(&self) -> (i32, &'static str)"
            .parse()
            .unwrap(),
    );
    extend(&mut impl_body, Group::new(Delimiter::Brace, function_body));
    let mut full_impl = TokenStream::new();
    full_impl.extend::<TokenStream>("impl StatusCode".parse().unwrap());
    extend(&mut full_impl, Group::new(Delimiter::Brace, impl_body));
    ret.extend(full_impl);
    ret
}

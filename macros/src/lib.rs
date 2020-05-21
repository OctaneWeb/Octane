extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree, Ident, Span, Punct, Spacing, Group, Delimiter};
#[macro_use]
extern crate quote;
use std::iter;

fn pascal_case(dat: &str) -> String {
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

fn extend<T: Into<TokenTree>>(ts: &mut TokenStream, tt: T) {
    ts.extend(iter::once(tt.into()));
}

#[proc_macro]
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
                    let val: i32 = repr.parse().expect("Expected number but did not receive one.");
                    prev_num = val;
                } else {
                    let name: &str = &repr[1..repr.len() - 1];
                    let cased = pascal_case(name);
                    entries.push((prev_num, name.to_string(), cased.clone()));
                    extend(&mut enum_stream, Ident::new(&cased, Span::call_site()));
                    extend(&mut enum_stream, Punct::new(',', Spacing::Alone));
                }
                is_string = !is_string;
            },
            _ => continue
        }
    }
    enum_stream.extend::<TokenStream>(quote! {
        Other(i32, &'static str),
    }.into());
    let enum_group = Group::new(Delimiter::Brace, enum_stream);
    let mut enum_tot = TokenStream::new();
    enum_tot.extend::<TokenStream>(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum StatusCode
    }.into());
    extend(&mut enum_tot, enum_group);
    ret.extend(enum_tot);
    let mut match_stream = TokenStream::new();
    for (code, name, cased) in entries {
        match_stream.extend::<TokenStream>(quote! {
            StatusCode::
        }.into());
        extend(&mut match_stream, Ident::new(&cased, Span::call_site()));
        extend(&mut match_stream, Punct::new('=', Spacing::Joint));
        extend(&mut match_stream, Punct::new('>', Spacing::Alone));
        match_stream.extend::<TokenStream>(quote! {
            (#code, #name),
        }.into());
    }
    match_stream.extend::<TokenStream>(quote! {
        StatusCode::Other(n, s) => (*n, s)
    }.into());
    let mut function_body = TokenStream::new();
    function_body.extend::<TokenStream>(quote! {
        match self
    }.into());
    extend(&mut function_body, Group::new(Delimiter::Brace, match_stream));
    let mut impl_body = TokenStream::new();
    impl_body.extend::<TokenStream>(quote! {
        pub fn fetch(&self) -> (i32, &'static str)
    }.into());
    extend(&mut impl_body, Group::new(Delimiter::Brace, function_body));
    let mut full_impl = TokenStream::new();
    full_impl.extend::<TokenStream>(quote! {
        impl StatusCode
    }.into());
    extend(&mut full_impl, Group::new(Delimiter::Brace, impl_body));
    ret.extend(full_impl);
    ret
}

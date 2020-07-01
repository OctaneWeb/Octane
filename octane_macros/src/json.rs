use proc_macro::{Delimiter, Ident, TokenStream, TokenTree};
use crate::util::extend;

pub fn derive_from_json(toks: TokenStream) -> TokenStream {
    let mut tok_iter = toks.into_iter();
    if tok_iter
        .by_ref()
        .find(|tok| match tok {
            TokenTree::Ident(i) => i.to_string() == "struct",
            _ => false,
        })
        .is_none()
    {
        return r#"compile_error!("Only structs are supported.");"#.parse().unwrap();
    }
    let name = tok_iter.next().unwrap().to_string();
    let mut gen_between = TokenStream::new();
    let mut where_between = TokenStream::new();
    let mut cur = &mut gen_between;
    let mut generic_level = 0;
    let mut generics = Vec::new();
    let mut is_gen_name = true;
    for tok in tok_iter {
        if tok.to_string() == "<" {
            generic_level += 1;
        }
        if generic_level == 1 {
            if is_gen_name {
                if let TokenTree::Ident(i) = &tok {
                    generics.push(i.to_string());
                    is_gen_name = false;
                }
            } else if tok.to_string() == "," {
                is_gen_name = true;
            }
        }
        if tok.to_string() == ">" {
            generic_level -= 1;
        }
        if generic_level != 0 {
            extend(cur, tok);
            continue;
        }
        if let TokenTree::Group(grp) = &tok {
            match grp.delimiter() {
                Delimiter::Brace => {
                    let proced =  process_braces(grp.stream(), name, gen_between, where_between, generics);
                    return proced;
                },
                _ => {}
            };
        }
        if tok.to_string() == "where" {
            cur = &mut where_between;
        }
        extend(cur, tok);
    }
    r#"compile_error!("Could not find compatible struct body."#
        .parse()
        .unwrap()
}

fn process_braces(toks: TokenStream, name: String, gen_between: TokenStream, mut where_between: TokenStream, generics: Vec<String>) -> TokenStream {
    let mut fields: Vec<Ident> = Vec::new();
    let mut is_field = true;
    for tok in toks {
        match tok {
            TokenTree::Ident(ident) if is_field => {
                is_field = false;
                fields.push(ident);
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                is_field = true;
            }
            _ => {}
        };
    }
    let mut vals = String::new();
    for field in fields {
        vals.push_str(&format!(
            "{0}: octane::json::FromJSON::from_json(obj.remove({0:?})?)?,",
            field.to_string()
        ));
    }
    let mut gen_list: String = String::new();
    if generics.len() > 0 {
        gen_list.push('<');
    }
    for (i, s) in generics.iter().enumerate() {
        gen_list.push_str(s);
        if i < generics.len() - 1 {
            gen_list.push(',');
        }
    }
    if generics.len() > 0 {
        gen_list.push('>');
    }
    for gen in generics {
        where_between.extend::<TokenStream>(format!(", {}: octane::json::FromJSON", gen).parse().unwrap());
    }
    format!(
        "\
    impl{} octane::json::FromJSON for {}{} {} {{\
        fn from_json(val: octane::json::Value) -> Option<Self> {{\
            if let octane::json::Value::Object(mut obj) = val {{\
                let ret = Self {{\
                    {}\
                }};\
                if !obj.is_empty() {{\
                    return None;\
                }}\
                Some(ret)\
            }} else {{\
                None\
            }}\
        }}\
    }}",
        gen_between, name, gen_list, where_between, vals
    )
    .parse()
    .unwrap()
}

use crate::util::extend;
use proc_macro::{Delimiter, Ident, TokenStream, TokenTree};

pub struct StructInfo {
    name: String,
    gen_between: TokenStream,
    where_between: TokenStream,
    generics: Vec<String>,
}

pub fn handle_derive<
    F: Fn(TokenStream, StructInfo) -> TokenStream,
    K: Fn(TokenStream, StructInfo) -> TokenStream,
>(
    toks: TokenStream,
    process_braces: F,
    process_parens: K,
) -> TokenStream {
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
                    return process_braces(
                        grp.stream(),
                        StructInfo {
                            name,
                            gen_between,
                            where_between,
                            generics,
                        },
                    )
                }
                Delimiter::Parenthesis => {
                    return process_parens(
                        grp.stream(),
                        StructInfo {
                            name,
                            gen_between,
                            where_between,
                            generics,
                        },
                    )
                }
                _ => {}
            };
        }
        if tok.to_string() == "where" {
            cur = &mut where_between;
            continue;
        }
        extend(cur, tok);
    }
    r#"compile_error!("Could not find compatible struct body."#
        .parse()
        .unwrap()
}

pub fn derive_from_json(toks: TokenStream) -> TokenStream {
    handle_derive(toks, fromjson_braces, fromjson_parens)
}

fn fromjson_braces(toks: TokenStream, mut info: StructInfo) -> TokenStream {
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
            "{0}: octane_json::FromJSON::from_json(obj.remove({0:?})?)?,",
            field.to_string()
        ));
    }
    let mut gen_list: String = String::new();
    if info.generics.len() > 0 {
        gen_list.push('<');
    }
    for (i, s) in info.generics.iter().enumerate() {
        gen_list.push_str(s);
        if i < info.generics.len() - 1 {
            gen_list.push(',');
        }
    }
    if info.generics.len() > 0 {
        gen_list.push('>');
    }
    let mut comma = ", ";
    if info
        .where_between
        .clone()
        .into_iter()
        .last()
        .map(|v| v.to_string() == ",")
        .unwrap_or(true)
    {
        comma = "";
    }
    for gen in info.generics {
        info.where_between.extend::<TokenStream>(
            format!("{}{}: octane_json::FromJSON", comma, gen)
                .parse()
                .unwrap(),
        );
        comma = ", ";
    }
    format!(
        "\
    impl{} octane_json::FromJSON for {}{} where {} {{\
        fn from_json(val: octane_json::Value) -> Option<Self> {{\
            if let octane_json::Value::Object(mut obj) = val {{\
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
        info.gen_between, info.name, gen_list, info.where_between, vals
    )
    .parse()
    .unwrap()
}

fn fromjson_parens(toks: TokenStream, mut info: StructInfo) -> TokenStream {
    let mut fields = 0;
    let mut is_field = true;
    for tok in toks {
        match tok {
            TokenTree::Ident(_) if is_field => {
                is_field = false;
                fields += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                is_field = true;
            }
            _ => {}
        };
    }
    let mut vals = String::new();
    for _ in 0..fields {
        vals.push_str("octane_json::FromJSON::from_json(it.next()?)?,");
    }
    let mut gen_list: String = String::new();
    if info.generics.len() > 0 {
        gen_list.push('<');
    }
    for (i, s) in info.generics.iter().enumerate() {
        gen_list.push_str(s);
        if i < info.generics.len() - 1 {
            gen_list.push(',');
        }
    }
    if info.generics.len() > 0 {
        gen_list.push('>');
    }
    let mut comma = ", ";
    if info
        .where_between
        .clone()
        .into_iter()
        .last()
        .map(|v| v.to_string() == ",")
        .unwrap_or(true)
    {
        comma = "";
    }
    for gen in info.generics {
        info.where_between.extend::<TokenStream>(
            format!("{}{}: octane_json::FromJSON", comma, gen)
                .parse()
                .unwrap(),
        );
        comma = ", ";
    }
    format!(
        "\
    impl{} octane_json::FromJSON for {}{} where {} {{\
        fn from_json(val: octane_json::Value) -> Option<Self> {{\
            if let octane_json::Value::Array(arr) = val {{\
                let mut it = arr.into_iter();
                let ret = Self (\
                    {}\
                );\
                if it.next().is_some() {{\
                    return None;\
                }}\
                Some(ret)\
            }} else {{\
                None\
            }}\
        }}\
    }}",
        info.gen_between, info.name, gen_list, info.where_between, vals
    )
    .parse()
    .unwrap()
}

pub fn derive_to_json(toks: TokenStream) -> TokenStream {
    handle_derive(toks, tojson_braces, tojson_parens)
}

fn tojson_braces(toks: TokenStream, mut info: StructInfo) -> TokenStream {
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
            "obj.insert({0:?}.to_owned(), octane_json::ToJSON::to_json(self.{0})?);",
            field.to_string()
        ));
    }
    let mut gen_list: String = String::new();
    if info.generics.len() > 0 {
        gen_list.push('<');
    }
    for (i, s) in info.generics.iter().enumerate() {
        gen_list.push_str(s);
        if i < info.generics.len() - 1 {
            gen_list.push(',');
        }
    }
    if info.generics.len() > 0 {
        gen_list.push('>');
    }
    let mut comma = ", ";
    if info
        .where_between
        .clone()
        .into_iter()
        .last()
        .map(|v| v.to_string() == ",")
        .unwrap_or(true)
    {
        comma = "";
    }
    for gen in info.generics {
        info.where_between.extend::<TokenStream>(
            format!("{}{}: octane_json::ToJSON", comma, gen)
                .parse()
                .unwrap(),
        );
        comma = ", ";
    }
    format!(
        "\
    impl{} octane_json::ToJSON for {}{} where {} {{\
        fn to_json(self) -> Option<octane_json::Value> {{\
            let mut obj = std::collections::HashMap::new();\
            {}\
            Some(octane_json::Value::Object(obj))\
        }}\
    }}",
        info.gen_between, info.name, gen_list, info.where_between, vals
    )
    .parse()
    .unwrap()
}

fn tojson_parens(toks: TokenStream, mut info: StructInfo) -> TokenStream {
    let mut fields = 0;
    let mut is_field = true;
    for tok in toks {
        match tok {
            TokenTree::Ident(_) if is_field => {
                is_field = false;
                fields += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                is_field = true;
            }
            _ => {}
        };
    }
    let mut vals = String::new();
    for i in 0..fields {
        vals.push_str(&format!(
            "arr.push(octane_json::ToJSON::to_json(self.{})?);",
            i
        ));
    }
    let mut gen_list: String = String::new();
    if info.generics.len() > 0 {
        gen_list.push('<');
    }
    for (i, s) in info.generics.iter().enumerate() {
        gen_list.push_str(s);
        if i < info.generics.len() - 1 {
            gen_list.push(',');
        }
    }
    if info.generics.len() > 0 {
        gen_list.push('>');
    }
    let mut comma = ", ";
    if info
        .where_between
        .clone()
        .into_iter()
        .last()
        .map(|v| v.to_string() == ",")
        .unwrap_or(true)
    {
        comma = "";
    }
    for gen in info.generics {
        info.where_between.extend::<TokenStream>(
            format!("{}{}: octane_json::ToJSON", comma, gen)
                .parse()
                .unwrap(),
        );
        comma = ", ";
    }
    format!(
        "\
    impl{} octane_json::ToJSON for {}{} where {} {{\
        fn to_json(self) -> Option<octane_json::Value> {{\
            let mut arr = Vec::new();\
            {}\
            Some(octane_json::Value::Array(arr))\
        }}\
    }}",
        info.gen_between, info.name, gen_list, info.where_between, vals
    )
    .parse()
    .unwrap()
}

use proc_macro::{Delimiter, Ident, TokenStream, TokenTree};

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
    for tok in tok_iter {
        if let TokenTree::Group(grp) = tok {
            match grp.delimiter() {
                Delimiter::Brace => return process_braces(grp.stream(), name),
                _ => {}
            };
        }
    }
    r#"compile_error!("Could not find compatible struct body."#
        .parse()
        .unwrap()
}

fn process_braces(toks: TokenStream, name: String) -> TokenStream {
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
            "{0}: octane::json::FromJSON::from_json(match obj.remove({0:?}) {{\
            Some(v) => v,\
            None => return Err(octane::json::InvalidTypeError)\
        }})?,",
            field.to_string()
        ));
    }
    format!(
        "\
    impl octane::json::FromJSON for {} {{\
        fn from_json(val: octane::json::Value) -> Result<Self, octane::json::InvalidTypeError> {{\
            if let octane::json::Value::Object(mut obj) = val {{\
                Ok(Self {{\
                    {}\
                }})\
            }} else {{\
                Err(octane::json::InvalidTypeError)\
            }}\
        }}\
    }}",
        name, vals
    )
    .parse()
    .unwrap()
}

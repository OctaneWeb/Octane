extern crate proc_macro;

mod json;
mod status;
mod stream_parser;
mod util;

use crate::stream_parser::StreamParser;
use proc_macro::TokenStream;
use proc_macro::TokenTree;
use quote::quote;
use std::path::Path;

/// Used to generate status codes with their number
/// counter parts. You don't need to use this directly
/// as status codes are already declared for you.
#[proc_macro]
pub fn status_codes(toks: TokenStream) -> TokenStream {
    status::status_codes(toks)
}

/// Derive FromJSON to implement the [`FromJSON`](convert/trait.FromJSON.html) trait that allows
/// deserialising struct values to a struct
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// #[derive(FromJSON, ToJSON)]
/// struct User {
///     id: u64,
///     name: String,
///     email: String,
/// }
///
/// impl User {
///     pub fn new() -> Self {
///         Self {
///             id: 1,
///             name: "John Doe".to_string(),
///             email: "JohnDoe@hotmail.com".to_string(),
///         }
///     }
/// }
///
/// let json_string = User::new().to_json_string().unwrap();
/// let user_back: Option<User> = User::from_json_string(&json_string);
/// ```
#[proc_macro_derive(FromJSON)]
pub fn derive_from_json(toks: TokenStream) -> TokenStream {
    json::derive_from_json(toks)
}

/// Derive ToJSON to implement [`ToJSON`](convert/trait.ToJSON.html) trait that allows
/// serialising struct values to valid json
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// #[derive(ToJSON)]
/// struct User {
///     id: u64,
///     name: String,
///     email: String,
/// }
///
/// impl User {
///     pub fn new() -> Self {
///         Self {
///             id: 1,
///             name: "John Doe".to_string(),
///             email: "JohnDoe@hotmail.com".to_string(),
///         }
///     }
/// }
///
/// let json_string = User::new().to_json_string();
/// ```
#[proc_macro_derive(ToJSON)]
pub fn derive_to_json(toks: TokenStream) -> TokenStream {
    json::derive_to_json(toks)
}

/// The main attribute is just like #[tokio::main] but it defines
/// some parameters which are specific to octane
///
/// octane::main attribute sets the thread stack to 10 megabytes, core threads
/// to the number of cpus available * 2, use this in production
///
/// # Example
///
/// ```no_run
/// use octane::prelude::*;
/// use std::error::Error;
///
/// #[octane::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     let mut app = Octane::new();
///     app.add(Octane::static_dir("dir_name"));
///     app.get("/",
///         route!(
///             |req, res| {
///                 res.send("Hello, World");
///                 Flow::Stop
///             }
///         ),
///     )?;
///
///     app.listen(8080).await
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let stream = StreamParser::new(item.into());
    let properties = stream.parse();
    let num_cpus = num_cpus::get() * 2;
    let compile_error;
    if properties.is_async {
        compile_error = quote! {};
    } else {
        compile_error = quote! {
            compile_error!("the async keyword is missing from function declaration");
        }
    }
    let signature = properties.signature;
    let rest = properties.rest;
    let tokens = quote! {
        #signature {
            #compile_error
            let mut builder = tokio::runtime::Builder::new();
            builder
                .threaded_scheduler()
                .enable_io()
                .thread_stack_size(10485760)
                .thread_name("octane-main")
                .core_threads(#num_cpus);

            let mut runtime = builder.build().expect("Unable to build tokio runtime");
            runtime.block_on(async {
                #rest
            })
        }
    };
    let token_stream: TokenStream = tokens.into();
    token_stream
}

/// The test attribute is just like #[tokio::test] but it defines
/// some parameters which are specific to octane
///
/// octane::test attribute sets the scheduler to a basic scheduler, keeps everything
/// else to defaults
#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let stream = StreamParser::new(item.into());
    let properties = stream.parse();
    let compile_error;
    if properties.is_async {
        compile_error = quote! {};
    } else {
        compile_error = quote! {
            compile_error!("the async keyword is missing from function declaration");
        }
    }
    let signature = properties.signature;
    let rest = properties.rest;
    let tokens = quote! {
        #[test]
        #signature {
            #compile_error
            let mut builder = tokio::runtime::Builder::new();
            builder
                .basic_scheduler()
                .enable_io()
                .thread_name("octane-test");

            let mut runtime = builder.build().expect("Unable to build tokio runtime");
            runtime.block_on(async {
                #rest
            })
        }
    };
    let token_stream: TokenStream = tokens.into();
    token_stream
}
/// Alias for `concat!(env!("CARGO_MANIFEST_DIR"), "ANY_PATH")`.
/// So `path!("/url/file")` is equivalent to
/// `concat!(env!("CARGO_MANIFEST_DIR"), "/url/file")`
/// Checks if path exists at compile time, panics if the path
/// doesn't exists.
///
/// # Example
///
/// ```
/// let path = path!("/templates");
/// ```
///
/// This allows for safety, you cannot compile code that has
/// paths that don't exist. It's recommended to use when linking paths.
#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    let input: Vec<TokenTree> = input.into_iter().collect();
    let value = match &input.get(0) {
        Some(TokenTree::Literal(literal)) => literal.to_string(),
        _ => panic!(),
    };
    let str_value: String = value.parse().unwrap();
    let mut path = std::env::current_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    path.push_str(&str_value.replace("\"", ""));
    if !Path::new(&path).exists() {
        panic!("{} -> {}", "This directory or file doesn't exists", path);
    }

    format!("{:?}", path).parse().unwrap()
}

extern crate proc_macro;

mod json;
mod status;
mod stream_parser;
mod util;

use crate::stream_parser::StreamParser;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn status_codes(toks: TokenStream) -> TokenStream {
    status::status_codes(toks)
}

#[proc_macro_derive(FromJSON)]
pub fn derive_from_json(toks: TokenStream) -> TokenStream {
    json::derive_from_json(toks)
}

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
///     app.add(Octane::static_dir("dir_name")); // serve a static directory
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

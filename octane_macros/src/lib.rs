extern crate proc_macro;

mod json;
mod status;
mod stream_parser;
mod util;

use crate::stream_parser::StreamParser;
use proc_macro::TokenStream;

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

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let stream = StreamParser::new(item);
    let properties = stream.parse();
    let num_cpus = num_cpus::get() * 2;
    let mut compiler_error = String::new();
    if !properties.is_async {
        compiler_error.push_str(
            r#"compile_error!("the async keyword is missing from function declaration");"#,
        )
    }
    let final_stream = format!(
        r#"{}{{
            {}
            let mut builder = tokio::runtime::Builder::new();
            builder
                .threaded_scheduler()
                .enable_io()
                .thread_stack_size(32 * 10000000)
                .thread_name("octane-main")
                .core_threads({});

            let mut runtime = builder.build().expect("Unable to build tokio runtime");
            runtime.block_on(async {{
                {}
            }})
        }}"#,
        properties.signature, compiler_error, num_cpus, properties.rest,
    );
    final_stream.parse().unwrap()
}

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let stream = StreamParser::new(item);
    let properties = stream.parse();
    let mut compiler_error = String::new();
    if !properties.is_async {
        compiler_error.push_str(
            r#"compile_error!("the async keyword is missing from function declaration");"#,
        )
    }
    let final_stream = format!(
        r#"{}{{
            {}
            let mut builder = tokio::runtime::Builder::new();
            builder
                .basic_scheduler()
                .enable_io()
                .thread_name("octane-test");

            let mut runtime = builder.build().expect("Unable to build tokio runtime");
            runtime.block_on(async {{
                {}
            }})
        }}"#,
        properties.signature, compiler_error, properties.rest,
    );
    final_stream.parse().unwrap()
}

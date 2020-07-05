extern crate proc_macro;
mod json;
mod status;
mod util;

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

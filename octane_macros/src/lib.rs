extern crate proc_macro;

mod status;
mod util;

use proc_macro::TokenStream;

/// Used to generate status codes with their number
/// counter parts. You don't need to use this directly
/// as status codes are already declared for you.
#[proc_macro]
pub fn status_codes(toks: TokenStream) -> TokenStream {
    status::status_codes(toks)
}

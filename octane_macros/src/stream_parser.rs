use proc_macro2::{Delimiter, TokenStream, TokenTree};
#[derive(Debug, Clone)]
pub struct StreamParser {
    items: TokenStream,
    pub is_async: bool,
    pub rest: TokenStream,
    pub signature: TokenStream,
}

impl StreamParser {
    pub fn new(items: TokenStream) -> Self {
        Self {
            items,
            is_async: false,
            rest: TokenStream::new(),
            signature: TokenStream::new(),
        }
    }
    pub fn parse(mut self) -> Self {
        let mut peekable = self.items.clone().into_iter().peekable();
        let mut signature: Vec<TokenTree> = Vec::new();
        while peekable.peek().is_some() {
            let tt = peekable.peek().unwrap();
            match tt {
                TokenTree::Group(x) => {
                    if let Delimiter::Brace = x.delimiter() {
                        self.rest = x.stream();
                        break;
                    } else {
                        signature.push(peekable.next().unwrap())
                    }
                }
                TokenTree::Ident(x) => {
                    if x.to_string() == "async" {
                        self.is_async = true;
                        peekable.next();
                    } else {
                        signature.push(peekable.next().unwrap());
                    }
                }
                _ => signature.push(peekable.next().unwrap()),
            }
        }
        let final_sig: TokenStream = signature.into_iter().map(TokenStream::from).collect();
        self.signature = final_sig;
        self
    }
}

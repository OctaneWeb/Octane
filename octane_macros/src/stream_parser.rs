use proc_macro::{Delimiter, TokenStream, TokenTree};
#[derive(Debug, Clone)]
pub struct StreamParser {
    items: TokenStream,
    pub is_async: bool,
    pub rest: TokenStream,
    pub signature: String,
}

impl StreamParser {
    pub fn new(items: TokenStream) -> Self {
        Self {
            items,
            is_async: false,
            rest: TokenStream::new(),
            signature: String::new(),
        }
    }
    pub fn parse(mut self) -> Self {
        let mut peekable = self.items.clone().into_iter().peekable();
        while peekable.peek().is_some() {
            let tt = peekable.peek().unwrap();
            match tt {
                TokenTree::Group(x) => {
                    if let Delimiter::Brace = x.delimiter() {
                        self.rest = x.stream();
                        break;
                    }
                }
                TokenTree::Ident(_) => match tt.to_string().as_str() {
                    "async" => {
                        self.is_async = true;
                        peekable.next();
                    }
                    _ => {
                        while peekable.peek().is_some() {
                            let next = peekable.peek();
                            match next {
                                Some(TokenTree::Group(ref x)) => {
                                    if let Delimiter::Brace = x.delimiter() {
                                        break;
                                    } else {
                                        self.signature.push_str(&x.to_string());
                                        peekable.next();
                                    };
                                }
                                Some(TokenTree::Ident(x)) => {
                                    match x.to_string().as_str() {
                                        "dyn" => self.signature.push_str(format!("dyn ").as_str()),
                                        "fn" => self.signature.push_str(format!("fn ").as_str()),
                                        y => self.signature.push_str(&y.to_string()),
                                    };
                                    peekable.next();
                                }
                                Some(x) => {
                                    self.signature.push_str(&x.to_string());
                                    peekable.next();
                                }
                                None => (),
                            }
                        }
                    }
                },
                _ => (),
            }
        }
        self
    }
}

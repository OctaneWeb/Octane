use octane_http::http1x::Method;

/// Holds the type of request method, like GET,
/// POST etc.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum RequestMethod {
    #[doc(hidden)]
    Options,
    #[doc(hidden)]
    Get,
    #[doc(hidden)]
    Head,
    #[doc(hidden)]
    Post,
    #[doc(hidden)]
    Put,
    #[doc(hidden)]
    Delete,
    #[doc(hidden)]
    Trace,
    #[doc(hidden)]
    Connect,
    #[doc(hidden)]
    Patch,
    #[doc(hidden)]
    All,
    #[doc(hidden)]
    None,
}

impl RequestMethod {
    /// Return false if the RequestMethod is `None`, otherwise it's true
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
    /// How many variants there are, see [`index`](#method.index)
    pub(crate) const COUNT: usize = 11;

    /// A distinct number below [`COUNT`](#associatedconstant.COUNT) per
    /// variant, to keep something per method in an array
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// The method's name, empty for `All` and `None` which aren't methods
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Options => "OPTIONS",
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Trace => "TRACE",
            Self::Connect => "CONNECT",
            Self::Patch => "PATCH",
            Self::All | Self::None => "",
        }
    }
}

impl From<Method<'_>> for RequestMethod {
    /// Methods without a variant of their own map to `None`
    fn from(method: Method<'_>) -> Self {
        match method {
            Method::Get => Self::Get,
            Method::Head => Self::Head,
            Method::Post => Self::Post,
            Method::Put => Self::Put,
            Method::Delete => Self::Delete,
            Method::Connect => Self::Connect,
            Method::Options => Self::Options,
            Method::Trace => Self::Trace,
            Method::Patch => Self::Patch,
            Method::Other(_) => Self::None,
        }
    }
}

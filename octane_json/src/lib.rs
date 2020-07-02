pub mod parse;
pub mod value;
pub mod convert;

// Bring important functions to top level namespace.
pub use value::Value;
pub use convert::{FromJSON, ToJSON};
pub use octane_macros::FromJSON;

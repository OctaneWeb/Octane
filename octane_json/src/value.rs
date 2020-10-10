use crate::parse::parse_element;
use std::collections::HashMap;

/// Represents a unit value in parsed json
#[derive(Debug, Clone)]
pub enum Value {
    /// An integer, both signed and unsigned
    Integer(i64),
    /// Floating values
    Float(f64),
    /// Strings
    String(String),
    /// Arrays which are just Vectors of elements [`Value`](enum.Value.html)
    Array(Vec<Value>),
    /// An object is a collection of values that are tied to a key
    /// for example
    /// ```json
    /// {
    ///   "key": "value"
    /// }
    /// ```
    Object(HashMap<String, Value>),
    /// truthy and falshy values
    Boolean(bool),
    /// No value at all
    Null,
}

macro_rules! make_as_func {
    ($name: ident, $type: ty, $variant: ident) => {
        make_as_func!($name, $type, $variant, stringify!($variant));

    };
    ($name: ident, $type: ty, $variant: ident, $doc_string : expr) => {
        #[doc = "Return x if the variant is `"]
        #[doc = $doc_string]
        #[doc = "(x)` else return None"]
        pub fn $name(&self) -> Option<&$type> {
            if let Value::$variant(x) = self {
                Some(x)
            } else {
                None
            }
        }
    }
}

macro_rules! make_is_func {
    ($name: ident, $variant: ident) => {
        make_is_func!($name, $variant, stringify!($variant));
    };
    ($name: ident, $variant: ident, $doc_string: expr) => {
        #[doc = "Returns true if the variant is `"]
        #[doc = $doc_string]
        #[doc = "` else return false"]
        pub fn $name(&self) -> bool {
            matches!(self, Value::$variant(_))
        }
    };
}

macro_rules! make_pe {
    ($type: ident, $variant: ident) => {
        impl PartialEq<$type> for Value {
            fn eq(&self, other: &$type) -> bool {
                if let Value::$variant(x) = self {
                    x.eq(other)
                } else {
                    false
                }
            }
        }
    };
}

impl Value {
    make_as_func!(as_float, f64, Float);
    make_as_func!(as_integer, i64, Integer);
    make_as_func!(as_boolean, bool, Boolean);
    make_as_func!(as_string, String, String);
    make_as_func!(as_array, Vec<Value>, Array);
    make_as_func!(as_object, HashMap<String, Value>, Object);

    /// Return a unit `()` if the variant is Null
    pub fn as_null(&self) -> Option<()> {
        if let Value::Null = self {
            Some(())
        } else {
            None
        }
    }

    make_is_func!(is_float, Float);
    make_is_func!(is_integer, Integer);
    make_is_func!(is_boolean, Boolean);
    make_is_func!(is_string, String);
    make_is_func!(is_array, Array);
    make_is_func!(is_object, Object);

    /// Return true if the variant is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    /// Parse a string literal and return the corresponding
    /// enum variant
    pub fn parse(dat: &str) -> Option<Self> {
        let (val, rest) = parse_element(dat)?;
        if !rest.is_empty() {
            return None;
        }
        Some(val)
    }
}

impl Eq for Value {}

make_pe!(i64, Integer);
make_pe!(f64, Float);
make_pe!(String, String);
make_pe!(bool, Boolean);

impl<T> PartialEq<Vec<T>> for Value
where
    Value: PartialEq<T>,
{
    fn eq(&self, other: &Vec<T>) -> bool {
        if let Value::Array(x) = self {
            x.eq(other)
        } else {
            false
        }
    }
}

impl<T> PartialEq<HashMap<String, T>> for Value
where
    Value: PartialEq<T>,
{
    fn eq(&self, other: &HashMap<String, T>) -> bool {
        if let Value::Object(x) = self {
            if x.len() != other.len() {
                return false;
            }
            for (k, v1) in x {
                if let Some(v2) = other.get(k) {
                    if v1.ne(v2) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        false
    }
}

impl PartialEq<()> for Value {
    fn eq(&self, _: &()) -> bool {
        self.is_null()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => x.eq(y),
            (Value::Float(x), Value::Float(y)) => x.eq(y),
            (Value::String(x), Value::String(y)) => x.eq(y),
            (Value::Boolean(x), Value::Boolean(y)) => x.eq(y),
            (Value::Array(x), Value::Array(y)) => x.eq(y),
            (Value::Object(x), Value::Object(y)) => x.eq(y),
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

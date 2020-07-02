use std::collections::HashMap;
use crate::parse::parse_element;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Boolean(bool),
    Null,
}

macro_rules! make_as_func {
    ($name: ident, $type: ty, $variant: ident) => {
        pub fn $name(&self) -> Option<&$type> {
            if let Value::$variant(x) = self {
                Some(x)
            } else {
                None
            }
        }
    };
}

macro_rules! make_is_func {
    ($name: ident, $variant: ident) => {
        pub fn $name(&self) -> bool {
            if let Value::$variant(_) = self {
                true
            } else {
                false
            }
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
    make_as_func!(as_number, f64, Number);
    make_as_func!(as_boolean, bool, Boolean);
    make_as_func!(as_string, String, String);
    make_as_func!(as_array, Vec<Value>, Array);
    make_as_func!(as_object, HashMap<String, Value>, Object);

    pub fn as_null(&self) -> Option<()> {
        if let Value::Null = self {
            Some(())
        } else {
            None
        }
    }

    make_is_func!(is_number, Number);
    make_is_func!(is_boolean, Boolean);
    make_is_func!(is_string, String);
    make_is_func!(is_array, Array);
    make_is_func!(is_object, Object);

    pub fn is_null(&self) -> bool {
        if let Value::Null = self {
            true
        } else {
            false
        }
    }

    pub fn parse(dat: &str) -> Option<Self> {
        let (val, rest) = parse_element(dat)?;
        if !rest.is_empty() {
            return None;
        }
        Some(val)
    }
}

impl Eq for Value {}

make_pe!(f64, Number);
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
            (Value::Number(x), Value::Number(y)) => x.eq(y),
            (Value::String(x), Value::String(y)) => x.eq(y),
            (Value::Boolean(x), Value::Boolean(y)) => x.eq(y),
            (Value::Array(x), Value::Array(y)) => x.eq(y),
            (Value::Object(x), Value::Object(y)) => x.eq(y),
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

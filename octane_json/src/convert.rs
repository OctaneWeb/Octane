use std::convert::{TryFrom, TryInto};
use crate::value::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTypeError;

pub trait FromJSON
where
    Self: Sized,
{
    fn from_json(val: Value) -> Option<Self>;
}

pub trait ToJSON
where
    Self: Sized,
{
    fn to_json(&self) -> Option<String>;
}

impl<T> FromJSON for T
where
    T: TryFrom<Value>,
{
    fn from_json(val: Value) -> Option<Self> {
        Self::try_from(val).ok()
    }
}

macro_rules! make_tryfrom {
    ($type: ty, $variant: ident) => {
        impl TryFrom<Value> for $type {
            type Error = InvalidTypeError;

            fn try_from(v: Value) -> Result<Self, Self::Error> {
                if let Value::$variant(x) = v {
                    Ok(x)
                } else {
                    Err(InvalidTypeError)
                }
            }
        }
    };
}

make_tryfrom!(bool, Boolean);
make_tryfrom!(f64, Number);
make_tryfrom!(String, String);

impl<T> TryFrom<Value> for Vec<T>
where
    T: FromJSON,
{
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Array(arr) = v {
            arr.into_iter()
                .map(T::from_json)
                .collect::<Option<_>>()
                .ok_or(InvalidTypeError)
        } else {
            Err(InvalidTypeError)
        }
    }
}

impl<T> TryFrom<Value> for HashMap<String, T>
where
    T: FromJSON,
{
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Object(map) = v {
            map.into_iter()
                .map(|(k, v)| Ok((k, T::from_json(v).ok_or(InvalidTypeError)?)))
                .collect::<Result<_, _>>()
        } else {
            Err(InvalidTypeError)
        }
    }
}

impl TryFrom<Value> for () {
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Null = v {
            Ok(())
        } else {
            Err(InvalidTypeError)
        }
    }
}

macro_rules! make_numeric_tryfrom {
    ($type: ty) => {
        impl TryFrom<Value> for $type {
            type Error = InvalidTypeError;

            #[allow(clippy::float_cmp)]
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                let num: f64 = v.try_into()?;
                let conved = num as $type;
                if num == conved as f64 {
                    Ok(conved)
                } else {
                    Err(InvalidTypeError)
                }
            }
        }
    };
}

macro_rules! make_numeric_tojson {
    ($type: ty) => {
        impl ToJSON for $type {
            fn to_json(&self) -> Option<String> {
                Some((*self as f64).to_string())
            }
        }
    };
}

make_numeric_tryfrom!(u128);
make_numeric_tryfrom!(u64);
make_numeric_tryfrom!(u32);
make_numeric_tryfrom!(u16);
make_numeric_tryfrom!(u8);
make_numeric_tryfrom!(i128);
make_numeric_tryfrom!(i64);
make_numeric_tryfrom!(i32);
make_numeric_tryfrom!(i16);
make_numeric_tryfrom!(i8);
make_numeric_tryfrom!(f32);

make_numeric_tojson!(u128);
make_numeric_tojson!(u64);
make_numeric_tojson!(u32);
make_numeric_tojson!(u16);
make_numeric_tojson!(u8);
make_numeric_tojson!(i128);
make_numeric_tojson!(i64);
make_numeric_tojson!(i32);
make_numeric_tojson!(i16);
make_numeric_tojson!(i8);
make_numeric_tojson!(f32);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_json().unwrap())
    }
}

impl ToJSON for f64 {
    fn to_json(&self) -> Option<String> {
        Some(self.to_string())
    }
}

impl ToJSON for () {
    fn to_json(&self) -> Option<String> {
        Some("null".to_string())
    }
}

impl ToJSON for bool {
    fn to_json(&self) -> Option<String> {
        Some(if *self {
            "true".to_string()
        } else {
            "false".to_string()
        })
    }
}

impl ToJSON for String {
    fn to_json(&self) -> Option<String> {
        Some(format!("{:?}", self))
    }
}

impl<T: ToJSON> ToJSON for Vec<T> {
    fn to_json(&self) -> Option<String> {
        let mut ret = "[".to_string();
        let len = self.len();
        for (i, v) in self.iter().enumerate() {
            ret.push_str(&v.to_json()?);
            ret.push(if i < len - 1 { ',' } else { ']' });
        }
        Some(ret)
    }
}

impl<T: ToJSON> ToJSON for HashMap<String, T> {
    fn to_json(&self) -> Option<String> {
        let mut ret = "{".to_string();
        let len = self.len();
        for (i, (k, v)) in self.iter().enumerate() {
            ret.push_str(&k.to_json()?);
            ret.push(':');
            ret.push_str(&v.to_json()?);
            ret.push(if i < len - 1 { ',' } else { '}' });
        }
        Some(ret)
    }
}

impl ToJSON for Value {
    fn to_json(&self) -> Option<String> {
        match self {
            Value::Number(x) => x.to_json(),
            Value::String(x) => x.to_json(),
            Value::Boolean(x) => x.to_json(),
            Value::Array(x) => x.to_json(),
            Value::Object(x) => x.to_json(),
            Value::Null => ().to_json(),
        }
    }
}
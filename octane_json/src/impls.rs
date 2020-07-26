use crate::convert::{FromJSON, ToJSON};
use crate::value::Value;

impl<T: FromJSON> FromJSON for Option<T> {
    fn from_json(val: Value) -> Option<Self> {
        let is_null = val.is_null();
        if let Some(x) = T::from_json(val) {
            Some(Some(x))
        } else if is_null {
            Some(None)
        } else {
            None
        }
    }
}

impl<T: ToJSON> ToJSON for Option<T> {
    fn to_json(self) -> Option<Value> {
        if let Some(x) = self {
            x.to_json()
        } else {
            Some(Value::Null)
        }
    }
}

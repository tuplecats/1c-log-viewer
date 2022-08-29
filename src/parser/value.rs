use chrono::NaiveDateTime;
use std::{borrow::Cow, fmt::Display, ops::Index};

#[derive(Debug, Clone)]
pub enum Value<'a> {
    String(Cow<'a, str>),
    Number(f64),
    DateTime(NaiveDateTime),
    MultiValue(Vec<Value<'a>>),
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        Value::String(Cow::Owned(String::new()))
    }
}

impl<'a> Value<'a> {
    pub fn len(&self) -> usize {
        match self {
            Value::MultiValue(arr) => arr.len(),
            _ => 1,
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = &Value> + '_> {
        match self {
            Value::MultiValue(arr) => Box::new(arr.iter()),
            _ => Box::new(std::iter::repeat(self).take(1)),
        }
    }
}

impl<'a> Index<usize> for Value<'a> {
    type Output = Value<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Value::MultiValue(arr) => &arr[index],
            _ => self,
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(string: &'a str) -> Self {
        if let Ok(value) = string.parse::<f64>() {
            Self::Number(value)
        } else {
            Self::String(Cow::from(string))
        }
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(string: String) -> Self {
        if let Ok(value) = string.as_str().parse::<f64>() {
            Self::Number(value)
        } else {
            Self::String(Cow::from(string))
        }
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::DateTime(dt) => write!(f, "{}", dt),
            Value::MultiValue(arr) => write!(f, "{:?}", arr),
        }
    }
}

impl<'a> PartialEq for Value<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Number(n1), Value::Number(n2)) => n1 == n2,
            (Value::DateTime(dt1), Value::DateTime(dt2)) => dt1 == dt2,
            _ => false,
        }
    }
}

impl<'a> PartialOrd for Value<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1.partial_cmp(s2),
            (Value::Number(n1), Value::Number(n2)) => n1.partial_cmp(n2),
            (Value::DateTime(dt1), Value::DateTime(dt2)) => dt1.partial_cmp(dt2),
            _ => None,
        }
    }
}

impl<'a> PartialEq<String> for Value<'a> {
    fn eq(&self, other: &String) -> bool {
        match self {
            Value::String(s) => s.as_ref() == other,
            _ => false,
        }
    }
}

impl<'a> PartialOrd<String> for Value<'a> {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        match self {
            Value::String(s) => s.as_ref().partial_cmp(other),
            _ => None,
        }
    }
}

impl<'a> PartialEq<f64> for Value<'a> {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Value::Number(n) => n == other,
            _ => false,
        }
    }
}

impl<'a> PartialOrd<f64> for Value<'a> {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        match self {
            Value::Number(n) => n.partial_cmp(other),
            _ => None,
        }
    }
}

impl<'a> PartialEq<NaiveDateTime> for Value<'a> {
    fn eq(&self, other: &NaiveDateTime) -> bool {
        match self {
            Value::DateTime(dt) => dt == other,
            _ => false,
        }
    }
}

impl<'a> PartialOrd<NaiveDateTime> for Value<'a> {
    fn partial_cmp(&self, other: &NaiveDateTime) -> Option<std::cmp::Ordering> {
        match self {
            Value::DateTime(dt) => dt.partial_cmp(other),
            _ => None,
        }
    }
}

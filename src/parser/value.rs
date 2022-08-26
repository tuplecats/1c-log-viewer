use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::Index;
use std::rc::Rc;
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct PartialString {
    inner: Rc<String>,
    begin: usize,
    len: usize,
}

impl PartialString {
    pub fn new(inner: Rc<String>, begin: usize, len: usize) -> Self {
        Self {
            inner,
            begin,
            len
        }
    }

    pub fn as_str(&self) -> &str {
        &self.inner.as_str()[self.begin..(self.begin + self.len)]
    }
}

impl PartialEq for PartialString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialOrd for PartialString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Eq for PartialString {

}

impl Display for PartialString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for PartialString {
    fn default() -> Self {
        PartialString {
            inner: Rc::new("".to_string()),
            begin: 0,
            len: 0
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    String(PartialString),
    Number(f64),
    DateTime(NaiveDateTime),
    MultiValue(Vec<Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::String(PartialString::default())
    }
}

impl Value {
    pub fn new(inner: Rc<String>, begin: usize, len: usize) -> Self {
        let string = PartialString::new(inner, begin, len);
        if let Ok(value) = string.as_str().parse::<f64>() {
            Self::Number(value)
        }
        else {
            Self::String(string)
        }
    }

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

impl Index<usize> for Value {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Value::MultiValue(arr) => &arr[index],
            _ => self,
        }
    }
}


impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::DateTime(dt) => write!(f, "{}", dt),
            Value::MultiValue(arr) => write!(f, "{:?}", arr),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Number(n1), Value::Number(n2)) => n1 == n2,
            (Value::DateTime(dt1), Value::DateTime(dt2)) => dt1 == dt2,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1.partial_cmp(s2),
            (Value::Number(n1), Value::Number(n2)) => n1.partial_cmp(n2),
            (Value::DateTime(dt1), Value::DateTime(dt2)) => dt1.partial_cmp(dt2),
            _ => None,
        }
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        match self {
            Value::String(s) => s.as_str() == other,
            _ => false,
        }
    }
}

impl PartialOrd<String> for Value {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        match self {
            Value::String(s) => s.as_str().partial_cmp(other),
            _ => None,
        }
    }
}

impl PartialEq<f64> for Value {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Value::Number(n) => n == other,
            _ => false,
        }
    }
}

impl PartialOrd<f64> for Value {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        match self {
            Value::Number(n) => n.partial_cmp(other),
            _ => None,
        }
    }
}

impl PartialEq<NaiveDateTime> for Value {
    fn eq(&self, other: &NaiveDateTime) -> bool {
        match self {
            Value::DateTime(dt) => dt == other,
            _ => false,
        }
    }
}

impl PartialOrd<NaiveDateTime> for Value {
    fn partial_cmp(&self, other: &NaiveDateTime) -> Option<std::cmp::Ordering> {
        match self {
            Value::DateTime(dt) => dt.partial_cmp(other),
            _ => None,
        }
    }
}

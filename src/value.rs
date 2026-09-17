use core::panic;
use std::fmt;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function(fn(&[LuaValue])),
}

impl LuaValue {
    pub fn sum(a: &LuaValue, b: &LuaValue) -> LuaValue {
        match (a, b) {
            (&LuaValue::Number(n1), &LuaValue::Number(n2)) => LuaValue::Number(n1 + n2),
            _ => unimplemented!("cannot sum {} and {}", a.clone(), b.clone()),
        }
    }
}

impl PartialEq for LuaValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LuaValue::Number(a), LuaValue::Number(b)) => a == b,
            (LuaValue::String(a), LuaValue::String(b)) => a == b,
            (LuaValue::Boolean(a), LuaValue::Boolean(b)) => a == b,
            (LuaValue::Nil, LuaValue::Nil) => true,
            _ => panic!("Incompatible types: {}, {}", self, other),
        }
    }
}

impl From<LuaValue> for String {
    fn from(value: LuaValue) -> Self {
        match value {
            LuaValue::Nil => String::from("nil"),
            LuaValue::Boolean(b) => b.to_string(),
            LuaValue::String(s) => s,
            LuaValue::Number(n) => n.to_string(),
            _ => unimplemented!("{} is not convertible to a String type", value),
        }
    }
}

impl fmt::Display for LuaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaValue::Nil => write!(f, "nil"),
            LuaValue::Boolean(value) => write!(f, "{}", value),
            LuaValue::Number(value) => write!(f, "{}", value),
            LuaValue::String(value) => write!(f, "{}", value),
            _ => unimplemented!("Function can't be printed out"),
        }
    }
}

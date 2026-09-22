use core::panic;
use std::{
    cell::RefCell,
    fmt::{self},
    ops::{Div, Mul, Not, Rem, Sub},
    rc::Rc,
};

use crate::proto::Proto;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function(fn(&[LuaValue])),
    LuaFunction(Rc<Proto>),
    Table(Rc<RefCell<Vec<LuaValue>>>),
}

impl LuaValue {
    pub fn sum(a: &LuaValue, b: &LuaValue) -> LuaValue {
        match (a, b) {
            (&LuaValue::Number(n1), &LuaValue::Number(n2)) => LuaValue::Number(n1 + n2),
            _ => unimplemented!("cannot sum {} and {}", a.clone(), b.clone()),
        }
    }

    pub fn pow(&self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a.powf(b)),
            _ => unimplemented!("Incompatible types"),
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

impl PartialOrd for LuaValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (LuaValue::Number(a), LuaValue::Number(b)) => a.partial_cmp(b),

            (LuaValue::String(a), LuaValue::String(b)) => a.partial_cmp(b),

            _ => None,
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
            LuaValue::Table(_content) => {
                let a = self as *const LuaValue;
                write!(f, "table: {:p}", a)
            }
            _ => unimplemented!("Function can't be printed out"),
        }
    }
}

impl Sub for LuaValue {
    type Output = LuaValue;
    fn sub(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a - b),
            _ => unimplemented!("Cannot subtract incompatible types"),
        }
    }
}

impl Div for LuaValue {
    type Output = LuaValue;
    fn div(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a / b),
            _ => unimplemented!("Cannot subtract incompatible types"),
        }
    }
}

impl Mul for LuaValue {
    type Output = LuaValue;
    fn mul(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a * b),
            _ => unimplemented!("Cannot subtract incompatible types"),
        }
    }
}

impl Rem for LuaValue {
    type Output = LuaValue;
    fn rem(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a % b),
            _ => unimplemented!("Cannot subtract incompatible types"),
        }
    }
}

impl Not for LuaValue {
    type Output = LuaValue;
    fn not(self) -> LuaValue {
        match self {
            LuaValue::Boolean(b) => LuaValue::Boolean(!b),
            _ => unimplemented!("Cannot subtract incompatible types"),
        }
    }
}

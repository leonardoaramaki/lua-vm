use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self},
    hash::Hash,
    ops::{Add, Div, Mul, Not, Rem, Sub},
    rc::Rc,
};

use crate::proto::Proto;

type NativeFn = fn(&[LuaValue]) -> anyhow::Result<Vec<LuaValue>>;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function(NativeFn),
    LuaFunction(Rc<Proto>),
    Table(
        Rc<RefCell<Vec<LuaValue>>>,
        Rc<RefCell<HashMap<LuaValue, LuaValue>>>,
    ),
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
            (LuaValue::LuaFunction(a), LuaValue::LuaFunction(b)) => Rc::ptr_eq(a, b),
            (LuaValue::Table(_v1, h1), LuaValue::Table(_v2, h2)) => Rc::ptr_eq(h1, h2),
            (_, _) => false,
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
            LuaValue::Table(_v, _h) => {
                let a = self as *const LuaValue;
                write!(f, "table: {:p}", a)
            }
            LuaValue::LuaFunction(proto) => write!(f, "function: {:p}", proto),
            _ => unimplemented!("Error: this type can't be displayed"),
        }
    }
}

impl Add for LuaValue {
    type Output = LuaValue;
    fn add(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a + b),
            _ => unimplemented!("Cannot add incompatible types"),
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
            _ => unimplemented!("Cannot divide incompatible types"),
        }
    }
}

impl Mul for LuaValue {
    type Output = LuaValue;
    fn mul(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a * b),
            _ => unimplemented!("Cannot multiply incompatible types"),
        }
    }
}

impl Rem for LuaValue {
    type Output = LuaValue;
    fn rem(self, rhs: Self) -> LuaValue {
        match (self, rhs) {
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a % b),
            _ => unimplemented!("Cannot rem incompatible types"),
        }
    }
}

impl Not for LuaValue {
    type Output = LuaValue;
    fn not(self) -> LuaValue {
        match self {
            LuaValue::Boolean(b) => LuaValue::Boolean(!b),
            _ => unimplemented!("Cannot not incompatible types"),
        }
    }
}

impl Hash for LuaValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            LuaValue::Number(n) => n.to_bits().hash(state),
            LuaValue::String(s) => s.hash(state),
            LuaValue::Boolean(b) => b.hash(state),
            LuaValue::Table(_v, h) => Rc::as_ptr(h).hash(state),
            LuaValue::LuaFunction(proto) => Rc::as_ptr(proto).hash(state),
            _ => {
                dbg!(&self);
                unimplemented!()
            }
        }
    }
}

impl Eq for LuaValue {}

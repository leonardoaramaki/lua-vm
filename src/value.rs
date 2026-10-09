use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self},
    hash::Hash,
    ops::{Add, Div, Mul, Not, Rem, Sub},
    rc::Rc,
};

use crate::proto::Proto;

pub type NativeFn = Rc<dyn Fn(&[LuaValue]) -> anyhow::Result<Vec<LuaValue>>>;

/// A variable captured by a closure. While the enclosing function is running it
/// still lives on the VM stack (`Open` holds its absolute slot); once that function
/// returns, the value is moved into the upvalue itself (`Closed`).
pub enum Upval {
    Open(usize),
    Closed(LuaValue),
}

/// A Lua function value: its prototype plus the upvalues it captured.
pub struct LuaFunc {
    pub proto: Rc<Proto>,
    pub upvals: Vec<Rc<RefCell<Upval>>>,
}

#[derive(Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function(NativeFn),
    LuaFunction(Rc<LuaFunc>),
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

    /// Lua truthiness: only nil and false are false.
    pub fn truthy(&self) -> bool {
        !matches!(self, LuaValue::Nil | LuaValue::Boolean(false))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            LuaValue::Nil => "nil",
            LuaValue::Boolean(_) => "boolean",
            LuaValue::Number(_) => "number",
            LuaValue::String(_) => "string",
            LuaValue::Function(_) | LuaValue::LuaFunction(_) => "function",
            LuaValue::Table(..) => "table",
        }
    }

    /// t[key], or nil when the key is absent. Errors if `self` is not a table.
    pub fn index(&self, key: &LuaValue) -> anyhow::Result<LuaValue> {
        let LuaValue::Table(v, h) = self else {
            anyhow::bail!("attempt to index a {} value", self.type_name());
        };
        if let Some(i) = array_index(key) {
            if let Some(value) = v.borrow().get(i) {
                return Ok(value.clone());
            }
        }
        Ok(h.borrow().get(key).cloned().unwrap_or(LuaValue::Nil))
    }

    /// t[key] = value. Integer keys that extend the array part are appended to
    /// it (pulling any following keys out of the hash part), so `#t` and
    /// `ipairs` see tables built with `t[#t + 1] = x`.
    pub fn set_index(&self, key: LuaValue, value: LuaValue) -> anyhow::Result<()> {
        let LuaValue::Table(v, h) = self else {
            anyhow::bail!("attempt to index a {} value", self.type_name());
        };
        if key == LuaValue::Nil {
            anyhow::bail!("table index is nil");
        }
        let mut v = v.borrow_mut();
        let mut h = h.borrow_mut();
        match array_index(&key) {
            Some(i) if i < v.len() => {
                v[i] = value;
                // Keep the array part free of trailing nils so its length is a border.
                while v.last() == Some(&LuaValue::Nil) {
                    v.pop();
                }
            }
            Some(i) if i == v.len() && value != LuaValue::Nil => {
                h.remove(&key);
                v.push(value);
                while let Some(next) = h.remove(&LuaValue::Number((v.len() + 1) as f64)) {
                    v.push(next);
                }
            }
            _ if value == LuaValue::Nil => {
                h.remove(&key);
            }
            _ => {
                h.insert(key, value);
            }
        }
        Ok(())
    }

    /// The length operator `#`.
    pub fn len(&self) -> anyhow::Result<usize> {
        match self {
            LuaValue::String(s) => Ok(s.len()),
            LuaValue::Table(v, _) => Ok(v.borrow().len()),
            other => anyhow::bail!("attempt to get length of a {} value", other.type_name()),
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
            (LuaValue::Function(a), LuaValue::Function(b)) => Rc::ptr_eq(a, b),
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
            LuaValue::Number(n) => format_number(n),
            _ => unimplemented!("{} is not convertible to a String type", value),
        }
    }
}

impl fmt::Display for LuaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaValue::Nil => write!(f, "nil"),
            LuaValue::Boolean(value) => write!(f, "{}", value),
            LuaValue::Number(value) => write!(f, "{}", format_number(*value)),
            LuaValue::String(value) => write!(f, "{}", value),
            LuaValue::Table(_v, h) => write!(f, "table: {:p}", Rc::as_ptr(h)),
            LuaValue::LuaFunction(func) => write!(f, "function: {:p}", Rc::as_ptr(func)),
            LuaValue::Function(func) => write!(f, "builtin: {:p}", Rc::as_ptr(func)),
        }
    }
}

impl fmt::Debug for LuaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaValue::String(value) => write!(f, "{:?}", value),
            other => write!(f, "{}", other),
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
            // Lua's modulo takes the sign of the divisor: -1 % 16 == 15.
            (LuaValue::Number(a), LuaValue::Number(b)) => LuaValue::Number(a - (a / b).floor() * b),
            _ => unimplemented!("Cannot rem incompatible types"),
        }
    }
}

impl Not for LuaValue {
    type Output = LuaValue;
    fn not(self) -> LuaValue {
        LuaValue::Boolean(!self.truthy())
    }
}

impl Hash for LuaValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            LuaValue::Number(n) => n.to_bits().hash(state),
            LuaValue::String(s) => s.hash(state),
            LuaValue::Boolean(b) => b.hash(state),
            LuaValue::Table(_v, h) => Rc::as_ptr(h).hash(state),
            LuaValue::LuaFunction(func) => Rc::as_ptr(func).hash(state),
            LuaValue::Function(func) => Rc::as_ptr(func).cast::<()>().hash(state),
            LuaValue::Nil => 0.hash(state),
        }
    }
}

impl Eq for LuaValue {}

/// The 0-based array slot for an integer key >= 1.
fn array_index(key: &LuaValue) -> Option<usize> {
    match key {
        LuaValue::Number(n) if n.fract() == 0.0 && *n >= 1.0 => Some(*n as usize - 1),
        _ => None,
    }
}

/// Formats numbers like Lua 5.1's "%.14g": integers without a decimal point,
/// other values with at most 14 significant digits.
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    if !n.is_finite() {
        return if n.is_nan() { "nan".into() } else if n > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let exp = n.abs().log10().floor() as i32;
    if !(-5..14).contains(&exp) {
        let s = format!("{:.13e}", n);
        let (mantissa, exp) = s.split_once('e').unwrap();
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        let exp: i32 = exp.parse().unwrap();
        return format!("{}e{}{:02}", mantissa, if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let decimals = (13 - exp).max(0) as usize;
    let s = format!("{:.*}", decimals, n);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

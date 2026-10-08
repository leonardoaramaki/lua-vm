use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Ok;

use crate::value::LuaValue;

pub fn math_module() -> LuaValue {
    let mut h = HashMap::new();
    h.insert(
        LuaValue::String("floor".into()),
        LuaValue::Function(Rc::new(math_floor)),
    );
    h.insert(
        LuaValue::String("sin".into()),
        LuaValue::Function(Rc::new(math_sin)),
    );
    LuaValue::Table(Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(h)))
}

pub fn math_floor(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let Some(LuaValue::Number(n)) = args.first() else {
        anyhow::bail!("math::floor(n) where n is not a number");
    };
    Ok(vec![LuaValue::Number(n.floor())])
}

pub fn math_sin(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let Some(LuaValue::Number(n)) = args.first() else {
        anyhow::bail!("math::sin(n) where n is not a number");
    };
    Ok(vec![LuaValue::Number(n.sin())])
}

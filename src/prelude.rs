use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::value::LuaValue;

pub fn lua_prelude_print(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    if args.is_empty() {
        return Ok(vec![]);
    }
    for arg in args {
        print!("{}\t", arg);
    }
    println!();
    Ok(vec![])
}

fn table_arg<'a>(args: &'a [LuaValue], name: &str) -> anyhow::Result<&'a LuaValue> {
    match args.first() {
        Some(t @ LuaValue::Table(..)) => Ok(t),
        other => anyhow::bail!(
            "bad argument #1 to '{}' (table expected, got {})",
            name,
            other.map_or("no value", LuaValue::type_name)
        ),
    }
}

/// next(t, k): the entry after `k` (or the first one when `k` is nil). The array
/// part comes first, in order, then the hash part.
pub fn lua_next(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let LuaValue::Table(v, h) = table_arg(args, "next")? else {
        unreachable!()
    };
    let key = args.get(1).cloned().unwrap_or(LuaValue::Nil);
    let (v, h) = (v.borrow(), h.borrow());
    let array_start = match &key {
        LuaValue::Nil => Some(0),
        LuaValue::Number(n) if n.fract() == 0.0 && *n >= 1.0 && *n as usize <= v.len() => {
            Some(*n as usize)
        }
        _ => None,
    };
    let mut entries = h.iter();
    if let Some(start) = array_start {
        if let Some(i) = (start..v.len()).find(|&i| v[i] != LuaValue::Nil) {
            return Ok(vec![LuaValue::Number((i + 1) as f64), v[i].clone()]);
        }
    } else if entries.by_ref().find(|(k, _)| **k == key).is_none() {
        anyhow::bail!("invalid key to 'next'");
    }
    Ok(match entries.next() {
        Some((k, value)) => vec![k.clone(), value.clone()],
        None => vec![LuaValue::Nil],
    })
}

/// pairs(t) returns next, t, nil.
pub fn lua_pairs(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let t = table_arg(args, "pairs")?.clone();
    Ok(vec![LuaValue::Function(Rc::new(lua_next)), t, LuaValue::Nil])
}

fn ipairs_aux(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let t = table_arg(args, "ipairs")?;
    let i = match args.get(1) {
        Some(LuaValue::Number(n)) => *n + 1.0,
        _ => 1.0,
    };
    let value = t.index(&LuaValue::Number(i))?;
    Ok(if value == LuaValue::Nil {
        vec![LuaValue::Nil]
    } else {
        vec![LuaValue::Number(i), value]
    })
}

/// ipairs(t) returns an iterator over t[1], t[2], ... up to the first nil.
pub fn lua_ipairs(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let t = table_arg(args, "ipairs")?.clone();
    Ok(vec![LuaValue::Function(Rc::new(ipairs_aux)), t, LuaValue::Number(0.0)])
}

pub fn table_module() -> LuaValue {
    let mut h = HashMap::new();
    h.insert(
        LuaValue::String("insert".into()),
        LuaValue::Function(Rc::new(table_insert)),
    );
    h.insert(
        LuaValue::String("remove".into()),
        LuaValue::Function(Rc::new(table_remove)),
    );
    LuaValue::Table(Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(h)))
}

/// table.insert(t, value) appends; table.insert(t, pos, value) shifts t[pos..] up.
fn table_insert(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let t = table_arg(args, "insert")?;
    let len = t.len()?;
    match args {
        [_, value] => t.set_index(LuaValue::Number((len + 1) as f64), value.clone())?,
        [_, LuaValue::Number(pos), value] => {
            if pos.fract() != 0.0 || *pos < 1.0 || *pos as usize > len + 1 {
                anyhow::bail!("bad argument #2 to 'insert' (position out of bounds)");
            }
            let LuaValue::Table(v, _) = t else { unreachable!() };
            v.borrow_mut().insert(*pos as usize - 1, value.clone());
        }
        _ => anyhow::bail!("wrong number of arguments to 'insert'"),
    }
    Ok(vec![])
}

/// table.remove(t [, pos]) removes and returns t[pos] (default: the last element).
fn table_remove(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let t = table_arg(args, "remove")?;
    let len = t.len()?;
    let pos = match args.get(1) {
        Some(LuaValue::Number(n)) => *n,
        _ => len as f64,
    };
    if len == 0 && (pos == 0.0 || pos == len as f64) {
        return Ok(vec![LuaValue::Nil]);
    }
    if pos.fract() != 0.0 || pos < 1.0 || pos as usize > len {
        anyhow::bail!("bad argument #2 to 'remove' (position out of bounds)");
    }
    let LuaValue::Table(v, _) = t else { unreachable!() };
    let mut v = v.borrow_mut();
    let removed = v.remove(pos as usize - 1);
    while v.last() == Some(&LuaValue::Nil) {
        v.pop();
    }
    Ok(vec![removed])
}

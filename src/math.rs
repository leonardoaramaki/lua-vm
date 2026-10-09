use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

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
    h.insert(
        LuaValue::String("abs".into()),
        LuaValue::Function(Rc::new(math_abs)),
    );
    h.insert(
        LuaValue::String("sqrt".into()),
        LuaValue::Function(Rc::new(math_sqrt)),
    );
    h.insert(
        LuaValue::String("max".into()),
        LuaValue::Function(Rc::new(math_max)),
    );
    h.insert(
        LuaValue::String("min".into()),
        LuaValue::Function(Rc::new(math_min)),
    );

    // randomseed and random share one generator state.
    let state = Rc::new(Cell::new(mix(now_nanos().unwrap_or(0)) | 1));

    let s = state.clone();
    h.insert(
        LuaValue::String("randomseed".into()),
        LuaValue::Function(Rc::new(move |args| {
            let seed = match args.first() {
                Some(LuaValue::Number(n)) => *n as i64 as u64,
                // No argument (or nil): random seed, like Lua 5.4.
                _ => now_nanos()?,
            };
            // xorshift gets stuck at 0, so keep the state odd.
            s.set(mix(seed) | 1);
            Ok(vec![])
        })),
    );

    let s = state.clone();
    h.insert(
        LuaValue::String("random".into()),
        LuaValue::Function(Rc::new(move |args| {
            let unit = (next(&s) >> 11) as f64 / (1u64 << 53) as f64;
            let (lo, hi) = match args {
                [] => return Ok(vec![LuaValue::Number(unit)]),
                [LuaValue::Number(m)] => (1.0, m.floor()),
                [LuaValue::Number(m), LuaValue::Number(n), ..] => (m.floor(), n.floor()),
                _ => anyhow::bail!("math.random(m, n) where m or n is not a number"),
            };
            if lo > hi {
                anyhow::bail!("math.random: interval is empty");
            }
            Ok(vec![LuaValue::Number(lo + (unit * (hi - lo + 1.0)).floor())])
        })),
    );

    LuaValue::Table(Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(h)))
}

pub fn math_abs(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let Some(LuaValue::Number(n)) = args.first() else {
        anyhow::bail!("math::abs(n) where n is not a number");
    };
    Ok(vec![LuaValue::Number(n.abs())])
}

pub fn math_sqrt(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let Some(LuaValue::Number(n)) = args.first() else {
        anyhow::bail!("math::sqrt(n) where n is not a number");
    };
    Ok(vec![LuaValue::Number(n.sqrt())])
}

/// The numbers in `args`, at least one; `name` is used in error messages.
fn numbers(args: &[LuaValue], name: &str) -> anyhow::Result<Vec<f64>> {
    if args.is_empty() {
        anyhow::bail!("math::{}() needs at least one number", name);
    }
    args.iter()
        .map(|arg| match arg {
            LuaValue::Number(n) => Ok(*n),
            _ => anyhow::bail!("math::{}() where an argument is not a number", name),
        })
        .collect()
}

pub fn math_max(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let max = numbers(args, "max")?.into_iter().fold(f64::NEG_INFINITY, f64::max);
    Ok(vec![LuaValue::Number(max)])
}

pub fn math_min(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let min = numbers(args, "min")?.into_iter().fold(f64::INFINITY, f64::min);
    Ok(vec![LuaValue::Number(min)])
}

fn now_nanos() -> anyhow::Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64)
}

// splitmix64: spreads nearby seeds (0, 1, 2...) into very different states.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

// xorshift64: advances the state and returns the next number.
fn next(state: &Cell<u64>) -> u64 {
    let mut x = state.get();
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.set(x);
    x
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

#[cfg(test)]
mod tests {
    use super::*;

    fn call(math: &LuaValue, name: &str, args: &[LuaValue]) -> Vec<LuaValue> {
        let LuaValue::Table(_, h) = math else { panic!("math is not a table") };
        let Some(LuaValue::Function(f)) = h.borrow().get(&LuaValue::String(name.into())).cloned()
        else {
            panic!("math.{name} is missing");
        };
        f(args).unwrap()
    }

    fn sequence(math: &LuaValue, args: &[LuaValue]) -> Vec<f64> {
        (0..10)
            .map(|_| match call(math, "random", args)[0] {
                LuaValue::Number(n) => n,
                _ => panic!("math.random did not return a number"),
            })
            .collect()
    }

    #[test]
    fn same_seed_repeats_the_sequence() {
        let math = math_module();
        call(&math, "randomseed", &[LuaValue::Number(42.0)]);
        let first = sequence(&math, &[]);
        call(&math, "randomseed", &[LuaValue::Number(42.0)]);
        assert_eq!(first, sequence(&math, &[]));
    }

    #[test]
    fn nearby_seeds_give_different_sequences() {
        let math = math_module();
        call(&math, "randomseed", &[LuaValue::Number(0.0)]);
        let zero = sequence(&math, &[]);
        call(&math, "randomseed", &[LuaValue::Number(1.0)]);
        let one = sequence(&math, &[]);
        assert_ne!(zero, one);
        // Seed 0 must not leave xorshift stuck at 0.
        assert!(zero.iter().any(|&n| n != 0.0));
    }

    #[test]
    fn random_respects_the_ranges() {
        let math = math_module();
        call(&math, "randomseed", &[LuaValue::Number(7.0)]);
        for _ in 0..1000 {
            let unit = sequence(&math, &[]);
            assert!(unit.iter().all(|&n| (0.0..1.0).contains(&n)));
            let m = sequence(&math, &[LuaValue::Number(6.0)]);
            assert!(m.iter().all(|&n| n.fract() == 0.0 && (1.0..=6.0).contains(&n)));
            let mn = sequence(&math, &[LuaValue::Number(-3.0), LuaValue::Number(3.0)]);
            assert!(mn.iter().all(|&n| n.fract() == 0.0 && (-3.0..=3.0).contains(&n)));
        }
    }

    #[test]
    fn empty_interval_is_an_error() {
        let math = math_module();
        let LuaValue::Table(_, h) = &math else { unreachable!() };
        let Some(LuaValue::Function(f)) = h.borrow().get(&LuaValue::String("random".into())).cloned()
        else {
            unreachable!()
        };
        assert!(f(&[LuaValue::Number(5.0), LuaValue::Number(1.0)]).is_err());
    }
}

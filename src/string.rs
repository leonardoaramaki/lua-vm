use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::value::LuaValue;

pub fn string_module() -> LuaValue {
    let mut h = HashMap::new();
    h.insert(
        LuaValue::String("format".into()),
        LuaValue::Function(Rc::new(string_format)),
    );
    LuaValue::Table(Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(h)))
}

/// One `%` directive: flags, width, precision and the conversion letter.
#[derive(Default)]
struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
    width: usize,
    precision: Option<usize>,
}

/// string.format(fmt, ...), following C's printf like Lua 5.1 does.
pub fn string_format(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let fmt = match args.first() {
        Some(LuaValue::String(s)) => s.clone(),
        Some(LuaValue::Number(n)) => crate::value::format_number(*n),
        other => anyhow::bail!(
            "bad argument #1 to 'format' (string expected, got {})",
            other.map_or("no value", LuaValue::type_name)
        ),
    };
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut next_arg = 1;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        let mut spec = Spec::default();
        while let Some(&flag) = chars.peek() {
            match flag {
                '-' => spec.left = true,
                '+' => spec.plus = true,
                ' ' => spec.space = true,
                '#' => spec.alt = true,
                '0' => spec.zero = true,
                _ => break,
            }
            chars.next();
        }
        spec.width = read_digits(&mut chars).unwrap_or(0);
        if chars.peek() == Some(&'.') {
            chars.next();
            spec.precision = Some(read_digits(&mut chars).unwrap_or(0));
        }
        let Some(conversion) = chars.next() else {
            anyhow::bail!("invalid option '%' to 'format'");
        };
        let arg_index = next_arg;
        next_arg += 1;
        let arg = args.get(arg_index);
        let number = || -> anyhow::Result<f64> {
            match arg {
                Some(LuaValue::Number(n)) => Ok(*n),
                Some(LuaValue::String(s)) if s.trim().parse::<f64>().is_ok() => {
                    Ok(s.trim().parse().unwrap())
                }
                other => anyhow::bail!(
                    "bad argument #{} to 'format' (number expected, got {})",
                    arg_index + 1,
                    other.map_or("no value", LuaValue::type_name)
                ),
            }
        };
        let piece = match conversion {
            'd' | 'i' => format_integer(number()?.trunc() as i64, &spec),
            'u' => format_unsigned(number()?.trunc() as i64 as u64, 10, false, &spec, ""),
            'x' => format_unsigned(number()?.trunc() as i64 as u64, 16, false, &spec, "0x"),
            'X' => format_unsigned(number()?.trunc() as i64 as u64, 16, true, &spec, "0X"),
            'o' => format_unsigned(number()?.trunc() as i64 as u64, 8, false, &spec, "0"),
            'c' => pad(&((number()? as u8) as char).to_string(), &spec, false),
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => format_float(number()?, conversion, &spec),
            's' => {
                let Some(value) = arg else {
                    anyhow::bail!("bad argument #{} to 'format' (no value)", arg_index + 1);
                };
                let mut s = value.to_string();
                if let Some(p) = spec.precision {
                    s = s.chars().take(p).collect();
                }
                pad(&s, &spec, false)
            }
            'q' => {
                let Some(LuaValue::String(s)) = arg else {
                    anyhow::bail!("bad argument #{} to 'format' (string expected)", arg_index + 1);
                };
                quote(s)
            }
            other => anyhow::bail!("invalid option '%{}' to 'format'", other),
        };
        out.push_str(&piece);
    }
    Ok(vec![LuaValue::String(out)])
}

fn read_digits(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<usize> {
    let mut digits = String::new();
    while let Some(&c) = chars.peek().filter(|c| c.is_ascii_digit()) {
        digits.push(c);
        chars.next();
    }
    digits.parse().ok()
}

/// Pads `body` to the field width. Zero padding goes after the sign or prefix.
fn pad(body: &str, spec: &Spec, numeric: bool) -> String {
    let len = body.chars().count();
    if len >= spec.width {
        return body.to_string();
    }
    let fill = spec.width - len;
    if spec.left {
        format!("{}{}", body, " ".repeat(fill))
    } else if spec.zero && numeric {
        let split = body
            .find(|c: char| c.is_ascii_digit())
            .filter(|&i| !body[i..].starts_with("0x") && !body[i..].starts_with("0X"))
            .unwrap_or_else(|| body.find(['x', 'X']).map_or(0, |i| i + 1));
        format!("{}{}{}", &body[..split], "0".repeat(fill), &body[split..])
    } else {
        format!("{}{}", " ".repeat(fill), body)
    }
}

fn sign(negative: bool, spec: &Spec) -> &'static str {
    if negative {
        "-"
    } else if spec.plus {
        "+"
    } else if spec.space {
        " "
    } else {
        ""
    }
}

fn format_integer(n: i64, spec: &Spec) -> String {
    let mut digits = n.unsigned_abs().to_string();
    if let Some(p) = spec.precision {
        if p == 0 && n == 0 {
            digits.clear();
        } else if digits.len() < p {
            digits = format!("{}{}", "0".repeat(p - digits.len()), digits);
        }
    }
    let body = format!("{}{}", sign(n < 0, spec), digits);
    // A precision turns off zero padding, like in C.
    let numeric = spec.precision.is_none();
    pad(&body, spec, numeric)
}

fn format_unsigned(n: u64, radix: u32, upper: bool, spec: &Spec, prefix: &str) -> String {
    let mut digits = match radix {
        16 if upper => format!("{:X}", n),
        16 => format!("{:x}", n),
        8 => format!("{:o}", n),
        _ => n.to_string(),
    };
    if let Some(p) = spec.precision {
        if p == 0 && n == 0 {
            digits.clear();
        } else if digits.len() < p {
            digits = format!("{}{}", "0".repeat(p - digits.len()), digits);
        }
    }
    let prefix = if spec.alt && n != 0 && !digits.starts_with('0') { prefix } else { "" };
    pad(&format!("{}{}", prefix, digits), spec, spec.precision.is_none())
}

/// `%e`: d.ddde+XX, with at least two exponent digits.
fn exponential(n: f64, precision: usize, upper: bool) -> String {
    let s = format!("{:.*e}", precision, n);
    let (mantissa, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let e = if upper { 'E' } else { 'e' };
    format!("{}{}{}{:02}", mantissa, e, if exp < 0 { '-' } else { '+' }, exp.abs())
}

fn format_float(n: f64, conversion: char, spec: &Spec) -> String {
    let upper = conversion.is_ascii_uppercase();
    let body = if !n.is_finite() {
        let s = if n.is_nan() { "nan" } else { "inf" };
        let s = if upper { s.to_uppercase() } else { s.to_string() };
        let body = format!("{}{}", sign(n.is_sign_negative() && !n.is_nan(), spec), s);
        return pad(&body, spec, false);
    } else {
        let precision = spec.precision.unwrap_or(6);
        let a = n.abs();
        let digits = match conversion {
            'f' | 'F' => format!("{:.*}", precision, a),
            'e' | 'E' => exponential(a, precision, upper),
            _ => {
                // %g: %e or %f, whichever is shorter, without trailing zeros.
                let p = precision.max(1);
                let exp = if a == 0.0 {
                    0
                } else {
                    let e = exponential(a, p - 1, false);
                    e.split_once('e').unwrap().1.parse::<i32>().unwrap()
                };
                let mut s = if exp < -4 || exp >= p as i32 {
                    exponential(a, p - 1, upper)
                } else {
                    format!("{:.*}", (p as i32 - 1 - exp).max(0) as usize, a)
                };
                if !spec.alt {
                    s = strip_trailing_zeros(&s);
                }
                s
            }
        };
        let digits = if spec.alt && !digits.contains('.') && !digits.contains(['e', 'E']) {
            format!("{}.", digits)
        } else {
            digits
        };
        format!("{}{}", sign(n.is_sign_negative() && n != 0.0 || n < 0.0, spec), digits)
    };
    pad(&body, spec, true)
}

/// Removes trailing zeros (and a trailing dot) from the fraction, keeping any exponent.
fn strip_trailing_zeros(s: &str) -> String {
    let (mantissa, exp) = match s.find(['e', 'E']) {
        Some(i) => s.split_at(i),
        None => (s, ""),
    };
    if !mantissa.contains('.') {
        return s.to_string();
    }
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    format!("{}{}", mantissa, exp)
}

/// `%q`: a string literal that Lua can read back.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\000"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(f: &str, args: &[LuaValue]) -> String {
        let mut all = vec![LuaValue::String(f.into())];
        all.extend_from_slice(args);
        match &string_format(&all).unwrap()[0] {
            LuaValue::String(s) => s.clone(),
            _ => unreachable!(),
        }
    }

    fn n(x: f64) -> LuaValue {
        LuaValue::Number(x)
    }

    #[test]
    fn integers() {
        assert_eq!(fmt("%02d:%02d", &[n(1.0), n(5.0)]), "01:05");
        assert_eq!(fmt("%d", &[n(-42.9)]), "-42");
        assert_eq!(fmt("%5d|%-5d|%05d", &[n(42.0), n(42.0), n(-42.0)]), "   42|42   |-0042");
        assert_eq!(fmt("%+d % d", &[n(3.0), n(3.0)]), "+3  3");
        assert_eq!(fmt("%.3d", &[n(7.0)]), "007");
        assert_eq!(fmt("%x %X %#x %o", &[n(255.0), n(255.0), n(255.0), n(8.0)]), "ff FF 0xff 10");
        assert_eq!(fmt("%c%c", &[n(72.0), n(105.0)]), "Hi");
    }

    #[test]
    fn floats() {
        assert_eq!(fmt("%0.2f kb", &[n(1234.5678)]), "1234.57 kb");
        assert_eq!(fmt("%f", &[n(1.5)]), "1.500000");
        assert_eq!(fmt("%8.3f|%-8.1f|", &[n(-3.14159), n(2.0)]), "  -3.142|2.0     |");
        assert_eq!(fmt("%e", &[n(12345.678)]), "1.234568e+04");
        assert_eq!(fmt("%.2E", &[n(0.000123)]), "1.23E-04");
        assert_eq!(fmt("%g %g %g", &[n(100000.0), n(1e20), n(0.0001)]), "100000 1e+20 0.0001");
        assert_eq!(fmt("%g %g", &[n(0.5), n(1e-5)]), "0.5 1e-05");
        assert_eq!(fmt("%.3g", &[n(3.14159)]), "3.14");
        assert_eq!(fmt("%f", &[n(f64::INFINITY)]), "inf");
    }

    #[test]
    fn strings() {
        assert_eq!(fmt("[%s] [%5s] [%-5s] [%.2s]", &[
            LuaValue::String("x".into()),
            LuaValue::String("ab".into()),
            LuaValue::String("ab".into()),
            LuaValue::String("hello".into()),
        ]), "[x] [   ab] [ab   ] [he]");
        assert_eq!(fmt("%s %s %s", &[n(3.0), n(1.5), LuaValue::Boolean(true)]), "3 1.5 true");
        assert_eq!(fmt("%q", &[LuaValue::String("a\"b\\c".into())]), "\"a\\\"b\\\\c\"");
        assert_eq!(fmt("100%%", &[]), "100%");
    }

    #[test]
    fn numeric_strings_are_numbers() {
        assert_eq!(fmt("%d", &[LuaValue::String("12".into())]), "12");
    }

    #[test]
    fn errors() {
        let call = |args: Vec<LuaValue>| string_format(&args);
        let err = call(vec![LuaValue::String("%d".into())]).unwrap_err();
        assert!(err.to_string().contains("bad argument #2 to 'format' (number expected, got no value)"));
        let err = call(vec![LuaValue::String("%d".into()), LuaValue::Boolean(true)]).unwrap_err();
        assert!(err.to_string().contains("got boolean"));
        assert!(call(vec![LuaValue::String("%y".into()), n(1.0)]).is_err());
    }
}

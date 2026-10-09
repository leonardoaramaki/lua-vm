//! Runs the Lua programs in tests/lua. Each one reports through `check(ok, message)`.

use std::{cell::RefCell, path::Path, rc::Rc};

use lua_vm::{Lua, LuaValue};

/// Runs `tests/lua/<name>.lua` and returns the messages of failed checks.
fn run(name: &str) -> anyhow::Result<Vec<String>> {
    let failures = Rc::new(RefCell::new(Vec::new()));
    let checks = Rc::new(RefCell::new(0));
    let mut lua = Lua::new();
    let (f, c) = (failures.clone(), checks.clone());
    lua.set_global(
        "check",
        LuaValue::Function(Rc::new(move |args| {
            *c.borrow_mut() += 1;
            if !args.first().is_some_and(LuaValue::truthy) {
                let message = args.get(1).map(|m| m.to_string()).unwrap_or_default();
                f.borrow_mut().push(message);
            }
            Ok(vec![])
        })),
    )?;
    lua.exec_file(Path::new("tests/lua").join(name).with_extension("lua"))?;
    assert!(*checks.borrow() > 0, "{name}.lua ran no checks");
    Ok(failures.take())
}

macro_rules! lua_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let failures = run(stringify!($name)).unwrap();
            assert!(failures.is_empty(), "failed checks: {failures:#?}");
        }
    };
}

lua_test!(functions);
lua_test!(upvalues);
lua_test!(tables);
lua_test!(iteration);
lua_test!(logic);
lua_test!(require);
lua_test!(numbers);
lua_test!(strings);

#[test]
fn errors_carry_the_source_line() {
    let mut lua = Lua::new();
    let err = lua.exec_file("tests/lua/mods/broken.lua").unwrap_err();
    let message = format!("{err:#}");
    assert!(message.contains("broken.lua:3"), "{message}");
    assert!(message.contains("attempt to index a nil value"), "{message}");
}

#[test]
fn host_can_call_functions_and_get_results() {
    let mut lua = Lua::new();
    lua.exec_file("tests/lua/mods/host.lua").unwrap();
    let results = lua.call_global("add", &[LuaValue::Number(2.0), LuaValue::Number(3.0)]).unwrap();
    assert_eq!(results, vec![LuaValue::Number(5.0), LuaValue::String("ok".into())]);
    // The closure's state survives between host calls.
    lua.call_global("tick", &[]).unwrap();
    let results = lua.call_global("tick", &[]).unwrap();
    assert_eq!(results, vec![LuaValue::Number(2.0)]);
}

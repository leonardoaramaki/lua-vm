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

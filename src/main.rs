use lua_vm::Lua;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let Some(filename) = args.get(1) else {
        anyhow::bail!("Usage: {} <lua-file>", args[0]);
    };
    Lua::new().exec_file(filename)
}

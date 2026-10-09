use std::{
    cell::RefCell,
    io::Cursor,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    chunk::Chunk,
    prelude::{lua_ipairs, lua_next, lua_pairs, lua_prelude_print, table_module},
    proto::Proto,
    value::LuaFunc,
    vm::LuaVM,
};

mod chunk;
mod closure;
mod math;
mod prelude;
mod proto;
mod string;
mod value;
mod vm;

pub use value::{LuaValue, NativeFn};

/// Bytecode of src/prelude.lua, compiled by build.rs.
const PRELUDE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/prelude.luac"));

pub struct Lua {
    lua_vm: LuaVM,
    /// Where `require` looks for modules: the directory of the last executed file.
    module_dir: Rc<RefCell<PathBuf>>,
}

/// Turns compiled bytecode into a callable function value.
fn load_bytecode(bytecode: Vec<u8>) -> anyhow::Result<LuaValue> {
    let main_function = Chunk::new(Cursor::new(bytecode)).load()?;
    let proto = Rc::new(Proto::from(main_function));
    Ok(LuaValue::LuaFunction(Rc::new(LuaFunc {
        proto,
        upvals: vec![],
    })))
}

fn load_file(path: &Path) -> anyhow::Result<LuaValue> {
    let bytecode = luac::compile_file(path)
        .map_err(|e| anyhow::anyhow!("could not compile {}: {}", path.display(), e))?;
    load_bytecode(bytecode)
}

impl Lua {
    pub fn new() -> Self {
        let mut vm = LuaVM::new();
        let module_dir = Rc::new(RefCell::new(PathBuf::new()));
        {
            let mut env = vm.env.borrow_mut();
            env.insert_global("print", LuaValue::Function(Rc::new(lua_prelude_print)));
            env.insert_global("math", math::math_module());
            env.insert_global("table", table_module());
            env.insert_global("string", string::string_module());
            env.insert_global("next", LuaValue::Function(Rc::new(lua_next)));
            env.insert_global("pairs", LuaValue::Function(Rc::new(lua_pairs)));
            env.insert_global("ipairs", LuaValue::Function(Rc::new(lua_ipairs)));
            // require("a.b") loads <module_dir>/a/b.lua; the prelude adds the caching.
            let dir = module_dir.clone();
            env.insert_global(
                "__load_module",
                LuaValue::Function(Rc::new(move |args| {
                    let Some(LuaValue::String(name)) = args.first() else {
                        anyhow::bail!("bad argument #1 to 'require' (string expected)");
                    };
                    let path = dir.borrow().join(name.replace('.', "/")).with_extension("lua");
                    if !path.is_file() {
                        anyhow::bail!("module '{}' not found: no file '{}'", name, path.display());
                    }
                    Ok(vec![load_file(&path)?])
                })),
            );
        }
        let prelude = load_bytecode(PRELUDE.to_vec()).expect("embedded prelude must load");
        vm.call(prelude, &[]).expect("embedded prelude must run");
        Self {
            lua_vm: vm,
            module_dir,
        }
    }

    pub fn exec_file(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        *self.module_dir.borrow_mut() = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let main_function = load_file(path)?;
        self.lua_vm.call(main_function, &[])?;
        Ok(())
    }

    pub fn set_global(&mut self, k: &str, v: LuaValue) -> anyhow::Result<()> {
        self.lua_vm.env.borrow_mut().insert_global(k, v);
        Ok(())
    }

    pub fn get_global(&self, name: &str) -> Option<LuaValue> {
        self.lua_vm.env.borrow_mut().globals.get(name).cloned()
    }

    pub fn call_global(&mut self, name: &str, args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
        let func = self
            .lua_vm
            .env
            .borrow()
            .globals
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("global '{name}' is not defined"))?;
        self.lua_vm.call(func, args)
    }
}

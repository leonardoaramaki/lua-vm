use std::{io::Cursor, path::Path, rc::Rc};

use crate::{chunk::Chunk, prelude::lua_prelude_print, proto::Proto, vm::LuaVM};

mod chunk;
mod closure;
mod math;
mod prelude;
mod proto;
mod value;
mod vm;

pub use value::{LuaValue, NativeFn};

pub struct Lua {
    lua_vm: LuaVM,
}

impl Lua {
    pub fn new() -> Self {
        let vm = LuaVM::new();
        vm.env
            .borrow_mut()
            .insert_global("print", LuaValue::Function(Rc::new(lua_prelude_print)));
        vm.env
            .borrow_mut()
            .insert_global("math", math::math_module());
        Self { lua_vm: vm }
    }

    pub fn exec_file(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let Ok(bytecode) = luac::compile_file(path) else {
            anyhow::bail!("Could not generate bytecode for {}", path.display());
        };
        let main_function = Chunk::new(Cursor::new(bytecode)).load()?;
        let proto = Proto::new(
            main_function.instructions,
            main_function.constants,
            main_function
                .protos
                .into_iter()
                .map(|x| Rc::new(Proto::from(x)))
                .collect(),
        );
        self.lua_vm.load_proto(proto);
        while self.lua_vm.step().is_some() {}
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

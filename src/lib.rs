use std::{io::Cursor, path::Path, rc::Rc};

use crate::{chunk::Chunk, prelude::lua_prelude_print, proto::Proto, value::LuaValue, vm::LuaVM};

mod chunk;
mod closure;
mod math;
mod prelude;
mod proto;
mod value;
mod vm;

pub struct Lua {
    lua_vm: LuaVM,
}

impl Lua {
    pub fn new() -> Self {
        let vm = LuaVM::new();
        Lua::add_global(&vm, "print", LuaValue::Function(lua_prelude_print));
        Lua::add_global(&vm, "math", math::math_module());
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

    fn add_global(vm: &LuaVM, k: &str, v: LuaValue) {
        vm.env.borrow_mut().insert_global(k, v);
    }
}

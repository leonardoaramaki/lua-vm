use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use anyhow::Ok;

use crate::chunk::Chunk;
use crate::math::math_module;
use crate::proto::*;
use crate::value::LuaValue;
use crate::vm::LuaVM;

mod chunk;
mod closure;
/// Built-in modules
mod math;
mod proto;
mod value;
mod vm;

fn print(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    if args.is_empty() {
        return Ok(vec![]);
    }
    for arg in args {
        print!("{}\t", arg);
    }
    println!();
    Ok(vec![])
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let file = args
        .iter()
        .skip(1)
        .find(|arg| Path::new(arg).extension().is_some_and(|x| x == "out"))
        .and_then(|f| File::open(f).ok());
    let mut chunk = match file {
        Some(f) => Chunk::new(f),
        None => {
            eprintln!("Error: should provide a file name with .out extension");
            std::process::exit(1);
        }
    };
    let main_function = chunk.load()?;
    let mut vm = LuaVM::new();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    vm.env.borrow_mut().insert_global("math", math_module());
    let proto = Proto::new(
        main_function.instructions,
        main_function.constants,
        main_function
            .protos
            .into_iter()
            .map(|x| Rc::new(Proto::from(x)))
            .collect(),
    );
    vm.load_proto(proto);
    while vm.step().is_some() {}
    Ok(())
}

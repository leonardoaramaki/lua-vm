use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use anyhow::Ok;

use crate::chunk::Chunk;
use crate::proto::*;
use crate::value::LuaValue;
use crate::vm::LuaVM;

mod chunk;
mod closure;
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

fn dobro(args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
    let Some(LuaValue::Number(n)) = args.first() else {
        anyhow::bail!("dobro: not a number")
    };
    Ok(vec![LuaValue::Number(n * 2.0)])
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
    vm.env
        .borrow_mut()
        .insert_global("dobro", LuaValue::Function(dobro));
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

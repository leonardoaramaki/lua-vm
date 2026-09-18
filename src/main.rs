use std::rc::Rc;

use crate::proto::*;
use crate::value::LuaValue;
use crate::vm::LuaVM;

mod closure;
mod proto;
mod value;
mod vm;

fn print(args: &[LuaValue]) {
    if args.is_empty() {
        return;
    }
    println!("{}", args[0]);
}

fn main() {
    // Proto 0: function imprime(msg) -- "msg" arrives in R0 (the callee's
    // own base is always 0 in this VM, so the caller must place the
    // argument at absolute R0 before entering this proto).
    let imprime_bytecode: Vec<u32> = vec![
        0x00000045, // PC 0: GETGLOBAL R1, K0     ; R1 = print
        0x00000080, // PC 1: MOVE      R2, R0     ; R2 = msg
        0x0100405D, // PC 2: TAILCALL  R1, 2, 1   ; print(msg)
        0x0080001E, // PC 3: RETURN    R0, 1
    ];

    // Proto 1: the main chunk.
    let main_bytecode: Vec<u32> = vec![
        0x00000024, // PC 0: CLOSURE   R0, P0     ; R0 = new closure of proto 0
        0x00000007, // PC 1: SETGLOBAL R0, K0     ; imprime = R0
        0x00000045, // PC 2: GETGLOBAL R1, K0     ; R1 = imprime
        0x00004081, // PC 3: LOADK     R2, K1     ; R2 = "Olá Leo"
        0x0100405D, // PC 4: TAILCALL  R1, 2, 1   ; imprime("Olá Leo")
        0x0080001E, // PC 5: RETURN    R0, 1
    ];

    let mut vm = LuaVM::new();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    let imprime_proto = Proto::new(
        imprime_bytecode,
        vec![LuaValue::String(String::from("print"))], // K0
        vec![],
    );
    let main_constants = vec![
        LuaValue::String(String::from("imprime")), // K0
        LuaValue::String(String::from("Olá Leo")), // K1
    ];
    let proto = Proto::new(
        main_bytecode,
        main_constants.clone(),
        vec![Rc::new(imprime_proto)],
    );
    vm.load_proto(proto);
    while vm.step().is_some() {}
}

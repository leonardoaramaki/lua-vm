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
    // local t = { x = 10, y = 20 }
    // print(t.x) --> 10
    // print(t.y) --> 20
    let main_bytecode: Vec<u32> = vec![
        0x0000800A, // PC 0: NEWTABLE  R0, 0, 2    ; R0 = {} (array: 0, hash: 2)
        0x80404009, // PC 1: SETTABLE  R0, K0, K1  ; R0["x"] = 10
        0x8140C009, // PC 2: SETTABLE  R0, K2, K3  ; R0["y"] = 20
        0x00010045, // PC 3: GETGLOBAL R1, K4      ; R1 = print
        0x00400086, // PC 4: GETTABLE  R2, R0, K0  ; R2 = R0["x"]
        0x0100405C, // PC 5: CALL      R1, 2, 1    ; print(R2) -> 10
        0x00010045, // PC 6: GETGLOBAL R1, K4      ; R1 = print
        0x00408086, // PC 7: GETTABLE  R2, R0, K2  ; R2 = R0["y"]
        0x0100405C, // PC 8: CALL      R1, 2, 1    ; print(R2) -> 20
        0x0000401E, // PC 9: RETURN    R0, 1
    ];

    let mut vm = LuaVM::new();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    let main_constants = vec![
        LuaValue::String(String::from("x")),     // K0
        LuaValue::Number(10.0),                  // K1
        LuaValue::String(String::from("y")),     // K2
        LuaValue::Number(20.0),                  // K3
        LuaValue::String(String::from("print")), // K4
    ];
    let proto = Proto::new(main_bytecode, main_constants, vec![]);
    vm.load_proto(proto);
    while vm.step().is_some() {}
}

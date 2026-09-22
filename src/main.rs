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
    // a = 10
    // b = 4
    // print(a * b)
    let main_bytecode: Vec<u32> = vec![
        0x00000001, // PC 0: LOADK     R0, K0      ; R0 = 10
        0x00004041, // PC 1: LOADK     R1, K1      ; R1 = 4
        0x0000408E, // PC 2: MUL       R2, R0, R1  ; R2 = R0 * R1 = 10 * 4 = 40
        0x000080C5, // PC 3: GETGLOBAL R3, K2      ; R3 = print
        0x01000100, // PC 4: MOVE      R4, R2      ; R4 = R2
        0x010040DD, // PC 5: TAILCALL  R3, 2, 1    ; print(R4)
        0x0080001E, // PC 6: RETURN    R0, 1
    ];

    let mut vm = LuaVM::new();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    let main_constants = vec![
        LuaValue::Number(10.0),                  // K0
        LuaValue::Number(4.0),                   // K1
        LuaValue::String(String::from("print")), // K2
    ];
    let proto = Proto::new(main_bytecode, main_constants, vec![]);
    vm.load_proto(proto);
    while vm.step().is_some() {}
}

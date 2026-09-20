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
    // tbl = { 10, 20, 30 }
    // print(tbl[2])
    let main_bytecode: Vec<u32> = vec![
        0x0180000A, // PC 0:  NEWTABLE  R0, 3, 0    ; R0 = {} (array size hint 3)
        0x00000041, // PC 1:  LOADK     R1, K0      ; R1 = 10
        0x00004081, // PC 2:  LOADK     R2, K1      ; R2 = 20
        0x000080C1, // PC 3:  LOADK     R3, K2      ; R3 = 30
        0x01804022, // PC 4:  SETLIST   R0, 3, 1    ; R0[1..3] = R1..R3
        0x0000C007, // PC 5:  SETGLOBAL R0, K3      ; tbl = R0
        0x00010045, // PC 6:  GETGLOBAL R1, K4      ; R1 = print
        0x0000C085, // PC 7:  GETGLOBAL R2, K3      ; R2 = tbl
        0x000140C1, // PC 8:  LOADK     R3, K5      ; R3 = 2
        0x0100C086, // PC 9:  GETTABLE  R2, R2, R3  ; R2 = tbl[2]
        0x0100405D, // PC 10: TAILCALL  R1, 2, 1    ; print(tbl[2])
        0x0080001E, // PC 11: RETURN    R0, 1
    ];

    let mut vm = LuaVM::new();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    let main_constants = vec![
        LuaValue::Number(10.0),                  // K0
        LuaValue::Number(20.0),                  // K1
        LuaValue::Number(30.0),                  // K2
        LuaValue::String(String::from("tbl")),   // K3
        LuaValue::String(String::from("print")), // K4
        LuaValue::Number(2.0),                   // K5
    ];
    let proto = Proto::new(main_bytecode, main_constants, vec![]);
    vm.load_proto(proto);
    while vm.step().is_some() {}
}

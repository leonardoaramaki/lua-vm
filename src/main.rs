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
    // print(a + b) --> 14
    // print(a - b) --> 6
    // print(a * b) --> 40
    // print(a / b) --> 2.5
    // print(a % b) --> 2
    // print(a ^ b) --> 10000
    // print(-a)    --> -10
    let main_bytecode: Vec<u32> = vec![
        0x00000001, // PC 0:  LOADK     R0, K0      ; R0 = 10
        0x00004041, // PC 1:  LOADK     R1, K1      ; R1 = 4
        0x0000408C, // PC 2:  ADD       R2, R0, R1  ; R2 = R0 + R1 = 10 + 4 = 14
        0x000040CD, // PC 3:  SUB       R3, R0, R1  ; R3 = R0 - R1 = 10 - 4 = 6
        0x0000410E, // PC 4:  MUL       R4, R0, R1  ; R4 = R0 * R1 = 10 * 4 = 40
        0x0000414F, // PC 5:  DIV       R5, R0, R1  ; R5 = R0 / R1 = 10 / 4 = 2.5
        0x00004190, // PC 6:  MOD       R6, R0, R1  ; R6 = R0 % R1 = 10 % 4 = 2
        0x000041D1, // PC 7:  POW       R7, R0, R1  ; R7 = R0 ^ R1 = 10 ^ 4 = 10000
        0x00000212, // PC 8:  UNM       R8, R0      ; R8 = -R0 = -10
        0x00008245, // PC 9:  GETGLOBAL R9, K2      ; R9 = print
        0x01000280, // PC 10: MOVE      R10, R2     ; R10 = R2 (14)
        0x0100425C, // PC 11: CALL      R9, 2, 1    ; print(R10) -> 14
        0x00008245, // PC 12: GETGLOBAL R9, K2      ; R9 = print
        0x01800280, // PC 13: MOVE      R10, R3     ; R10 = R3 (6)
        0x0100425C, // PC 14: CALL      R9, 2, 1    ; print(R10) -> 6
        0x00008245, // PC 15: GETGLOBAL R9, K2      ; R9 = print
        0x02000280, // PC 16: MOVE      R10, R4     ; R10 = R4 (40)
        0x0100425C, // PC 17: CALL      R9, 2, 1    ; print(R10) -> 40
        0x00008245, // PC 18: GETGLOBAL R9, K2      ; R9 = print
        0x02800280, // PC 19: MOVE      R10, R5     ; R10 = R5 (2.5)
        0x0100425C, // PC 20: CALL      R9, 2, 1    ; print(R10) -> 2.5
        0x00008245, // PC 21: GETGLOBAL R9, K2      ; R9 = print
        0x03000280, // PC 22: MOVE      R10, R6     ; R10 = R6 (2)
        0x0100425C, // PC 23: CALL      R9, 2, 1    ; print(R10) -> 2
        0x00008245, // PC 24: GETGLOBAL R9, K2      ; R9 = print
        0x03800280, // PC 25: MOVE      R10, R7     ; R10 = R7 (10000)
        0x0100425C, // PC 26: CALL      R9, 2, 1    ; print(R10) -> 10000
        0x00008245, // PC 27: GETGLOBAL R9, K2      ; R9 = print
        0x04000280, // PC 28: MOVE      R10, R8     ; R10 = R8 (-10)
        0x0100425C, // PC 29: CALL      R9, 2, 1    ; print(R10) -> -10
        0x0000401E, // PC 30: RETURN    R0, 1
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

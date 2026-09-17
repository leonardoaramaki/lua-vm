use crate::closure::LuaClosure;
use crate::value::LuaValue;
use crate::vm::LuaVM;

mod closure;
mod value;
mod vm;

fn print(args: &[LuaValue]) {
    if args.is_empty() {
        return;
    }
    println!("{}", args[0]);
}

fn main() {
    let bytecode: Vec<u32> = vec![
        0x00000001, // PC 0:  LOADK     R0, K0     ; R0 = "green"
        0x00004007, // PC 1:  SETGLOBAL R0, K1     ; TrafficLights = R0
        0x00004045, // PC 2:  GETGLOBAL R1, K1     ; R1 = TrafficLights
        0x00000081, // PC 3:  LOADK     R2, K0     ; R2 = "green"
        0x00808017, // PC 4:  EQ        0, R1, R2  ; if (R1 == R2) ~= 0 then pc++
        0x80008016, // PC 5:  JMP       +3          ; else skip the if-body (-> PC 9)
        0x000080C5, // PC 6:  GETGLOBAL R3, K2     ; R3 = print
        0x0000C101, // PC 7:  LOADK     R4, K3     ; R4 = "atravessar a rua"
        0x010040DC, // PC 8:  CALL      R3, 2, 1   ; print(...) -- not a tail call, code follows
        0x00008145, // PC 9:  GETGLOBAL R5, K2     ; R5 = print
        0x00010181, // PC 10: LOADK     R6, K4     ; R6 = "programa finalizado"
        0x0100415D, // PC 11: TAILCALL  R5, 2, 1   ; print(...) -- last statement
        0x0080001E, // PC 12: RETURN    R0, 1
    ];

    let mut vm = LuaVM::new();
    vm.load_chunk(bytecode);
    // For testing, clear the callstack and create a new LuaClosure
    vm.callstack.clear();
    vm.env
        .borrow_mut()
        .insert_global("print", LuaValue::Function(print));
    let mut closure = LuaClosure::new(0);
    closure.env = vm.env.clone();
    closure.constants.push(LuaValue::String(String::from("green")));
    closure
        .constants
        .push(LuaValue::String(String::from("TrafficLights")));
    closure
        .constants
        .push(LuaValue::String(String::from("print")));
    closure
        .constants
        .push(LuaValue::String(String::from("atravessar a rua")));
    closure
        .constants
        .push(LuaValue::String(String::from("programa finalizado")));
    vm.callstack.push_back(closure);
    while vm.step().is_some() {}
}

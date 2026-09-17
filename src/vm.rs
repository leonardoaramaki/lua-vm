use crate::closure::Env;
use crate::closure::LuaClosure;
use crate::value::LuaValue;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Debug)]
pub struct Proto {
    bytecode: Vec<u32>,
}

pub type A = u8;
pub type B = u32;
pub type C = u32;
pub type Bx = u32;
pub type SBx = i32;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Instruction {
    Add(A, B, C),
    Call(A, B, C),
    Concat(A, B, C),
    Eq(A, B, C),
    GetGlobal(A, Bx),
    Jmp(SBx),
    LoadBool(A, B, C),
    Loadk(A, Bx),
    Move(A, Bx),
    Return(A, Bx),
    SetGlobal(A, Bx),
    TailCall(A, B, C),
}

#[derive(Debug)]
pub struct LuaVM {
    pub callstack: VecDeque<LuaClosure>,
    pub protos: Vec<Proto>,
    pub env: Rc<RefCell<Env>>,
    pub stack: VecDeque<LuaValue>,
}

impl LuaVM {
    pub fn new() -> Self {
        Self {
            callstack: VecDeque::new(),
            protos: vec![],
            env: Rc::new(RefCell::new(Env::default())),
            stack: vec![LuaValue::Nil; 1000].into(),
        }
    }

    pub fn load_chunk(&mut self, bytecode: Vec<u32>) {
        let chunk = Proto { bytecode };
        // Proto is moved here and LuaVM is now its owner
        self.protos.push(chunk);
        self.callstack
            .push_back(LuaClosure::new(self.callstack.len()));
    }

    pub fn fetch(&mut self) -> u32 {
        // Read next 4 bytes from PC and increment PC.
        // PC should be in a closure which is an instance of
        let closure = &mut self.callstack[0];
        let chunk = &self.protos[closure.chunk_index];
        let pc = closure.pc;
        closure.pc += 1;
        chunk.bytecode[pc as usize]
    }

    pub fn decode(&self, instruction: u32) -> Instruction {
        let a = ((instruction >> 6) & 0xFF) as u8;
        let b = (instruction >> 23) & 0x1FF;
        let c = (instruction >> 14) & 0x1FF;
        let bx = instruction >> 14;
        // TODO: put 131071 into a constant MAXARG_SBX
        let sbx: i32 = (bx as i32) - 131071;
        let opcode = instruction & 0x3F;
        match opcode {
            0 => Instruction::Move(a, b),
            1 => Instruction::Loadk(a, bx),
            2 => Instruction::LoadBool(a, b, c),
            5 => Instruction::GetGlobal(a, bx),
            7 => Instruction::SetGlobal(a, bx),
            12 => Instruction::Add(a, b, c),
            21 => Instruction::Concat(a, b, c),
            22 => Instruction::Jmp(sbx),
            23 => Instruction::Eq(a, b, c),
            28 => Instruction::Call(a, b, c),
            29 => Instruction::TailCall(a, b, c),
            30 => Instruction::Return(a, b),
            _ => unimplemented!("{}", opcode),
        }
    }

    pub fn execute(&mut self, instruction: Instruction) {
        let closure = self.callstack.front_mut().unwrap();
        // FIXME: Here base is 0, but we should pick this up from somewhere else...
        let base: usize = 0;
        match instruction {
            Instruction::Move(a, b) => {
                // R(A) := R(B)
                self.stack[base + a as usize] = self.stack[base + b as usize].clone();
            }
            Instruction::Add(a, b, c) => {
                // R(A) := RK(B) + RK(C)
                let b = if b < 256 {
                    self.stack[base + b as usize].clone()
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.stack[base + c as usize].clone()
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.stack[a as usize] = LuaValue::sum(&b, &c);
            }
            Instruction::Call(a, b, _c) => {
                if let LuaValue::Function(function) = self.stack[base + a as usize].clone() {
                    let mut args = vec![];
                    for i in 1..=(b - 1) {
                        args.push(self.stack[base + a as usize + i as usize].clone())
                    }
                    function(&args);
                } else {
                    panic!("TAILCALL: R(A) is not a function");
                }
            }
            Instruction::Concat(a, b, c) => {
                // R(A) := R(B) .... R(C)
                assert!(c >= b);
                let mut result = String::new();
                for i in b..=c {
                    if let LuaValue::String(string) = self.stack[base + i as usize].clone() {
                        result.push_str(&string);
                    }
                }
                self.stack[base + a as usize] = LuaValue::String(result);
            }
            Instruction::Eq(a, b, c) => {
                // if ((RK(B) == RK(C)) ~= A) then PC++
                let b = if b < 256 {
                    self.stack[base + b as usize].clone()
                } else {
                    unimplemented!("FIXME: B is a constant: can't EQ constants")
                };
                let c = if c < 256 {
                    self.stack[base + c as usize].clone()
                } else {
                    unimplemented!("FIXME: C is a constant: can't EQ constants")
                };
                // Determine should skip next instruction
                let skip_next = (b == c) == (a != 1);
                if skip_next {
                    closure.pc += 1;
                }
            }
            Instruction::GetGlobal(a, bx) => {
                // R(A) := Glb(Kst(Bx))
                if let LuaValue::String(key) = closure.constants[bx as usize].clone() {
                    self.stack[base + a as usize] = self
                        .env
                        .borrow_mut()
                        .globals
                        .get(&key)
                        .expect("{key} is not a valid global")
                        .clone();
                }
            }
            Instruction::Jmp(sbx) => {
                closure.pc += sbx;
            }
            Instruction::LoadBool(a, b, c) => {
                // R(A) := (Bool)B; if (C) PC++
                self.stack[base + a as usize] = LuaValue::Boolean(b != 0);
                if c != 0 {
                    closure.pc += 1;
                }
            }
            Instruction::Loadk(a, bx) => {
                // R(A) := Kst(Bx)
                self.stack[base + a as usize] = closure.constants[bx as usize].clone();
            }
            Instruction::TailCall(a, b, _c) => {
                if let LuaValue::Function(function) = self.stack[base + a as usize].clone() {
                    let mut args = vec![];
                    for i in 1..=(b - 1) {
                        args.push(self.stack[base + a as usize + i as usize].clone())
                    }
                    function(&args);
                } else {
                    panic!("TAILCALL: R(A) is not a function");
                }
            }
            Instruction::Return(_a, _b) => {
                self.callstack.pop_front();
            }
            Instruction::SetGlobal(a, bx) => {
                let ra = self.stack[base + a as usize].clone();
                let k = closure.constants[bx as usize].clone();
                self.env.borrow_mut().insert_global(&String::from(k), ra);
            }
        }
    }

    /// Step next cycle of fetch, decode, execute.
    /// Returns the processed closure or None otherwise.
    pub fn step(&mut self) -> Option<&LuaClosure> {
        let encoded_instruction = self.fetch();
        let decoded_instruction = self.decode(encoded_instruction);
        self.execute(decoded_instruction);
        self.callstack.front()
    }
}

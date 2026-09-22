use crate::closure::{Env, LuaClosure};
use crate::proto::*;
use crate::value::LuaValue;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

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
    Closure(A, Bx),
    Div(A, B, C),
    Eq(A, B, C),
    GetGlobal(A, Bx),
    GetTable(A, B, C),
    Jmp(SBx),
    Lt(A, B, C),
    LoadBool(A, B, C),
    Loadk(A, Bx),
    Mod(A, B, C),
    Move(A, Bx),
    Mul(A, B, C),
    NewTable(A, B, C),
    Pow(A, B, C),
    Return(A, Bx),
    SetGlobal(A, Bx),
    SetList(A, B, C),
    Sub(A, B, C),
    TailCall(A, B, C),
    Unm(A, B),
}

#[derive(Debug)]
pub struct LuaVM {
    pub callstack: VecDeque<LuaClosure>,
    pub protos: Vec<Rc<Proto>>,
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

    fn set_reg<T>(&mut self, index: T, value: LuaValue)
    where
        T: Into<usize>,
    {
        let closure = self.callstack.front_mut().unwrap();
        let base: usize = closure.base;
        self.stack[base + index.into()] = value;
    }

    fn get_reg<T>(&mut self, index: T) -> LuaValue
    where
        T: TryInto<usize>,
    {
        let index = index.try_into().ok().unwrap();
        let closure = self.callstack.front().unwrap();
        let base: usize = closure.base;
        self.stack[base + index].clone()
    }

    fn get_closure(&self) -> &LuaClosure {
        self.callstack.front().unwrap()
    }

    fn get_closure_mut(&mut self) -> &mut LuaClosure {
        self.callstack.front_mut().unwrap()
    }

    pub fn load_proto(&mut self, proto: Proto) {
        let proto = Rc::new(proto);
        self.protos.push(proto.clone());
        self.callstack.push_back(LuaClosure::new(proto));
    }

    pub fn fetch(&mut self) -> u32 {
        let closure = self.callstack.front_mut().unwrap();
        let pc = closure.pc;
        closure.pc += 1;
        closure.proto.bytecode()[pc as usize]
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
            6 => Instruction::GetTable(a, b, c),
            7 => Instruction::SetGlobal(a, bx),
            10 => Instruction::NewTable(a, b, c),
            12 => Instruction::Add(a, b, c),
            13 => Instruction::Sub(a, b, c),
            14 => Instruction::Mul(a, b, c),
            15 => Instruction::Div(a, b, c),
            16 => Instruction::Mod(a, b, c),
            17 => Instruction::Pow(a, b, c),
            18 => Instruction::Unm(a, b),
            21 => Instruction::Concat(a, b, c),
            22 => Instruction::Jmp(sbx),
            23 => Instruction::Eq(a, b, c),
            24 => Instruction::Lt(a, b, c),
            28 => Instruction::Call(a, b, c),
            29 => Instruction::TailCall(a, b, c),
            30 => Instruction::Return(a, b),
            34 => Instruction::SetList(a, b, c),
            36 => Instruction::Closure(a, bx),
            _ => unimplemented!("{}", opcode),
        }
    }

    pub fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::Move(a, b) => {
                // R(A) := R(B)
                let reg_b = self.get_reg(b);
                self.set_reg(a, reg_b);
            }
            Instruction::Add(a, b, c) => {
                // R(A) := RK(B) + RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, LuaValue::sum(&b, &c));
            }
            Instruction::Call(a, b, _c) => {
                if let LuaValue::Function(function) = self.get_reg(a) {
                    let mut args = vec![];
                    for i in 1..=(b - 1) {
                        args.push(self.get_reg(a + i as u8));
                    }
                    function(&args);
                } else {
                    panic!("TAILCALL: R(A) is not a function");
                }
            }
            Instruction::Closure(a, bx) => {
                // Bx is the function number of the function to be instantiated in the table of function prototypes
                let proto = self.get_closure().proto.protos().get(bx as usize).unwrap();
                self.set_reg(a, LuaValue::LuaFunction(proto.clone()));
            }
            Instruction::Concat(a, b, c) => {
                // R(A) := R(B) .... R(C)
                assert!(c >= b);
                let mut result = String::new();
                let base = self.get_closure().base;
                for i in b..=c {
                    if let LuaValue::String(string) = self.get_reg(base + i as usize) {
                        result.push_str(&string);
                    }
                }
                self.set_reg(a, LuaValue::String(result));
            }
            Instruction::Div(a, b, c) => {
                // R(A) := RK(B) / RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, b / c);
            }
            Instruction::Eq(a, b, c) => {
                // if ((RK(B) == RK(C)) ~= A) then PC++
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: B is a constant: can't EQ constants")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: C is a constant: can't EQ constants")
                };
                // Determine should skip next instruction
                let skip_next = (b == c) == (a != 1);
                if skip_next {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::GetGlobal(a, bx) => {
                // R(A) := Glb(Kst(Bx))
                let closure = self.get_closure();
                let proto = closure.proto.clone();
                if let LuaValue::String(key) = proto.constants()[bx as usize].clone() {
                    let env = self
                        .env
                        .borrow_mut()
                        .globals
                        .get(&key)
                        .expect("Not a valid global")
                        .clone();
                    self.set_reg(a, env);
                }
            }
            Instruction::GetTable(a, b, c) => {
                if let LuaValue::Table(content) = self.get_reg(b) {
                    let c = if c < 256 {
                        self.get_reg(c)
                    } else {
                        let constants = self.get_closure().proto.constants();
                        constants[c as usize].clone()
                    };
                    if let LuaValue::Number(n) = c {
                        let v = content.borrow();
                        self.set_reg(a, v[(n as usize) - 1].clone());
                    }
                }
            }
            Instruction::Jmp(sbx) => {
                self.get_closure_mut().pc += sbx;
            }
            Instruction::Lt(a, b, c) => {
                // if ((RK(B) == RK(C)) ~= A) then PC++
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: B is a constant: can't EQ constants")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: C is a constant: can't EQ constants")
                };
                // Determine should skip next instruction
                let skip_next = (b < c) == (a != 1);
                if skip_next {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::LoadBool(a, b, c) => {
                // R(A) := (Bool)B; if (C) PC++
                self.set_reg(a, LuaValue::Boolean(b != 0));
                if c != 0 {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::Loadk(a, bx) => {
                // R(A) := Kst(Bx)
                self.set_reg(a, self.get_closure().proto.constants()[bx as usize].clone());
            }
            Instruction::Mul(a, b, c) => {
                // R(A) := RK(B) * RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, b * c);
            }
            Instruction::Mod(a, b, c) => {
                // R(A) := RK(B) + RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, b % c);
            }
            Instruction::NewTable(a, b, c) => {
                self.set_reg(a, LuaValue::Table(Rc::new(RefCell::from(vec![]))));
            }
            Instruction::Pow(a, b, c) => {
                // R(A) := RK(B) + RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, b.pow(c));
            }
            Instruction::TailCall(a, b, _c) => {
                let base = self.get_closure().base;
                match self.stack[base + a as usize].clone() {
                    LuaValue::Function(function) => {
                        let mut args = vec![];
                        for i in 1..=(b - 1) {
                            args.push(self.get_reg(a + i as u8));
                        }
                        function(&args);
                    }
                    LuaValue::LuaFunction(function) => {
                        let base = self.get_closure().base;
                        let mut closure = LuaClosure::new(function.clone());
                        closure.env = self.env.clone();
                        closure.base = base + a as usize + 1;
                        self.callstack.push_front(closure);
                    }
                    _ => panic!("TAILCALL: R(A) is not a valid function"),
                }
            }
            Instruction::Return(_a, _b) => {
                self.callstack.pop_front();
            }
            Instruction::SetGlobal(a, bx) => {
                let ra = self.get_reg(a);
                let k = self.get_closure().proto.constants()[bx as usize].clone();
                self.env.borrow_mut().insert_global(&String::from(k), ra);
            }
            Instruction::SetList(a, b, _c) => {
                if b > 0 {
                    for i in a + 1..=(a + b as u8) {
                        if let LuaValue::Table(content) = self.get_reg(a) {
                            let v = self.get_reg(i);
                            content.borrow_mut().push(v);
                        }
                    }
                } else {
                    unimplemented!("SETLIST: Variable number of arguments");
                }
            }
            Instruction::Sub(a, b, c) => {
                // R(A) := RK(B) - RK(C)
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    unimplemented!("FIXME: adding constants not implemented")
                };
                self.set_reg(a, b - c);
            }
            Instruction::Unm(a, b) => {
                let b = self.get_reg(b);
                self.set_reg(a, b * LuaValue::Number(-1.0));
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

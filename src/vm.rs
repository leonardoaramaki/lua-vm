use crate::closure::{Env, LuaClosure};
use crate::proto::*;
use crate::value::LuaValue;
use std::collections::HashMap;
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
    ForLoop(A, SBx),
    ForPrep(A, SBx),
    GetTable(A, B, C),
    Jmp(SBx),
    Len(A, B),
    Lt(A, B, C),
    LoadBool(A, B, C),
    Loadk(A, Bx),
    Mod(A, B, C),
    Move(A, Bx),
    Mul(A, B, C),
    NewTable(A, B, C),
    Not(A, B),
    Pow(A, B, C),
    Return(A, Bx),
    SelfOp(A, B, C),
    SetGlobal(A, Bx),
    SetList(A, B, C),
    SetTable(A, B, C),
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
    pub top: usize,
}

impl LuaVM {
    pub fn new() -> Self {
        Self {
            callstack: VecDeque::new(),
            protos: vec![],
            env: Rc::new(RefCell::new(Env::default())),
            stack: vec![LuaValue::Nil; 1000].into(),
            top: 0,
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
        let mut main = LuaClosure::new(proto.clone());
        main.base = 1;
        main.env = self.env.clone();
        self.stack[0] = LuaValue::LuaFunction(proto);
        self.callstack.push_back(main);
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
            9 => Instruction::SetTable(a, b, c),
            10 => Instruction::NewTable(a, b, c),
            11 => Instruction::SelfOp(a, b, c),
            12 => Instruction::Add(a, b, c),
            13 => Instruction::Sub(a, b, c),
            14 => Instruction::Mul(a, b, c),
            15 => Instruction::Div(a, b, c),
            16 => Instruction::Mod(a, b, c),
            17 => Instruction::Pow(a, b, c),
            18 => Instruction::Unm(a, b),
            19 => Instruction::Not(a, b),
            20 => Instruction::Len(a, b),
            21 => Instruction::Concat(a, b, c),
            22 => Instruction::Jmp(sbx),
            23 => Instruction::Eq(a, b, c),
            24 => Instruction::Lt(a, b, c),
            28 => Instruction::Call(a, b, c),
            29 => Instruction::TailCall(a, b, c),
            30 => Instruction::Return(a, b),
            31 => Instruction::ForLoop(a, sbx),
            32 => Instruction::ForPrep(a, sbx),
            34 => Instruction::SetList(a, b, c),
            36 => Instruction::Closure(a, bx),
            _ => unimplemented!("{}", opcode),
        }
    }

    fn decode_fpb(fpb: u32) -> (u32, u32) {
        ((fpb >> 3) & 0x1F, (fpb & 0b00000111))
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
            Instruction::Call(a, b, c) => {
                let func = self.get_closure().base + a as usize;
                let nargs = if b == 0 {
                    self.top - (func + 1)
                } else {
                    b as usize - 1
                };
                let nresults = c as i32 - 1;
                match self.get_reg(a) {
                    LuaValue::Function(function) => {
                        let args: Vec<LuaValue> = self
                            .stack
                            .range(func + 1..func + 1 + nargs)
                            .cloned()
                            .collect();
                        let result = function(&args).unwrap();
                        if c == 0 {
                            // Caller wants "all results".
                            result
                                .iter()
                                .enumerate()
                                .for_each(|x| self.stack[func + x.0] = x.1.clone());

                            self.top = func + result.len();
                        } else {
                            // Caller wants exactly c - 1 results in R(A), R(A + 1), ...
                            for i in 0..nresults as usize {
                                self.stack[func + i] =
                                    result.get(i).cloned().unwrap_or(LuaValue::Nil);
                            }
                        }
                    }
                    LuaValue::LuaFunction(lua_function) => {
                        let mut closure = LuaClosure::new(lua_function);
                        closure.env = self.env.clone();
                        closure.base = func + 1;
                        closure.nresults = nresults;
                        self.callstack.push_front(closure);
                    }
                    other => panic!("CALL: attempt to call a {} value", other),
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
                for i in b..=c {
                    if let LuaValue::String(string) = self.get_reg(i as usize) {
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
            Instruction::ForPrep(a, sbx) => {
                // Initial value
                let initial = self.get_reg(a);
                // Limit
                let _limit = self.get_reg(a + 1);
                // Step
                let step = self.get_reg(a + 2);
                // Loop Variable
                let _loop_var = self.get_reg(a + 3);
                self.set_reg(a, initial - step);
                self.get_closure_mut().pc += sbx;
            }
            Instruction::ForLoop(a, sbx) => {
                // Limit
                let limit = self.get_reg(a + 1);
                // Step
                let step = self.get_reg(a + 2);
                let next = self.get_reg(a) + step.clone();
                self.set_reg(a, next.clone());
                let continue_loop = match step {
                    LuaValue::Number(n) => {
                        if n > 0.0 {
                            next <= limit
                        } else {
                            next >= limit
                        }
                    }
                    other => panic!("'for' step must be a number, got, {}", other),
                };
                if continue_loop {
                    self.get_closure_mut().pc += sbx;
                    self.set_reg(a + 3, next);
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
                if let LuaValue::Table(_, h) = self.get_reg(b) {
                    let c = if c < 256 {
                        self.get_reg(c)
                    } else {
                        let constants = self.get_closure().proto.constants();
                        constants[c as usize % 256].clone()
                    };

                    // C is the key, so check which type is it
                    match c {
                        LuaValue::Number(n) => {
                            // Assert that B is a table and get that
                            if let LuaValue::Table(v, h) = self.get_reg(b) {
                                // Return from the array if within bounds otherwise return from the
                                // map
                                if n.fract() == 0.0 && n >= 1.0 && n <= v.borrow().len() as f64 {
                                    self.set_reg(a, v.borrow()[n as usize - 1].clone());
                                } else {
                                    self.set_reg(
                                        a,
                                        h.borrow().get(&c).unwrap_or(&LuaValue::Nil).clone(),
                                    );
                                }
                            }
                        }
                        _ => {
                            self.set_reg(a, h.borrow().get(&c).unwrap_or(&LuaValue::Nil).clone());
                        }
                    }
                }
            }
            Instruction::Jmp(sbx) => {
                self.get_closure_mut().pc += sbx;
            }
            Instruction::Len(a, b) => {
                let b = self.get_reg(b);
                let l = match b {
                    LuaValue::String(s) => s.len(),
                    LuaValue::Table(v, _h) => {
                        v.borrow().iter().filter(|x| **x != LuaValue::Nil).count()
                    }
                    _ => unimplemented!(),
                };
                self.set_reg(a, LuaValue::Number(l as f64));
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
                let (be, bm) = Self::decode_fpb(b);
                let (ce, cm) = Self::decode_fpb(c);
                let b = if be > 0 {
                    // 1xxx*2^(eeeee-1)
                    (0b1000 | bm) * 2u32.pow(be - 1)
                } else {
                    bm
                };
                let c = if ce > 0 {
                    // 1xxx*2^(eeeee-1)
                    (0b1000 | cm) * 2u32.pow(ce - 1)
                } else {
                    cm
                };
                self.set_reg(
                    a,
                    LuaValue::Table(
                        Rc::new(RefCell::from(Vec::with_capacity(b as usize))),
                        Rc::new(RefCell::from(HashMap::with_capacity(c as usize))),
                    ),
                );
            }
            Instruction::Not(a, b) => {
                if let LuaValue::Boolean(b) = self.get_reg(b) {
                    self.set_reg(a, !LuaValue::Boolean(b));
                }
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
            Instruction::TailCall(a, b, c) => {
                let base = self.get_closure().base;
                let func = self.get_closure().base + a as usize;
                let nargs = if b == 0 {
                    self.top - (func + 1)
                } else {
                    b as usize - 1
                };
                let nresults = c as i32 - 1;
                match self.stack[base + a as usize].clone() {
                    LuaValue::Function(function) => {
                        let args: Vec<LuaValue> = self
                            .stack
                            .range(func + 1..func + 1 + nargs)
                            .cloned()
                            .collect();
                        let result = function(&args).unwrap();
                        if c == 0 {
                            // Caller wants "all results".
                            result
                                .iter()
                                .enumerate()
                                .for_each(|x| self.stack[func + x.0] = x.1.clone());

                            self.top = func + result.len();
                        } else {
                            // Caller wants exactly c - 1 results in R(A), R(A + 1), ...
                            for i in 0..nresults as usize {
                                self.stack[func + i] =
                                    result.get(i).cloned().unwrap_or(LuaValue::Nil);
                            }
                        }
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
            Instruction::Return(a, b) => {
                let frame = self.callstack.pop_front().unwrap();
                let first = frame.base + a as usize;
                let n = if b == 0 {
                    self.top - first
                } else {
                    b as usize - 1
                };
                if self.callstack.is_empty() {
                    return;
                }
                let dst = frame.base - 1;
                for i in 0..n {
                    self.stack[dst + i] = self.stack[first + i].clone();
                }
                if frame.nresults < 0 {
                    self.top = dst + n;
                } else {
                    // nresults is how many values the caller wants.
                    // n is how many values are actually returned.
                    for i in n..frame.nresults as usize {
                        self.stack[dst + i] = LuaValue::Nil;
                    }
                }
            }
            Instruction::SelfOp(a, b, c) => {
                let b_value = self.get_reg(b);
                self.set_reg(a + 1, b_value.clone());
                let c_value = if c < 256 {
                    self.get_reg(c)
                } else {
                    self.get_closure().proto.constants()[c as usize - 256].clone()
                };
                if let LuaValue::Table(_, h) = b_value {
                    let value = h.borrow().get(&c_value).cloned().unwrap_or(LuaValue::Nil);
                    self.set_reg(a, value);
                } else {
                    panic!("SELF: R(B) is not a table");
                }
            }
            Instruction::SetGlobal(a, bx) => {
                let ra = self.get_reg(a);
                let k = self.get_closure().proto.constants()[bx as usize].clone();
                self.env.borrow_mut().insert_global(&String::from(k), ra);
            }
            Instruction::SetList(a, b, _c) => {
                if b > 0 {
                    for i in a + 1..=(a + b as u8) {
                        if let LuaValue::Table(vec, _) = self.get_reg(a) {
                            let v = self.get_reg(i);
                            vec.borrow_mut().push(v);
                        }
                    }
                } else {
                    unimplemented!("SETLIST: Variable number of arguments");
                }
            }
            Instruction::SetTable(a, b, c) => {
                let tbl = self.get_reg(a);
                let b = if b < 256 {
                    self.get_reg(b)
                } else {
                    let proto = self.get_closure().proto.clone();
                    let constants = proto.constants();
                    constants[b as usize % 256].clone()
                };
                let c = if c < 256 {
                    self.get_reg(c)
                } else {
                    let proto = self.get_closure().proto.clone();
                    let constants = proto.constants();
                    constants[c as usize % 256].clone()
                };
                if let LuaValue::Table(v, h) = tbl {
                    if let LuaValue::Number(n) = b
                        && n.fract() == 0.0
                        && n >= 1.0
                        && n <= v.borrow().len() as f64
                    {
                        v.borrow_mut()[n as usize - 1] = c;
                        return;
                    }
                    h.borrow_mut().insert(b, c);
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

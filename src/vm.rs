use crate::closure::{Env, LuaClosure};
use crate::value::{LuaFunc, LuaValue, Upval, format_number};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

pub type A = u8;
pub type B = u32;
pub type C = u32;
pub type Bx = u32;
pub type SBx = i32;

/// Lua 5.1 flushes table constructors to SETLIST in batches of this size.
const FIELDS_PER_FLUSH: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Instruction {
    Add(A, B, C),
    Call(A, B, C),
    Close(A),
    Concat(A, B, C),
    Closure(A, Bx),
    Div(A, B, C),
    Eq(A, B, C),
    GetGlobal(A, Bx),
    GetUpval(A, B),
    ForLoop(A, SBx),
    ForPrep(A, SBx),
    GetTable(A, B, C),
    Jmp(SBx),
    Len(A, B),
    Le(A, B, C),
    Lt(A, B, C),
    LoadBool(A, B, C),
    Loadk(A, Bx),
    LoadNil(A, B),
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
    SetUpval(A, B),
    Sub(A, B, C),
    TailCall(A, B, C),
    Test(A, C),
    TestSet(A, B, C),
    TForLoop(A, C),
    Unm(A, B),
}

pub struct LuaVM {
    pub callstack: VecDeque<LuaClosure>,
    pub env: Rc<RefCell<Env>>,
    pub stack: VecDeque<LuaValue>,
    pub top: usize,
    /// Upvalues that still point into the stack, shared by every closure that
    /// captured the same local.
    open_upvals: Vec<Rc<RefCell<Upval>>>,
}

impl LuaVM {
    pub fn new() -> Self {
        Self {
            callstack: VecDeque::new(),
            env: Rc::new(RefCell::new(Env::default())),
            stack: vec![LuaValue::Nil; 1000].into(),
            top: 0,
            open_upvals: vec![],
        }
    }

    fn ensure_stack(&mut self, size: usize) {
        if self.stack.len() < size {
            self.stack.resize(size, LuaValue::Nil);
        }
    }

    fn base(&self) -> usize {
        self.get_closure().base
    }

    fn set_reg<T>(&mut self, index: T, value: LuaValue)
    where
        T: Into<usize>,
    {
        let base = self.base();
        self.stack[base + index.into()] = value;
    }

    fn get_reg<T>(&mut self, index: T) -> LuaValue
    where
        T: TryInto<usize>,
    {
        let index = index.try_into().ok().unwrap();
        let base = self.base();
        self.stack[base + index].clone()
    }

    fn get_closure(&self) -> &LuaClosure {
        self.callstack.front().unwrap()
    }

    fn get_closure_mut(&mut self) -> &mut LuaClosure {
        self.callstack.front_mut().unwrap()
    }

    /// Calls `func` with `args` and returns all of its results. Works both from
    /// the host (empty call stack) and from inside a running instruction.
    pub fn call(&mut self, func: LuaValue, args: &[LuaValue]) -> anyhow::Result<Vec<LuaValue>> {
        // Place the call above everything the current frame may be using.
        let at = match self.callstack.front() {
            Some(frame) => frame.base + frame.proto.max_stack_size as usize,
            None => 0,
        };
        self.call_at(at, func, args)
    }

    fn call_at(
        &mut self,
        at: usize,
        func: LuaValue,
        args: &[LuaValue],
    ) -> anyhow::Result<Vec<LuaValue>> {
        self.ensure_stack(at + args.len() + 1);
        self.stack[at] = func;
        for (i, arg) in args.iter().enumerate() {
            self.stack[at + 1 + i] = arg.clone();
        }
        let depth = self.callstack.len();
        let result = self.precall(at, args.len(), -1).and_then(|()| {
            while self.callstack.len() > depth {
                self.step()?;
            }
            Ok(())
        });
        if let Err(e) = result {
            // Drop the frames of the failed call so the VM can be used again.
            while self.callstack.len() > depth {
                let frame = self.callstack.pop_front().unwrap();
                self.close_upvals(frame.base);
            }
            return Err(e);
        }
        Ok(self.stack.range(at..self.top).cloned().collect())
    }

    /// Starts a call to the function in `stack[func]` whose `nargs` arguments
    /// follow it. Native functions run to completion here and leave their results
    /// starting at `func`; Lua functions get a new frame that `step` then runs.
    fn precall(&mut self, func: usize, nargs: usize, nresults: i32) -> anyhow::Result<()> {
        match self.stack[func].clone() {
            LuaValue::Function(function) => {
                let args: Vec<LuaValue> =
                    self.stack.range(func + 1..func + 1 + nargs).cloned().collect();
                let result = function(&args)?;
                if nresults < 0 {
                    // Caller wants "all results".
                    self.ensure_stack(func + result.len() + 1);
                    for (i, value) in result.iter().enumerate() {
                        self.stack[func + i] = value.clone();
                    }
                    self.top = func + result.len();
                } else {
                    // Caller wants exactly nresults results in R(A), R(A + 1), ...
                    for i in 0..nresults as usize {
                        self.stack[func + i] = result.get(i).cloned().unwrap_or(LuaValue::Nil);
                    }
                }
            }
            LuaValue::LuaFunction(lua_function) => {
                let base = func + 1;
                let num_params = lua_function.proto.num_params as usize;
                let max_stack = lua_function.proto.max_stack_size as usize;
                self.ensure_stack(base + max_stack.max(nargs) + 1);
                // Missing parameters, extra arguments and the remaining registers
                // all start out as nil, like luaD_precall does.
                for i in nargs.min(num_params)..max_stack {
                    self.stack[base + i] = LuaValue::Nil;
                }
                let mut closure = LuaClosure::new(lua_function);
                closure.env = self.env.clone();
                closure.base = base;
                closure.nresults = nresults;
                self.callstack.push_front(closure);
            }
            other => anyhow::bail!("attempt to call a {} value", other.type_name()),
        }
        Ok(())
    }

    /// Returns the open upvalue for stack slot `slot`, creating it if needed.
    fn find_upval(&mut self, slot: usize) -> Rc<RefCell<Upval>> {
        for upval in &self.open_upvals {
            if matches!(*upval.borrow(), Upval::Open(i) if i == slot) {
                return upval.clone();
            }
        }
        let upval = Rc::new(RefCell::new(Upval::Open(slot)));
        self.open_upvals.push(upval.clone());
        upval
    }

    /// Moves every open upvalue at or above stack slot `level` off the stack.
    fn close_upvals(&mut self, level: usize) {
        let stack = &self.stack;
        self.open_upvals.retain(|upval| {
            let slot = match *upval.borrow() {
                Upval::Open(i) => i,
                Upval::Closed(_) => return false,
            };
            if slot < level {
                return true;
            }
            *upval.borrow_mut() = Upval::Closed(stack[slot].clone());
            false
        });
    }

    pub fn fetch(&mut self) -> u32 {
        let closure = self.callstack.front_mut().unwrap();
        let pc = closure.pc;
        closure.pc += 1;
        closure.proto.bytecode()[pc as usize]
    }

    pub fn decode(&self, instruction: u32) -> anyhow::Result<Instruction> {
        let a = ((instruction >> 6) & 0xFF) as u8;
        let b = (instruction >> 23) & 0x1FF;
        let c = (instruction >> 14) & 0x1FF;
        let bx = instruction >> 14;
        // TODO: put 131071 into a constant MAXARG_SBX
        let sbx: i32 = (bx as i32) - 131071;
        let opcode = instruction & 0x3F;
        Ok(match opcode {
            0 => Instruction::Move(a, b),
            1 => Instruction::Loadk(a, bx),
            2 => Instruction::LoadBool(a, b, c),
            3 => Instruction::LoadNil(a, b),
            4 => Instruction::GetUpval(a, b),
            5 => Instruction::GetGlobal(a, bx),
            6 => Instruction::GetTable(a, b, c),
            7 => Instruction::SetGlobal(a, bx),
            8 => Instruction::SetUpval(a, b),
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
            25 => Instruction::Le(a, b, c),
            26 => Instruction::Test(a, c),
            27 => Instruction::TestSet(a, b, c),
            28 => Instruction::Call(a, b, c),
            29 => Instruction::TailCall(a, b, c),
            30 => Instruction::Return(a, b),
            31 => Instruction::ForLoop(a, sbx),
            32 => Instruction::ForPrep(a, sbx),
            33 => Instruction::TForLoop(a, c),
            34 => Instruction::SetList(a, b, c),
            35 => Instruction::Close(a),
            36 => Instruction::Closure(a, bx),
            37 => anyhow::bail!("VARARG (...) is not supported"),
            _ => anyhow::bail!("unknown opcode {}", opcode),
        })
    }

    fn decode_fpb(fpb: u32) -> (u32, u32) {
        ((fpb >> 3) & 0x1F, (fpb & 0b00000111))
    }

    fn get_rk(&mut self, rk: u32) -> LuaValue {
        // RK(x): registrador se x < 256, senão Kst(x - 256)
        if rk < 256 {
            self.get_reg(rk)
        } else {
            self.get_closure().proto.constants()[(rk - 256) as usize].clone()
        }
    }

    /// Applies an arithmetic operator, coercing numeric strings like Lua does.
    fn arith(
        &mut self,
        b: LuaValue,
        c: LuaValue,
        op: impl Fn(f64, f64) -> f64,
    ) -> anyhow::Result<LuaValue> {
        let to_number = |v: &LuaValue| match v {
            LuaValue::Number(n) => Some(*n),
            LuaValue::String(s) => s.trim().parse().ok(),
            _ => None,
        };
        match (to_number(&b), to_number(&c)) {
            (Some(x), Some(y)) => Ok(LuaValue::Number(op(x, y))),
            (None, _) => anyhow::bail!("attempt to perform arithmetic on a {} value", b.type_name()),
            _ => anyhow::bail!("attempt to perform arithmetic on a {} value", c.type_name()),
        }
    }

    pub fn execute(&mut self, instruction: Instruction) -> anyhow::Result<()> {
        match instruction {
            Instruction::Move(a, b) => {
                // R(A) := R(B)
                let reg_b = self.get_reg(b);
                self.set_reg(a, reg_b);
            }
            Instruction::Add(a, b, c) => {
                // R(A) := RK(B) + RK(C)
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, |x, y| x + y)?;
                self.set_reg(a, v);
            }
            Instruction::Call(a, b, c) => {
                let func = self.base() + a as usize;
                let nargs = if b == 0 {
                    self.top - (func + 1)
                } else {
                    b as usize - 1
                };
                self.precall(func, nargs, c as i32 - 1)?;
            }
            Instruction::Close(a) => {
                // close all upvalues >= R(A)
                let level = self.base() + a as usize;
                self.close_upvals(level);
            }
            Instruction::Closure(a, bx) => {
                // Bx is the function number of the function to be instantiated in the table of function prototypes
                let proto = self.get_closure().proto.protos()[bx as usize].clone();
                // Each upvalue is described by a pseudo-instruction after CLOSURE:
                // MOVE 0 B captures local R(B), GETUPVAL 0 B shares our upvalue B.
                let mut upvals = Vec::with_capacity(proto.num_upvalues as usize);
                for _ in 0..proto.num_upvalues {
                    let pseudo = self.fetch();
                    let b = ((pseudo >> 23) & 0x1FF) as usize;
                    match pseudo & 0x3F {
                        0 => {
                            let slot = self.base() + b;
                            upvals.push(self.find_upval(slot));
                        }
                        4 => upvals.push(self.get_closure().func.upvals[b].clone()),
                        op => anyhow::bail!("CLOSURE: unexpected upvalue opcode {}", op),
                    }
                }
                self.set_reg(a, LuaValue::LuaFunction(Rc::new(LuaFunc { proto, upvals })));
            }
            Instruction::Concat(a, b, c) => {
                // R(A) := R(B) .... R(C)
                assert!(c >= b);
                let mut result = String::new();
                for i in b..=c {
                    match self.get_reg(i as usize) {
                        LuaValue::String(string) => result.push_str(&string),
                        LuaValue::Number(n) => result.push_str(&format_number(n)),
                        other => {
                            anyhow::bail!("attempt to concatenate a {} value", other.type_name())
                        }
                    }
                }
                self.set_reg(a, LuaValue::String(result));
            }
            Instruction::Div(a, b, c) => {
                // R(A) := RK(B) / RK(C)
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, |x, y| x / y)?;
                self.set_reg(a, v);
            }
            Instruction::Eq(a, b, c) => {
                // if ((RK(B) == RK(C)) ~= A) then PC++
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                // Determine should skip next instruction
                let skip_next = (b == c) == (a != 1);
                if skip_next {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::ForPrep(a, sbx) => {
                // Initial value
                let initial = self.get_reg(a);
                // Step
                let step = self.get_reg(a + 2);
                let v = self.arith(initial, step, |x, y| x - y)?;
                self.set_reg(a, v);
                self.get_closure_mut().pc += sbx;
            }
            Instruction::ForLoop(a, sbx) => {
                // Limit
                let limit = self.get_reg(a + 1);
                // Step
                let step = self.get_reg(a + 2);
                let index = self.get_reg(a);
                let next = self.arith(index, step.clone(), |x, y| x + y)?;
                self.set_reg(a, next.clone());
                let continue_loop = match step {
                    LuaValue::Number(n) => {
                        if n > 0.0 {
                            next <= limit
                        } else {
                            next >= limit
                        }
                    }
                    other => anyhow::bail!("'for' step must be a number, got {}", other),
                };
                if continue_loop {
                    self.get_closure_mut().pc += sbx;
                    self.set_reg(a + 3, next);
                }
            }
            Instruction::GetGlobal(a, bx) => {
                // R(A) := Glb(Kst(Bx))
                let key = self.get_closure().proto.constants()[bx as usize].clone();
                if let LuaValue::String(key) = key {
                    let value = self.env.borrow().globals.get(&key).cloned();
                    self.set_reg(a, value.unwrap_or(LuaValue::Nil));
                }
            }
            Instruction::GetTable(a, b, c) => {
                // R(A) := R(B)[RK(C)]
                let table = self.get_reg(b);
                let key = self.get_rk(c);
                let value = table.index(&key)?;
                self.set_reg(a, value);
            }
            Instruction::GetUpval(a, b) => {
                // R(A) := UpValue[B]
                let upval = self.get_closure().func.upvals[b as usize].clone();
                let value = match &*upval.borrow() {
                    Upval::Open(slot) => self.stack[*slot].clone(),
                    Upval::Closed(value) => value.clone(),
                };
                self.set_reg(a, value);
            }
            Instruction::Jmp(sbx) => {
                self.get_closure_mut().pc += sbx;
            }
            Instruction::Len(a, b) => {
                let l = self.get_reg(b).len()?;
                self.set_reg(a, LuaValue::Number(l as f64));
            }
            Instruction::Le(a, b, c) => {
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let a = a == 1;
                if (b <= c) != a {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::Lt(a, b, c) => {
                // if ((RK(B) == RK(C)) ~= A) then PC++
                let (b, c) = (self.get_rk(b), self.get_rk(c));
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
            Instruction::LoadNil(a, b) => {
                for i in a..=b as u8 {
                    self.set_reg(i, LuaValue::Nil);
                }
            }
            Instruction::Mul(a, b, c) => {
                // R(A) := RK(B) * RK(C)
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, |x, y| x * y)?;
                self.set_reg(a, v);
            }
            Instruction::Mod(a, b, c) => {
                // R(A) := RK(B) % RK(C), with the sign of the divisor
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, |x, y| x - (x / y).floor() * y)?;
                self.set_reg(a, v);
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
                let b = self.get_reg(b);
                self.set_reg(a, !b);
            }
            Instruction::Pow(a, b, c) => {
                // R(A) := RK(B) ^ RK(C)
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, f64::powf)?;
                self.set_reg(a, v);
            }
            Instruction::TailCall(a, b, _c) => {
                // Runs as a regular call; the RETURN that follows it hands the
                // results back to our caller.
                let func = self.base() + a as usize;
                let nargs = if b == 0 {
                    self.top - (func + 1)
                } else {
                    b as usize - 1
                };
                self.precall(func, nargs, -1)?;
            }
            Instruction::Test(a, c) => {
                // if not (R(A) <=> C) then pc++
                if self.get_reg(a).truthy() != (c != 0) {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::TestSet(a, b, c) => {
                // if (R(B) <=> C) then R(A) := R(B) else pc++
                let value = self.get_reg(b);
                if value.truthy() == (c != 0) {
                    self.set_reg(a, value);
                } else {
                    self.get_closure_mut().pc += 1;
                }
            }
            Instruction::TForLoop(a, c) => {
                // R(A+3), ..., R(A+2+C) := R(A)(R(A+1), R(A+2));
                // if R(A+3) ~= nil then R(A+2) = R(A+3) else pc++
                let ra = self.base() + a as usize;
                let iterator = self.stack[ra].clone();
                let args = [self.stack[ra + 1].clone(), self.stack[ra + 2].clone()];
                let results = self.call_at(ra + 3, iterator, &args)?;
                for i in 0..c as usize {
                    self.stack[ra + 3 + i] = results.get(i).cloned().unwrap_or(LuaValue::Nil);
                }
                let first = self.stack[ra + 3].clone();
                if first != LuaValue::Nil {
                    self.stack[ra + 2] = first;
                } else {
                    // Skip the JMP that goes back to the loop body.
                    self.get_closure_mut().pc += 1;
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
                // Locals captured by closures must outlive this frame.
                self.close_upvals(frame.base);
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
                // R(A+1) := R(B); R(A) := R(B)[RK(C)]
                let object = self.get_reg(b);
                self.set_reg(a + 1, object.clone());
                let key = self.get_rk(c);
                let method = object.index(&key)?;
                self.set_reg(a, method);
            }
            Instruction::SetGlobal(a, bx) => {
                let ra = self.get_reg(a);
                let k = self.get_closure().proto.constants()[bx as usize].clone();
                self.env.borrow_mut().insert_global(&String::from(k), ra);
            }
            Instruction::SetList(a, b, c) => {
                // R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B
                let table = self.get_reg(a);
                let n = if b == 0 {
                    // Values go up to the top left by a multi-result call.
                    self.top - (self.base() + a as usize) - 1
                } else {
                    b as usize
                };
                // A batch number too big for C is stored in the next instruction.
                let c = if c == 0 { self.fetch() } else { c } as usize;
                let offset = (c - 1) * FIELDS_PER_FLUSH;
                for i in 1..=n {
                    let value = self.get_reg(a as usize + i);
                    table.set_index(LuaValue::Number((offset + i) as f64), value)?;
                }
            }
            Instruction::SetTable(a, b, c) => {
                // R(A)[RK(B)] := RK(C)
                let table = self.get_reg(a);
                let (key, value) = (self.get_rk(b), self.get_rk(c));
                table.set_index(key, value)?;
            }
            Instruction::SetUpval(a, b) => {
                // UpValue[B] := R(A)
                let value = self.get_reg(a);
                let upval = self.get_closure().func.upvals[b as usize].clone();
                let mut upval = upval.borrow_mut();
                match &mut *upval {
                    Upval::Open(slot) => self.stack[*slot] = value,
                    Upval::Closed(v) => *v = value,
                }
            }
            Instruction::Sub(a, b, c) => {
                // R(A) := RK(B) - RK(C)
                let (b, c) = (self.get_rk(b), self.get_rk(c));
                let v = self.arith(b, c, |x, y| x - y)?;
                self.set_reg(a, v);
            }
            Instruction::Unm(a, b) => {
                let b = self.get_reg(b);
                let v = self.arith(b, LuaValue::Number(0.0), |x, _| -x)?;
                self.set_reg(a, v);
            }
        }
        Ok(())
    }

    /// Runs the next instruction of the current frame. Errors are tagged with
    /// the source position, so nested calls read like a traceback.
    pub fn step(&mut self) -> anyhow::Result<()> {
        let frame = self.get_closure();
        let (proto, pc) = (frame.proto.clone(), frame.pc as usize);
        let encoded_instruction = self.fetch();
        self.decode(encoded_instruction)
            .and_then(|instruction| self.execute(instruction))
            .map_err(|e| {
                let line = proto.line_info.get(pc).copied().unwrap_or(0);
                e.context(format!("{}:{}", proto.source, line))
            })
    }
}

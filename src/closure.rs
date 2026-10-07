use crate::{proto::Proto, value::LuaValue};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// env is where the globals table lives (G_)
#[derive(Default)]
pub struct Env {
    pub globals: HashMap<String, LuaValue>,
}

impl Env {
    pub fn insert_global(&mut self, k: &str, v: LuaValue) {
        self.globals.insert(String::from(k), v);
    }
}

pub struct LuaClosure {
    pub pc: i32,
    pub proto: Rc<Proto>,
    pub env: Rc<RefCell<Env>>,
    pub base: usize,
    pub nresults: i32, // C - 1; -1 = MULTRET (C = 0)
}

impl LuaClosure {
    pub fn new(proto: Rc<Proto>) -> Self {
        Self {
            pc: 0,
            proto,
            env: Rc::new(RefCell::new(Env::default())),
            base: 0,
            nresults: -1,
        }
    }
}

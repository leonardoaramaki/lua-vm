use crate::value::LuaValue;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// env is where the globals table lives (G_)
#[derive(Debug, Default)]
pub struct Env {
    pub globals: HashMap<String, LuaValue>,
}

impl Env {
    pub fn insert_global(&mut self, k: &str, v: LuaValue) {
        self.globals.insert(String::from(k), v);
    }
}

#[derive(Debug, Default)]
pub struct LuaClosure {
    pub pc: i32,
    pub chunk_index: usize,
    pub env: Rc<RefCell<Env>>,
    pub constants: Vec<LuaValue>,
}

impl LuaClosure {
    pub fn new(chunk_index: usize) -> Self {
        Self {
            pc: 0,
            chunk_index,
            env: Rc::new(RefCell::new(Env::default())),
            constants: Vec::new(),
        }
    }
}

use std::rc::Rc;

use crate::value::LuaValue;

#[derive(Debug, Clone)]
pub struct Proto {
    bytecode: Vec<u32>,
    constants: Vec<LuaValue>,
    p: Vec<Rc<Proto>>,
}

impl Proto {
    pub fn new(bytecode: Vec<u32>, constants: Vec<LuaValue>, p: Vec<Rc<Proto>>) -> Self {
        Self {
            bytecode,
            constants,
            p,
        }
    }

    pub fn bytecode(&self) -> &[u32] {
        &self.bytecode
    }

    pub fn constants(&self) -> &[LuaValue] {
        &self.constants
    }

    pub fn protos(&self) -> &[Rc<Proto>] {
        &self.p
    }
}

use std::rc::Rc;

use crate::{chunk::FunctionBlock, value::LuaValue};

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

impl From<FunctionBlock> for Proto {
    fn from(value: FunctionBlock) -> Self {
        Self {
            bytecode: value.instructions.clone(),
            constants: value.constants.clone(),
            p: value
                .protos
                .into_iter()
                .map(|x| Rc::new(Proto::from(x)))
                .collect(),
        }
    }
}

use std::rc::Rc;

use crate::{chunk::FunctionBlock, value::LuaValue};

#[derive(Clone)]
pub struct Proto {
    bytecode: Vec<u32>,
    constants: Vec<LuaValue>,
    p: Vec<Rc<Proto>>,
    pub num_upvalues: u8,
    pub num_params: u8,
    pub max_stack_size: u8,
    pub source: String,
    pub line_info: Vec<u32>,
}

impl Proto {
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
            bytecode: value.instructions,
            constants: value.constants,
            p: value
                .protos
                .into_iter()
                .map(|x| Rc::new(Proto::from(x)))
                .collect(),
            num_upvalues: value.num_upvalues,
            num_params: value.num_params,
            max_stack_size: value.max_stack_size,
            source: value.source,
            line_info: value.line_info,
        }
    }
}

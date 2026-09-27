use std::{
    fs::File,
    io::{self, Read},
};

use crate::value::LuaValue;

#[derive(Debug, PartialEq)]
pub enum Endianess {
    Big,
    Little,
}

#[derive(Debug)]
pub struct HeaderBlock {
    version: u8,
    endianess: Endianess,
    size_of_int: usize,
    size_of_size_t: usize,
    size_of_instruction: usize,
    size_of_lua_number: usize,
}

impl HeaderBlock {
    pub fn default() -> Self {
        Self {
            version: 0x51,
            endianess: Endianess::Little,
            size_of_int: 4,
            size_of_size_t: 4,
            size_of_lua_number: 8,
            size_of_instruction: 4,
        }
    }
}

#[derive(Default)]
pub struct FunctionBlock {
    pub constants: Vec<LuaValue>,
    pub instructions: Vec<u32>,
    pub protos: Vec<FunctionBlock>,
}

impl FunctionBlock {
    pub fn new(
        constants: Vec<LuaValue>,
        instructions: Vec<u32>,
        protos: Vec<FunctionBlock>,
    ) -> Self {
        Self {
            constants,
            instructions,
            protos,
        }
    }
}

#[allow(unused)]
pub struct Chunk {
    file: File,
    header: HeaderBlock,
    function: FunctionBlock,
}

impl Chunk {
    pub fn new(file: File) -> Self {
        Self {
            file,
            header: HeaderBlock::default(),
            function: FunctionBlock::default(),
        }
    }

    fn read_header_block(&mut self) -> anyhow::Result<()> {
        let sig = self.read_bytes(4)?;
        assert_eq!(sig, b"\x1BLua");
        let version = self.read_byte()?;
        let _format = self.read_byte()?;
        let endianess = if self.read_byte()? == 0 {
            Endianess::Big
        } else {
            Endianess::Little
        };
        let size_of_int = self.read_byte()?;
        let size_of_size_t = self.read_byte()?;
        let size_of_instruction = self.read_byte()?;
        let size_of_lua_number = self.read_byte()?;
        let _integral_flag = self.read_byte()?;
        self.header.version = version;
        self.header.endianess = endianess;
        self.header.size_of_int = size_of_int as usize;
        self.header.size_of_size_t = size_of_size_t as usize;
        self.header.size_of_lua_number = size_of_lua_number as usize;
        self.header.size_of_instruction = size_of_instruction as usize;
        Ok(())
    }

    fn read_function_block(&mut self) -> anyhow::Result<FunctionBlock> {
        let _source_name = self.read_string()?;
        let _line_defined = self.read_integer()?;
        let _last_line_defined = self.read_integer()?;
        let _num_of_upvalues = self.read_byte()?;
        let _num_of_parameters = self.read_byte()?;
        let _is_vararg = self.read_byte()?;
        let _max_stack_size = self.read_byte()?;

        let mut function = FunctionBlock::default();
        // Instruction list
        let sizecode = self.read_integer()?;
        for _ in 0..sizecode {
            let instruction = self.read_instruction()?;
            function.instructions.push(instruction as u32);
        }

        // Constant list
        let sizek = self.read_integer()?;
        for _ in 0..sizek {
            match self.read_byte()? {
                0 => {
                    function.constants.push(LuaValue::Nil);
                }
                1 => {
                    let b = self.read_byte()?;
                    function.constants.push(LuaValue::Boolean(b != 0));
                }
                3 => {
                    let n = self.read_number()?;
                    function.constants.push(LuaValue::Number(f64::from_bits(n)));
                }
                4 => {
                    let s = self.read_string()?;
                    function.constants.push(LuaValue::String(s));
                }
                _ => unimplemented!(),
            }
        }

        let sizep = self.read_integer()?;
        for _ in 0..sizep {
            let proto = self.read_function_block()?;
            function.protos.push(proto);
        }

        Ok(function)
    }

    pub fn load(&mut self) -> anyhow::Result<FunctionBlock> {
        self.read_header_block()?;
        self.read_function_block()
    }

    pub fn read_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            let b = self.read_byte()?;
            result.push(b);
        }
        Ok(result)
    }

    pub fn read_byte(&mut self) -> io::Result<u8> {
        let mut buffer = [0u8; 1];
        self.file.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    pub fn read_instruction(&mut self) -> io::Result<u64> {
        let mut buffer = vec![0u8; self.header.size_of_instruction];
        self.file.read_exact(&mut buffer)?;
        Ok(if self.header.endianess == Endianess::Big {
            if self.header.size_of_instruction == 8 {
                u64::from_be_bytes(buffer.try_into().unwrap())
            } else {
                u32::from_be_bytes(buffer.try_into().unwrap()) as u64
            }
        } else {
            if self.header.size_of_instruction == 8 {
                u64::from_le_bytes(buffer.try_into().unwrap())
            } else {
                u32::from_le_bytes(buffer.try_into().unwrap()) as u64
            }
        })
    }

    pub fn read_number(&mut self) -> io::Result<u64> {
        let mut buffer = vec![0u8; self.header.size_of_lua_number];
        self.file.read_exact(&mut buffer)?;
        Ok(if self.header.endianess == Endianess::Big {
            if self.header.size_of_lua_number == 8 {
                u64::from_be_bytes(buffer.try_into().unwrap())
            } else {
                u32::from_be_bytes(buffer.try_into().unwrap()) as u64
            }
        } else {
            if self.header.size_of_lua_number == 8 {
                u64::from_le_bytes(buffer.try_into().unwrap())
            } else {
                u32::from_le_bytes(buffer.try_into().unwrap()) as u64
            }
        })
    }

    pub fn read_integer(&mut self) -> io::Result<u64> {
        let mut buffer = vec![0u8; self.header.size_of_int];
        self.file.read_exact(&mut buffer)?;
        Ok(if self.header.endianess == Endianess::Big {
            if self.header.size_of_int == 4 {
                u32::from_be_bytes(buffer.try_into().unwrap()) as u64
            } else {
                u64::from_be_bytes(buffer.try_into().unwrap())
            }
        } else {
            if self.header.size_of_int == 4 {
                u32::from_le_bytes(buffer.try_into().unwrap()) as u64
            } else {
                u64::from_le_bytes(buffer.try_into().unwrap())
            }
        })
    }

    pub fn read_usize(&mut self) -> io::Result<usize> {
        let mut buffer = vec![0u8; self.header.size_of_size_t];
        self.file.read_exact(&mut buffer)?;
        Ok(if self.header.endianess == Endianess::Big {
            usize::from_be_bytes(buffer.try_into().unwrap())
        } else {
            usize::from_le_bytes(buffer.try_into().unwrap())
        })
    }
    /// Read a String.
    /// All strings are defined in the following format:
    /// Size_t String data size
    /// Bytes String data, includes a NUL (ASCII 0) at the end
    pub fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_usize()?;
        let mut bytes: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            let b = self.read_byte()?;
            // ignore \0
            if b as char != '\0' {
                bytes.push(b);
            }
        }
        let s = String::from_utf8_lossy(&bytes).to_string();
        Ok(s)
    }
}

use arbitrary_int::prelude::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;

pub struct SAP1 {
    pub memory: [u8; 16],
    pub instruction_register: u8,
    pub memory_address_register: u4,
    pub accumulator: u8,
    pub register_b: u8,
    pub out_register: u8,
    pub program_counter: u4,
    pub is_halted: bool,
    pub stage: Stage,
    pub decoded_op: Opcode,
    pub decoded_operand: u4,

    // The bus "W", does nothing
    // other than for observer to
    // know what is in bus
    pub bus: u8,

    // ALU always calculating, directly from content of
    // accumulator and register B
    pub alu_output: u8,
    pub alu_opcode: ALUOpcode,
}

#[derive(FromPrimitive)]
#[repr(u8)]
pub enum Opcode {
    LoadA = 0b0000,
    Add = 0b0001,
    Sub = 0b0010,
    Out = 0b1110,
    Halt = 0b1111,
}

pub enum ALUOpcode {
    Add,
    Sub,
}

impl Default for ALUOpcode {
    fn default() -> Self {
        Self::Add
    }
}

pub enum Stage {
    Fetch,
    Decode,
    Execute1,
    Execute2,
    Execute3,
    Increment,
}

impl Default for SAP1 {
    fn default() -> Self {
        Self {
            memory: Default::default(),
            instruction_register: Default::default(),
            memory_address_register: Default::default(),
            accumulator: Default::default(),
            register_b: Default::default(),
            out_register: Default::default(),
            program_counter: Default::default(),
            is_halted: Default::default(),
            alu_output: Default::default(),
            stage: Stage::Fetch,
            decoded_op: Opcode::LoadA,
            decoded_operand: Default::default(),
            alu_opcode: Default::default(),
            bus: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Unknown operation, opcode: {opcode}, operand: {operand}")]
    UnknownOperation { opcode: u4, operand: u4 },
    #[error("Machine is halted")]
    MachineIsHalted,
}

impl SAP1 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self) -> Result<(), ExecutionError> {
        if self.is_halted {
            return Err(ExecutionError::MachineIsHalted);
        }

        let new_bus;
        match self.stage {
            Stage::Fetch => {
                new_bus = self.memory[usize::from(self.program_counter.value())];
                self.instruction_register = new_bus;
                self.stage = Stage::Decode;
            }
            Stage::Decode => {
                new_bus = 0;

                let opcode = u4::extract_u8(self.instruction_register, 4);
                let operand = u4::extract_u8(self.instruction_register, 0);

                let Some(opcode) = Opcode::from_u8(opcode.as_u8()) else {
                    return Err(ExecutionError::UnknownOperation { opcode, operand });
                };

                self.decoded_op = opcode;
                self.decoded_operand = operand;
                self.stage = Stage::Execute1;
            }
            Stage::Execute1 => {
                match self.decoded_op {
                    Opcode::LoadA | Opcode::Add | Opcode::Sub => {
                        new_bus = self.decoded_operand.as_u8();
                        self.memory_address_register = u4::from_u8(new_bus);
                    }
                    Opcode::Out => {
                        new_bus = self.accumulator;
                        self.out_register = new_bus;
                    }
                    Opcode::Halt => {
                        new_bus = 0;
                        self.is_halted = true;
                    }
                }

                self.stage = Stage::Execute2;
            }
            Stage::Execute2 => {
                match self.decoded_op {
                    Opcode::LoadA => {
                        new_bus = self.memory[usize::from(self.memory_address_register.as_u8())];
                        self.accumulator = new_bus;
                    }
                    Opcode::Add | Opcode::Sub => {
                        new_bus = self.memory[usize::from(self.memory_address_register.as_u8())];
                        self.register_b = new_bus;
                    }
                    Opcode::Out | Opcode::Halt => new_bus = 0,
                }

                self.stage = Stage::Execute3;
            }
            Stage::Execute3 => {
                match self.decoded_op {
                    Opcode::Add | Opcode::Sub => {
                        new_bus = self.alu_output;
                        self.accumulator = new_bus;
                    }
                    Opcode::LoadA | Opcode::Out | Opcode::Halt => new_bus = 0,
                }
                self.stage = Stage::Increment;
            }
            Stage::Increment => {
                new_bus = 0;
                self.program_counter = self.program_counter.wrapping_add(u4::from_u8(1));
                self.stage = Stage::Fetch;
            }
        }

        // Do ALU stuff, its always calculating. Do this as last iteration
        // just so previous accumulaor/register b update, ALU always
        // shows up to date data
        self.alu_opcode = match self.decoded_op {
            Opcode::LoadA => ALUOpcode::default(),
            Opcode::Add => ALUOpcode::Add,
            Opcode::Sub => ALUOpcode::Sub,
            Opcode::Out => ALUOpcode::default(),
            Opcode::Halt => ALUOpcode::default(),
        };

        match self.alu_opcode {
            ALUOpcode::Add => self.alu_output = self.accumulator.wrapping_add(self.register_b),
            ALUOpcode::Sub => self.alu_output = self.accumulator.wrapping_sub(self.register_b),
        }
        self.bus = new_bus;

        Ok(())
    }
}

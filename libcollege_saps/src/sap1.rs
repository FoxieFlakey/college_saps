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
    pub micro_operation_done: MicroOperation,
}

#[derive(Clone, Copy)]
pub enum MicroOperation {
    IncrementPC,
    MoveOperation { src: MoveSource, dest: Register },
    Decoded,
    Halt,
    Nothing,
}

#[derive(Clone, Copy)]
pub enum MoveSource {
    Accumulator,
    ALUOutput,
    DecodedOperand,
    ProgramCounter,
    Memory { addr: u4 },
}

#[derive(Clone, Copy)]
pub enum Register {
    A,
    B,
    InstructionRegister,
    Output,
    MemoryAddressRegister,
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
    Fetch1,
    Fetch2,
    Execute1,
    Execute2,
    Execute3,
    Fetch3,
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
            stage: Stage::Fetch1,
            decoded_op: Opcode::LoadA,
            decoded_operand: Default::default(),
            alu_opcode: Default::default(),
            bus: Default::default(),
            micro_operation_done: MicroOperation::IncrementPC,
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
            Stage::Fetch1 => {
                new_bus = self.program_counter.as_u8();
                self.memory_address_register = self.program_counter;
                self.stage = Stage::Fetch2;
                self.micro_operation_done = MicroOperation::MoveOperation {
                    src: MoveSource::ProgramCounter,
                    dest: Register::MemoryAddressRegister,
                };
            }
            Stage::Fetch2 => {
                new_bus = 0;
                self.program_counter = self.program_counter.wrapping_add(u4::from_u8(1));
                self.micro_operation_done = MicroOperation::IncrementPC;
                self.stage = Stage::Fetch3;
            }
            Stage::Fetch3 => {
                new_bus = self.memory[usize::from(self.memory_address_register.as_u8())];
                self.instruction_register = new_bus;

                let opcode = u4::extract_u8(self.instruction_register, 4);
                let operand = u4::extract_u8(self.instruction_register, 0);

                let Some(opcode) = Opcode::from_u8(opcode.as_u8()) else {
                    return Err(ExecutionError::UnknownOperation { opcode, operand });
                };

                self.decoded_op = opcode;
                self.decoded_operand = operand;

                self.micro_operation_done = MicroOperation::MoveOperation {
                    src: MoveSource::Memory {
                        addr: self.program_counter,
                    },
                    dest: Register::InstructionRegister,
                };
                self.stage = Stage::Execute1;
            }
            Stage::Execute1 => {
                match self.decoded_op {
                    Opcode::LoadA | Opcode::Add | Opcode::Sub => {
                        new_bus = self.decoded_operand.as_u8();
                        self.memory_address_register = u4::from_u8(new_bus);
                        self.micro_operation_done = MicroOperation::MoveOperation {
                            src: MoveSource::DecodedOperand,
                            dest: Register::MemoryAddressRegister,
                        };
                    }
                    Opcode::Out => {
                        new_bus = self.accumulator;
                        self.out_register = new_bus;
                        self.micro_operation_done = MicroOperation::MoveOperation {
                            src: MoveSource::Accumulator,
                            dest: Register::Output,
                        };
                    }
                    Opcode::Halt => {
                        new_bus = 0;
                        self.is_halted = true;
                        self.micro_operation_done = MicroOperation::Halt;
                    }
                }

                self.stage = Stage::Execute2;
            }
            Stage::Execute2 => {
                match self.decoded_op {
                    Opcode::LoadA => {
                        new_bus = self.memory[usize::from(self.memory_address_register.as_u8())];
                        self.accumulator = new_bus;
                        self.micro_operation_done = MicroOperation::MoveOperation {
                            src: MoveSource::Memory {
                                addr: self.memory_address_register,
                            },
                            dest: Register::A,
                        }
                    }
                    Opcode::Add | Opcode::Sub => {
                        new_bus = self.memory[usize::from(self.memory_address_register.as_u8())];
                        self.register_b = new_bus;
                        self.micro_operation_done = MicroOperation::MoveOperation {
                            src: MoveSource::Memory {
                                addr: self.memory_address_register,
                            },
                            dest: Register::B,
                        }
                    }
                    Opcode::Out | Opcode::Halt => {
                        new_bus = 0;
                        self.micro_operation_done = MicroOperation::Nothing;
                    }
                }

                self.stage = Stage::Execute3;
            }
            Stage::Execute3 => {
                match self.decoded_op {
                    Opcode::Add | Opcode::Sub => {
                        new_bus = self.alu_output;
                        self.accumulator = new_bus;
                        self.micro_operation_done = MicroOperation::MoveOperation {
                            src: MoveSource::ALUOutput,
                            dest: Register::A,
                        };
                    }
                    Opcode::LoadA | Opcode::Out | Opcode::Halt => {
                        self.micro_operation_done = MicroOperation::Nothing;
                        new_bus = 0;
                    }
                }
                self.stage = Stage::Fetch1;
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

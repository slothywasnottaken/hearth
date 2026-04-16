use std::ops::Index;

/// R4-R11
#[derive(Debug, Clone, Copy)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

const IP: Register = Register::R12;
const SP: Register = Register::R13;
const LR: Register = Register::R14;
const PC: Register = Register::R15;

impl Index<Register> for Registers {
    type Output = u64;

    fn index(&self, index: Register) -> &Self::Output {
        &self.registers[index as usize]
    }
}

#[derive(Debug)]
pub struct Registers {
    registers: [u64; 16],
}

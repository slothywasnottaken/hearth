use crate::vm::{memory::Memory, registers::Registers};

#[derive(Debug)]
pub struct CPU {
    memory: Memory,
    registers: Registers,
}

impl CPU {}

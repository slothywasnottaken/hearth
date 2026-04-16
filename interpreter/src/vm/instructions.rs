use crate::vm::{memory::Address, registers::Register};

#[derive(Debug, Clone, Copy)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    MovRegMem(Register, Address),
    MovRegReg(Register, Register),
    MovRegNum(Register, Value),

    MovMemReg(Address, Register),
    MovMemNum(Address, Value),

    AddRegReg(Register, Register),
    AddRegNum(Register, Value),
    AddRegMem(Register, Address),
    AddMemReg(Address, Register),

    IncReg(Register),
    IncMem(Address),

    PushReg(Register),
    PushMem(Address),
    PushVal(Value),

    PopReg(Register),

    CmpReg(Register, Register),
    CmpVal(Value, Value),

    Jump(Address),
    JumpGe(Address),
    JumpGte(Address),
    JumpLt(Address),
    JumpLte(Address),

    Call(Address),

    Load(Register, Address),

    StoreReg(Address, Register),
    StoreVal(Address, Value),

    Interrupt(u32),
    InterruptReg(Register),

    Halt,
    Ret,
}

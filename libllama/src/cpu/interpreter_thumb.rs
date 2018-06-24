use cpu::{Cpu, InstrStatus};

pub type InstFn = fn(&mut Cpu, u16) -> InstrStatus;
mod interpreter {
    use cpu;
    pub use cpu::instructions_thumb::*;
    pub fn undef(cpu: &mut cpu::Cpu, instr: u16) -> cpu::InstrStatus {
        panic!("Unimplemented instruction! {:#X}: {:?}", cpu.regs[15] - cpu.get_pc_offset(), instr)
    }
    #[allow(non_upper_case_globals)] pub const bkpt: super::InstFn = undef;
    #[allow(non_upper_case_globals)] pub const cmn: super::InstFn = undef;
    #[allow(non_upper_case_globals)] pub const swi: super::InstFn = undef;
}

include!(concat!(env!("OUT_DIR"), "/thumb.decoder.rs"));

#[inline]
pub fn interpret_next(cpu: &mut Cpu, addr: u32) -> InstrStatus {
    let instr = cpu.mpu.imem_read::<u16>(addr);
    let inst_fn = *cpu.thumb_decode_cache.get_or(instr as u32, &mut ());
    inst_fn(cpu, instr)
}

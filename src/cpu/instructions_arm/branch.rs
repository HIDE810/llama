use cpu;
use cpu::Cpu;
use utils::sign_extend;

#[inline(always)]
fn instr_branch_exchange(cpu: &mut Cpu, data: cpu::ArmInstrBranchExchange, link: bool) -> u32 {
    use cpu::ArmInstrBranchExchange as ArmInstr;

    if !cpu::cond_passed(data.get(ArmInstr::cond()), &cpu.cpsr) {
        return 4;
    }

    let addr = cpu.regs[data.get(ArmInstr::rm()) as usize];

    if link {
        cpu.regs[14] = cpu.regs[15] - 4;
    }

    cpu.cpsr.set(cpu::Psr::thumb_bit(), bit!(addr, 0));
    cpu.branch(addr & 0xFFFFFFFE);

    0
}

#[inline(always)]
pub fn bbl(cpu: &mut Cpu, data: cpu::ArmInstrBBL) -> u32 {
    use cpu::ArmInstrBBL as ArmInstr;

    if !cpu::cond_passed(data.get(ArmInstr::cond()), &cpu.cpsr) {
        return 4;
    }

    let signed_imm_24 = data.get(ArmInstr::signed_imm_24());

    if data.get(ArmInstr::link_bit()) == 1 {
        cpu.regs[14] = cpu.regs[15] - 4;
    }

    let pc = cpu.regs[15];
    cpu.branch(((pc as i32) + (sign_extend(signed_imm_24, 24) << 2)) as u32);

    0
}

#[inline(always)]
pub fn blx(cpu: &mut Cpu, data: cpu::ArmInstrBranchExchange) -> u32 {
    instr_branch_exchange(cpu, data, true)
}

#[inline(always)]
pub fn bx(cpu: &mut Cpu, data: cpu::ArmInstrBranchExchange) -> u32 {
    instr_branch_exchange(cpu, data, false)
}

#[inline(always)]
pub fn mod_blx(cpu: &mut Cpu, data: cpu::ArmInstrModBLX) -> u32 {
    use cpu::ArmInstrModBLX as ArmInstr;

    let signed_imm_24 = data.get(ArmInstr::signed_imm_24());
    let h_bit = data.get(ArmInstr::h_bit());

    cpu.regs[14] = cpu.regs[15] - 4;
    cpu.cpsr.set(cpu::Psr::thumb_bit(), 1);

    let pc = cpu.regs[15];
    cpu.branch((pc as i32 + (sign_extend(signed_imm_24, 24) << 2)) as u32 + (h_bit << 1));

    0
}
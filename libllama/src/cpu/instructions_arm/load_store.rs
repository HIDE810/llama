use cpu;
use cpu::Cpu;

#[inline(always)]
fn decode_addressing_mode(instr_data: &cpu::ArmInstrLoadStore, cpu: &Cpu) -> u32 {
    use cpu::ArmInstrLoadStore as ArmInstr;

    let c_bit = cpu.cpsr.get(cpu::Psr::c_bit()) == 1;

    let i_bit = instr_data.get(ArmInstr::i_bit());
    let u_bit = instr_data.get(ArmInstr::u_bit());
    let base_addr = cpu.regs[instr_data.get(ArmInstr::rn()) as usize];

    let offset = if i_bit == 0 {
        bits!(instr_data.raw(), 0 => 11)
    } else {
        let pre_shift = cpu.regs[bits!(instr_data.raw(), 0 => 3) as usize];

        let offset = if bits!(instr_data.raw(), 4 => 11) == 0 {
            pre_shift
        } else {
            let shift = bits!(instr_data.raw(), 5 => 6);
            let shift_imm = bits!(instr_data.raw(), 7 => 11);

            match shift {
                0b00 => pre_shift << shift_imm,
                0b01 => {
                    let index = if shift_imm == 0 {
                        0
                    } else {
                        pre_shift >> shift_imm
                    }; index
                },
                0b10 => {
                    let index = if shift_imm == 0 {
                        let index = if bit!(pre_shift, 31) == 1 {
                            0xFFFFFFFF
                        } else {
                            0
                        }; index
                    } else {
                        ((pre_shift as i32) >> shift_imm) as u32
                    }; index
                },
                0b11 => {
                    let index = if shift_imm == 0 {
                        ((c_bit as u32) << 31) | (pre_shift >> 1)
                    } else {
                        pre_shift.rotate_right(shift_imm)
                    }; index
                }
                _ => {
                    panic!("Unhandled shifter operation!");
                }
            }
        }; offset
    };

    if u_bit == 1 {
        return base_addr + offset;
    } else {
        return base_addr - offset;
    }
}

#[inline(always)]
fn instr_load(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore, byte: bool) -> u32 {
    use cpu::ArmInstrLoadStore as ArmInstr;

    if !cpu::cond_passed(data.get(ArmInstr::cond()), &cpu.cpsr) {
        return 4;
    }

    let rd = data.get(ArmInstr::rd());
    let addr = decode_addressing_mode(&data, cpu);

    // TODO: determine behavior based on CP15 r1 bit_U (22)
    let val = if byte {
        cpu.memory.read::<u8>(addr) as u32
    } else {
        cpu.memory.read::<u32>(addr.rotate_right(8 * bits!(addr, 0 => 1)))
    };

    // TODO: Implement
    assert!(data.get(ArmInstr::p_bit()) == 1);
    assert!(data.get(ArmInstr::w_bit()) == 0);

    if rd == 15 {
        cpu.cpsr.set(cpu::Psr::thumb_bit(), bit!(val, 0));
        cpu.branch(val & 0xFFFFFFFE);
        return 0;
    } else {
        cpu.regs[rd as usize] = val;
    }

    4
}

#[inline(always)]
fn instr_store(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore, byte: bool) -> u32 {
    use cpu::ArmInstrLoadStore as ArmInstr;

    if !cpu::cond_passed(data.get(ArmInstr::cond()), &cpu.cpsr) {
        return 4;
    }

    let addr = decode_addressing_mode(&data, cpu);
    let val = cpu.regs[data.get(ArmInstr::rd()) as usize];

    // TODO: Implement
    assert!(data.get(ArmInstr::p_bit()) == 1);
    assert!(data.get(ArmInstr::w_bit()) == 0);

    if byte {
        cpu.memory.write::<u8>(addr, val as u8);
    } else {
        cpu.memory.write::<u32>(addr, val);
    };

    4
}

#[inline(always)]
pub fn ldr(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore) -> u32 {
    instr_load(cpu, data, false)
}

#[inline(always)]
pub fn ldrb(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore) -> u32 {
    instr_load(cpu, data, true)
}

#[inline(always)]
pub fn str(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore) -> u32 {
    instr_store(cpu, data, false)
}

#[inline(always)]
pub fn strb(cpu: &mut Cpu, data: cpu::ArmInstrLoadStore) -> u32 {
    instr_store(cpu, data, true)
}
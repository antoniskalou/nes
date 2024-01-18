use bitflags::bitflags;
use crate::memory::Memory;

// 2KB working RAM for the CPU
const WRAM_SIZE: usize = 0x0800;
// not the real size of a rom, just for now
const ROM_SIZE: usize = 0x0F00;

/// see https://www.nesdev.org/obelisk-6502-guide/addressing.html
#[derive(Debug)]
pub enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

bitflags! {
    /// see https://www.nesdev.org/obelisk-6502-guide/registers.html
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct Status: u8 {
        const C = 0b0000_0001; // Carry
        const Z = 0b0000_0010; // Zero
        const I = 0b0000_0100; // Disable Interrupt
        const D = 0b0000_1000; // Decimal Mode
        const B = 0b0001_0000; // Break
        const U = 0b0010_0000; // Unused
        const V = 0b0100_0000; // Overflow
        const N = 0b1000_0000; // Negative
    }
}

impl Status {
    fn set_zn_flags(&mut self, val: u8) {
        self.set(Status::Z, val == 0);
        self.set(Status::N, val & 0x80 == 0x80);
    }
}

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
// FIXME: only zero-page access currently supported
enum Instruction {
    ADC(u8),
    AND(u8),
    ASL(u8),
    BRK,
    BCC(u8),
    CLC,
    CLD,
    CLI,
    CLV,
    DEX,
    DEY,
    INC(u8),
    INX,
    INY,
    LDA(u8), // Immediate
    LDX(u8), // Immediate
    LDY(u8), // Immediate
    NOP,
    SEC,
    SED,
    SEI,
    STA(u8),
    TAX,
    TAY,
    TXA,
    TYA,
    Illegal(u8),
}

// naming conventions from https://www.masswerk.at/6502/6502_instruction_set.html
#[derive(Debug)]
pub struct CPU {
    // acccumulator
    acc: u8,
    // X register
    x: u8,
    // Y register
    y: u8,
    // status register [NV-BDIZC]
    sr: Status,
    // stacck pointer
    sp: u8,
    // program counter
    pc: u16,
    wram: Memory<WRAM_SIZE>,
    rom: Memory<ROM_SIZE>,
}

impl CPU {
    pub fn new(rom: Memory<ROM_SIZE>) -> CPU {
        let wram = Memory::new();
        CPU {
            acc: 0,
            x: 0,
            y: 0,
            sr: Status::U & Status::I,
            sp: 0xFD,
            pc: 0,
            rom,
            wram,
        }
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.rom.read_u8(self.pc);
        self.pc += 1;
        opcode
    }

    // may step PC if opcode requires data
    fn decode(&mut self, opcode: u8) -> Instruction {
        use Instruction::*;
        match opcode {
            0x00 => BRK,
            0x06 => ASL(self.fetch()),
            0x18 => CLC,
            0x25 => AND(self.fetch()),
            0x38 => SEC,
            0x58 => CLI,
            0x65 => ADC(self.fetch()),
            0x78 => SEI,
            0x85 => STA(self.fetch()),
            0x88 => DEY,
            0x8A => TXA,
            0x90 => BCC(self.fetch()),
            0x98 => TYA,
            0xA4 => LDY(self.fetch()),
            0xA6 => LDX(self.fetch()),
            0xA8 => TAY,
            0xA9 => LDA(self.fetch()),
            0xAA => TAX,
            0xB8 => CLV,
            0xC8 => INY,
            0xCA => DEX,
            0xD8 => CLD,
            0xE6 => INC(self.fetch()),
            0xE8 => INX,
            0xEA => NOP,
            0xF8 => SED,
            _ => Illegal(opcode)
        }
    }

    fn execute(&mut self, inst: Instruction) {
        use Instruction::*;
        match inst {
            BRK => {
                // TODO
                // loop forever until we come up with a better
                // way of handling this
                todo!("interrupts");
            }
            ASL(addr) => {
                let data = self.wram.read_u8(addr as u16);
                self.sr.set(Status::C, (data >> 7) & 1 > 0);
                let x = data.wrapping_shl(1);
                self.sr.set_zn_flags(x);
                self.wram.write_u8(addr as u16, x);
            }
            AND(addr) => {
                let data = self.wram.read_u8(addr as u16);
                self.acc &= data;
                self.sr.set_zn_flags(self.acc);
            }
            ADC(addr) => {
                let data = self.wram.read_u8(addr as u16);
                let (x, o) = self.acc.overflowing_add(data);
                self.acc = x;
                self.sr.set_zn_flags(self.acc);
                self.sr.set(Status::C, o);
                // TODO: overflow flag
            }
            CLC => {
                self.sr.set(Status::C, false);
            }
            CLD => {
                self.sr.set(Status::D, false);
            }
            CLI => {
                self.sr.set(Status::I, false);
            }
            CLV => {
                self.sr.set(Status::V, false);
            }
            DEX => {
                self.x = self.x.wrapping_sub(1);
                self.sr.set_zn_flags(self.x);
            }
            DEY => {
                self.y = self.y.wrapping_sub(1);
                self.sr.set_zn_flags(self.y);
            }
            LDA(data) => {
                self.acc = data;
                self.sr.set_zn_flags(self.acc);
            }
            SEC => {
                self.sr.set(Status::C, true);
            }
            SED => {
                self.sr.set(Status::D, true);
            }
            SEI => {
                self.sr.set(Status::I, true);
            }
            STA(addr) => {
                self.wram.write_u8(addr as u16, self.acc)
            }
            BCC(offset) => {
                if self.sr.contains(Status::C) {
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
            }
            INC(addr) => {
                let data = self.wram.read_u8(addr as u16);
                let x = data.wrapping_add(1);
                self.wram.write_u8(addr as u16, x);
                self.sr.set_zn_flags(x);
            }
            LDX(data) => {
                self.x = data;
                self.sr.set_zn_flags(self.x);
            }
            LDY(data) => {
                self.y = data;
                self.sr.set_zn_flags(self.y);
            }
            NOP => {}
            INX => {
                self.x = self.x.wrapping_add(1);
                self.sr.set_zn_flags(self.x);
            }
            INY => {
                self.y = self.y.wrapping_add(1);
                self.sr.set_zn_flags(self.y);
            }
            TAX => {
                self.x = self.acc;
                self.sr.set_zn_flags(self.x);
            }
            TAY => {
                self.y = self.acc;
                self.sr.set_zn_flags(self.y);
            }
            TXA => {
                self.acc = self.x;
                self.sr.set_zn_flags(self.acc);
            }
            TYA => {
                self.acc = self.y;
                self.sr.set_zn_flags(self.acc);
            }
            Illegal(opcode) => panic!("illegal opcode: 0x{:02X}", opcode)
        }
    }

    pub fn tick(&mut self) {
        let opcode = self.fetch();
        let inst = self.decode(opcode);
        self.execute(inst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(bytes: &[u8]) -> CPU {
        CPU::new(Memory::with_program(bytes))
    }

    #[test]
    fn test_0x00_brk() {
        // TODO
    }

    #[test]
    fn test_0x06_asl() {
        let mut cpu = program(&[0x06, 0x20]);
        cpu.wram.write_u8(0x20, 0b0000_0001);
        cpu.tick();
        assert_eq!(cpu.wram.read_u8(0x20), 0b0000_0010);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x06_asl_zero_flag() {
        let mut cpu = program(&[0x06, 0x20]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x06_asl_negative_flag() {
        let mut cpu = program(&[0x06, 0x20]);
        cpu.wram.write_u8(0x20, 0x40);
        cpu.tick();
        // multiplies by 2
        assert_eq!(cpu.wram.read_u8(0x20), 0x80);
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x06_asl_carry_flag() {
        let mut cpu = program(&[0x06, 0x20]);
        cpu.wram.write_u8(0x20, 0b1000_0000);
        cpu.tick();
        assert!(cpu.sr.contains(Status::C));
    }

    #[test]
    fn test_0x18_clc() {
        let mut cpu = program(&[0x18]);
        cpu.sr.set(Status::C, true);
        cpu.tick();
        assert_eq!(cpu.sr.contains(Status::C), false);
    }

    #[test]
    fn test_0xd8_cld() {
        let mut cpu = program(&[0xD8]);
        cpu.sr.set(Status::D, true);
        cpu.tick();
        assert_eq!(cpu.sr.contains(Status::D), false);
    }

    #[test]
    fn test_0x58_cli() {
        let mut cpu = program(&[0x58]);
        cpu.sr.set(Status::I, true);
        cpu.tick();
        assert_eq!(cpu.sr.contains(Status::I), false);
    }

    #[test]
    fn test_0xb8_clv() {
        let mut cpu = program(&[0xB8]);
        cpu.sr.set(Status::V, true);
        cpu.tick();
        assert_eq!(cpu.sr.contains(Status::V), false);
    }

    #[test]
    fn test_0x25_and() {
        let mut cpu = program(&[0x25, 0x20]);
        cpu.wram.write_u8(0x20, 0b1010);
        cpu.acc = 0b1111;
        cpu.tick();
        assert_eq!(cpu.acc, 0b1010);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x25_and_zero_flag() {
        let mut cpu = program(&[0x25, 0]);
        cpu.acc = 0;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x25_and_negative_flag() {
        let mut cpu = program(&[0x25, 0x20]);
        cpu.wram.write_u8(0x20, 0xFF);
        cpu.acc = 0x80;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xa9_lda_immediate() {
        let mut cpu = program(&[0xA9, 0x40]);
        cpu.tick();
        assert_eq!(cpu.acc, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = program(&[0xA9, 0x00]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xa9_lda_negative_flag() {
        let mut cpu = program(&[0xA9, 0x80]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x85_sta() {
        let mut cpu = program(&[0x85, 0xFF]);
        cpu.acc = 0xFF;
        cpu.tick();
        assert_eq!(cpu.wram.read_u8(0xFF), 0xFF);
    }

    #[test]
    fn test_0x90_bcc_with_carry() {
        let mut cpu = program(&[0x90, 0x00, 0xC8]);
        cpu.sr.set(Status::C, true);
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.y, 1);
    }

    #[test]
    fn test_0x90_bcc_offset() {
        let mut cpu = program(&[
            0x90, 0x02,
            0xFF, 0xFF, // should never be reached
            0xC8,
        ]);
        cpu.sr.set(Status::C, true);
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.y, 1);
    }

    #[test]
    fn test_0x90_bcc_no_carry() {
        let mut cpu = program(&[
            0x90, 0x02,
            0xA9, 0xFF,
            0xC8, // shouldn't be reached in test
        ]);
        cpu.sr.set(Status::C, false);
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.acc, 0xFF);
        // shouldn't have run INY
        assert_eq!(cpu.y, 0);
    }

    #[test]
    fn test_0x65_adc() {
        let mut cpu = program(&[0x65, 0x20]);
        cpu.wram.write_u8(0x20, 0x40);
        cpu.acc = 0x04;
        cpu.tick();
        assert_eq!(cpu.acc, 0x44);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x65_adc_zero_flag() {
        let mut cpu = program(&[0x65, 0x20]);
        cpu.acc = 0;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x65_adc_negative_flag() {
        let mut cpu = program(&[0x65, 0x20]);
        cpu.wram.write_u8(0x20, 1);
        cpu.acc = 0x7F;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x65_adc_carry_flag() {
        let mut cpu = program(&[0x65, 0x20]);
        cpu.wram.write_u8(0x20, 1);
        cpu.acc = 0xFF;
        cpu.tick();
        assert!(cpu.sr.contains(Status::C));
    }

    #[test]
    fn test_0xe6_inc() {
        let mut cpu = program(&[0xE6, 0x20]);
        cpu.wram.write_u8(0x20, 0x40);
        cpu.tick();
        assert_eq!(cpu.wram.read_u8(0x20), 0x41);
    }

    #[test]
    fn test_0xe6_inc_zero_flag() {
        let mut cpu = program(&[0xE6, 0x20]);
        cpu.wram.write_u8(0x20, 0xFF);
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xe6_inc_negative_flag() {
        let mut cpu = program(&[0xE6, 0x20]);
        cpu.wram.write_u8(0x20, 0x7F);
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xa6_ldx() {
        let mut cpu = program(&[0xA6, 0x40]);
        cpu.tick();
        assert_eq!(cpu.x, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xa6_ldx_zero_flag() {
        let mut cpu = program(&[0xA6, 0]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xa6_ldx_negative_flag() {
        let mut cpu = program(&[0xA6, 0x80]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xa4_ldy() {
        let mut cpu = program(&[0xA4, 0x40]);
        cpu.tick();
        assert_eq!(cpu.y, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xa4_ldy_zero_flag() {
        let mut cpu = program(&[0xA4, 0x00]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xa4_ldy_negative_flag() {
        let mut cpu = program(&[0xA4, 0x80]);
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xe8_inx() {
        let mut cpu = program(&[0xE8]);
        cpu.x = 0x40;
        cpu.tick();
        assert_eq!(cpu.x, 0x41);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xe8_inx_zero_flag() {
        let mut cpu = program(&[0xE8]);
        cpu.x = 0xFF;
        cpu.tick();
        assert_eq!(cpu.x, 0);
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xe8_inx_negative_flag() {
        let mut cpu = program(&[0xE8]);
        cpu.x = 0x7F;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xc8_iny() {
        let mut cpu = program(&[0xC8]);
        cpu.y = 0x40;
        cpu.tick();
        assert_eq!(cpu.y, 0x41);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xc8_iny_zero_flag() {
        let mut cpu = program(&[0xC8]);
        cpu.y = 0xFF;
        cpu.tick();
        assert_eq!(cpu.y, 0);
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xc8_iny_negative_flag() {
        let mut cpu = program(&[0xC8]);
        cpu.y = 0x7F;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xca_dex() {
        let mut cpu = program(&[0xCA]);
        cpu.x = 2;
        cpu.tick();
        assert_eq!(cpu.x, 1);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xca_dex_underflow() {
        let mut cpu = program(&[0xCA]);
        cpu.x = 0;
        cpu.tick();
        assert_eq!(cpu.x, 0xFF);
    }

    #[test]
    fn test_0xca_dex_zero_flag() {
        let mut cpu = program(&[0xCA]);
        cpu.x = 1;
        cpu.tick();
        assert_eq!(cpu.x, 0);
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xca_dex_negative_flag() {
        let mut cpu = program(&[0xCA]);
        cpu.x = 0xFF;
        cpu.tick();
        assert_eq!(cpu.x, 0xFE);
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x88_dey() {
        let mut cpu = program(&[0x88]);
        cpu.y = 2;
        cpu.tick();
        assert_eq!(cpu.y, 1);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x88_dey_underflow() {
        let mut cpu = program(&[0x88]);
        cpu.y = 0;
        cpu.tick();
        assert_eq!(cpu.y, 0xFF);
    }

    #[test]
    fn test_0x88_dey_zero_flag() {
        let mut cpu = program(&[0x88]);
        cpu.y = 1;
        cpu.tick();
        assert_eq!(cpu.y, 0);
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x88_dey_negative_flag() {
        let mut cpu = program(&[0x88]);
        cpu.y = 0xFF;
        cpu.tick();
        assert_eq!(cpu.y, 0xFE);
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xea_nop() {
        let mut cpu = program(&[0xEA]);
        cpu.tick();
        // as long as we don't panic, we're good
    }

    #[test]
    fn test_0x38_sec() {
        let mut cpu = program(&[0x38]);
        cpu.sr.set(Status::C, false);
        cpu.tick();
        assert!(cpu.sr.contains(Status::C));
    }

    #[test]
    fn test_0xf8_sed() {
        let mut cpu = program(&[0xF8]);
        cpu.sr.set(Status::D, false);
        cpu.tick();
        assert!(cpu.sr.contains(Status::D));
    }

    #[test]
    fn test_0x78_sei() {
        let mut cpu = program(&[0x78]);
        cpu.sr.set(Status::I, false);
        cpu.tick();
        assert!(cpu.sr.contains(Status::I));
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = program(&[0xAA]);
        cpu.x = 0x00;
        cpu.acc = 0x40;
        cpu.tick();
        assert_eq!(cpu.x, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xaa_tax_zero_flag() {
        let mut cpu = program(&[0xAA]);
        cpu.x = 0x40;
        cpu.acc = 0x00;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xaa_tax_negative_flag() {
        let mut cpu = program(&[0xAA]);
        cpu.x = 0x00;
        cpu.acc = 0x80;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0xa8_tay() {
        let mut cpu = program(&[0xA8]);
        cpu.y = 0x00;
        cpu.acc = 0x40;
        cpu.tick();
        assert_eq!(cpu.y, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0xa8_tay_zero_flag() {
        let mut cpu = program(&[0xA8]);
        cpu.y = 0x40;
        cpu.acc = 0x00;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0xa8_tay_negative_flag() {
        let mut cpu = program(&[0xA8]);
        cpu.y = 0x00;
        cpu.acc = 0x80;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x8a_txa() {
        let mut cpu = program(&[0x8A]);
        cpu.acc = 0x00;
        cpu.x = 0x40;
        cpu.tick();
        assert_eq!(cpu.acc, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x8a_txa_zero_flag() {
        let mut cpu = program(&[0x8A]);
        cpu.acc = 0x40;
        cpu.x = 0x00;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x8a_txa_negative_flag() {
        let mut cpu = program(&[0x8A]);
        cpu.acc = 0x00;
        cpu.x = 0x80;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }

    #[test]
    fn test_0x98_txa() {
        let mut cpu = program(&[0x98]);
        cpu.acc = 0x00;
        cpu.y = 0x40;
        cpu.tick();
        assert_eq!(cpu.acc, 0x40);
        assert!(cpu.sr.is_empty());
    }

    #[test]
    fn test_0x98_txa_zero_flag() {
        let mut cpu = program(&[0x98]);
        cpu.acc = 0x40;
        cpu.y = 0x00;
        cpu.tick();
        assert!(cpu.sr.contains(Status::Z));
    }

    #[test]
    fn test_0x98_txa_negative_flag() {
        let mut cpu = program(&[0x98]);
        cpu.acc = 0x00;
        cpu.y = 0x80;
        cpu.tick();
        assert!(cpu.sr.contains(Status::N));
    }
}

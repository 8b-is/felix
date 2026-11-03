//! `felix/crates/felix-lang/src/codegen_v2.rs`
//! Fixed 64-bit `AGP-ISA` (`Admissible Graph Projection ISA`) Instruction Word Emitter for Felix v2.0 (`8B-META-WP-v2.0`).

/// 64-bit Instruction Layout:
/// - `[63..56]` Opcode (`8b`)
/// - `[55..48]` Required `CapMask` (`5b`) + `TrustRung` (`3b`)
/// - `[47..40]` `TOSS` Step Index (`8b`)
/// - `[39..20]` Dst / Linear Slot ID (`20b`)
/// - `[19..00]` Src / Immediate Operand (`20b`)
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgpWord64(pub u64);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgpOp {
    VerifyHeader64 = 0x01,   // Enforces 64B MEMNET epoch & MAC before execution
    GatherBfp64Q16 = 0x02,   // Keyed 4-head Lemire p/q gather in Q16.16
    StepWaveQ24 = 0x03,      // Steps 64x64 toroidal wave lattice & updates R_{t+1}
    FoldSigmaDelta64 = 0x04, // Packs complex wave row into 96B with parity byte
    ContractPopcnt64 = 0x05, // Parity-checked POPCNT inner product
    ConsumeLinearCap = 0x06, // Zeroizes & retires linear capability handle (π = 1)
    HaltSettled = 0xFF,      // Terminal TOSS crystal state (F_8 = 1)
}

impl AgpWord64 {
    #[inline(always)]
    pub const fn encode(
        op: AgpOp,
        cap_and_rung: u8,
        toss_step: u8,
        dst_slot: u32,
        src_imm: u32,
    ) -> Self {
        let w = ((op as u64) << 56)
            | ((cap_and_rung as u64) << 48)
            | ((toss_step as u64) << 40)
            | (((dst_slot & 0xFFFFF) as u64) << 20)
            | ((src_imm & 0xFFFFF) as u64);
        Self(w)
    }

    #[inline(always)]
    pub const fn opcode_byte(self) -> u8 {
        (self.0 >> 56) as u8
    }

    #[inline(always)]
    pub const fn cap_and_rung(self) -> u8 {
        ((self.0 >> 48) & 0xFF) as u8
    }

    #[inline(always)]
    pub const fn toss_step(self) -> u8 {
        ((self.0 >> 40) & 0xFF) as u8
    }
}

/// Emits the canonical 7-instruction v2.0 `AuroraDyadCapsule` binary program.
pub const fn compile_aurora_dyad_capsule_v2() -> [AgpWord64; 7] {
    const CAP_INFER_RUNG3: u8 = (0x04 << 3) | 0x03; // CAP_AYEOS_INFER | TrustRung::HybridPqBound
    [
        AgpWord64::encode(AgpOp::VerifyHeader64, CAP_INFER_RUNG3, 0, 0, 0),
        AgpWord64::encode(AgpOp::GatherBfp64Q16, CAP_INFER_RUNG3, 1, 1, 4),
        AgpWord64::encode(AgpOp::StepWaveQ24, CAP_INFER_RUNG3, 4, 2, 1), // Monozukuri plateau F_4=4
        AgpWord64::encode(AgpOp::FoldSigmaDelta64, CAP_INFER_RUNG3, 5, 3, 2), // Monozukuri plateau F_5=4
        AgpWord64::encode(AgpOp::ContractPopcnt64, CAP_INFER_RUNG3, 7, 4, 3),
        AgpWord64::encode(AgpOp::ConsumeLinearCap, CAP_INFER_RUNG3, 8, 0, 0), // Enforces π = 1 consumption
        AgpWord64::encode(AgpOp::HaltSettled, CAP_INFER_RUNG3, 8, 4, 0), // Terminal crystal F_8=1
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeck::toss_freedom;

    #[test]
    fn test_agp_isa_v2_aurora_dyad_capsule_encoding_and_toss_monotonicity() {
        let prog = compile_aurora_dyad_capsule_v2();
        assert_eq!(prog.len(), 7);
        assert_eq!(prog[0].opcode_byte(), AgpOp::VerifyHeader64 as u8);
        assert_eq!(prog[6].opcode_byte(), AgpOp::HaltSettled as u8);

        // Verify TOSS step indices are non-decreasing and freedom levels are non-increasing
        let mut prev_step = 0u8;
        let mut prev_freedom = 8u8;
        for w in &prog {
            let s = w.toss_step();
            let f = toss_freedom(s as usize);
            assert!(s >= prev_step);
            assert!(f <= prev_freedom);
            assert_eq!(w.cap_and_rung() & 0x07, 3, "TrustRung::HybridPqBound == 3");
            prev_step = s;
            prev_freedom = f;
        }
        assert_eq!(prev_freedom, 1);
    }
}

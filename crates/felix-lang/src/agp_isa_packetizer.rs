//! `src/agp_isa_packetizer.rs`
//!
//! AGP-ISA (asm-graph-phoenix) 64-bit instruction packetizer, capability gating,
//! and AXI4-Stream 512-bit streaming execution.

use crate::{
    felix_phoenix::CapEffectMask,
    fpga_axis_ternary::{
        AxisBeat512, FpgaTernary64x64AxisCore, AXIS_CMD_DECODE_FUTHARK27, AXIS_CMD_EXEC_NAND_ARRAY,
        AXIS_CMD_EXEC_TGEMM_PQ, AXIS_CMD_EXEC_TGEMM_RAW, AXIS_STATUS_OK,
    },
    ljodatal_18_charms::CAP_ROOT_SEAL,
    omni_subsystems::futhark_cipher_simd::RokCipherKey,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// 1. AGP-ISA Opcode Constants & Instruction Encoding
// ============================================================================

pub const AGP_OP_NOP: u8 = 0x01;
pub const AGP_OP_YIELD: u8 = 0x02;
pub const AGP_OP_CAP_ASSERT: u8 = 0x03;
pub const AGP_OP_SYNC_BARRIER: u8 = 0x04;
pub const AGP_OP_LD_WEIGHT: u8 = 0x10;
pub const AGP_OP_LD_ACT_LO: u8 = 0x11;
pub const AGP_OP_LD_ACT_HI: u8 = 0x12;
pub const AGP_OP_TGEMM_RAW: u8 = 0x20;
pub const AGP_OP_TGEMM_PQ: u8 = 0x21;
pub const AGP_OP_FUTHARK_DEC: u8 = 0x22;
pub const AGP_OP_DECODE_FUTHARK27: u8 = 0x22;
pub const AGP_OP_NAND_VEC: u8 = 0x23;
pub const AGP_OP_TERM_FLUSH: u8 = 0x30;

pub const AGP_FLAG_STANDARD: u8 = 0b00;
pub const AGP_FLAG_BARRIER_YIELD: u8 = 0b01;
pub const AGP_FLAG_STREAM_CONT: u8 = 0b10;
pub const AGP_FLAG_TERMINAL: u8 = 0b11;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgpInstruction(pub u64);

impl AgpInstruction {
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        opcode: u8,
        flag: u8,
        rok_invert: bool,
        bend_enable: bool,
        dst: u8,
        src1: u8,
        src2: u8,
        imm20: u32,
        cap_token: u16,
    ) -> Self {
        let op_part = ((opcode as u64) & 0x3F) << 58;
        let fl_part = ((flag as u64) & 0x03) << 56;
        let ro_part = (if rok_invert { 1u64 } else { 0u64 }) << 55;
        let en_part = (if bend_enable { 1u64 } else { 0u64 }) << 54;
        let dst_part = ((dst as u64) & 0x3F) << 48;
        let s1_part = ((src1 as u64) & 0x3F) << 42;
        let s2_part = ((src2 as u64) & 0x3F) << 36;
        let imm_part = ((imm20 as u64) & 0x000F_FFFF) << 16;
        let cap_part = (cap_token as u64) & 0xFFFF;

        Self(op_part | fl_part | ro_part | en_part | dst_part | s1_part | s2_part | imm_part | cap_part)
    }

    #[inline(always)]
    pub fn opcode(self) -> u8 {
        ((self.0 >> 58) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn flag(self) -> u8 {
        ((self.0 >> 56) & 0x03) as u8
    }

    #[inline(always)]
    pub fn rok_invert(self) -> bool {
        ((self.0 >> 55) & 0x01) != 0
    }

    #[inline(always)]
    pub fn bend_enable(self) -> bool {
        ((self.0 >> 54) & 0x01) != 0
    }

    #[inline(always)]
    pub fn dst(self) -> u8 {
        ((self.0 >> 48) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn src1(self) -> u8 {
        ((self.0 >> 42) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn src2(self) -> u8 {
        ((self.0 >> 36) & 0x3F) as u8
    }

    #[inline(always)]
    pub fn imm20(self) -> u32 {
        ((self.0 >> 16) & 0x000F_FFFF) as u32
    }

    #[inline(always)]
    pub fn cap_token(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

// ============================================================================
// 2. Capability DAG Node & Graph Representation
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgpCapabilityNode {
    pub node_id: usize,
    pub name: String,
    pub cap_mask: CapEffectMask,
    pub dependencies: Vec<usize>,
    pub instructions: Vec<AgpInstruction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgpCapabilityGraph {
    pub graph_id: u64,
    pub nodes: Vec<AgpCapabilityNode>,
}

// ============================================================================
// 3. DMA Packetizer & Hardware Stream Engine
// ============================================================================

pub struct AgpStreamPacketizer {
    pub graph: AgpCapabilityGraph,
    pub granted_caps: CapEffectMask,
    pub hardware_active_row: usize,
}

impl AgpStreamPacketizer {
    pub fn new(graph: AgpCapabilityGraph, granted_caps: CapEffectMask) -> Self {
        Self {
            graph,
            granted_caps,
            hardware_active_row: 0,
        }
    }

    /// Verifies static capability safety and transforms a node's execution segment into
    /// compliant 512-bit AXI4-Stream beats.
    pub fn packetize_node(
        &mut self,
        node_id: usize,
        weights_data: &Option<[[i8; 64]; 64]>,
        acts_data: &Option<[i16; 64]>,
        raw_futhark_bytes: &Option<[u8; 64]>,
    ) -> Result<Vec<AxisBeat512>, String> {
        let node = self
            .graph
            .nodes
            .iter()
            .find(|n| n.node_id == node_id)
            .ok_or_else(|| format!("Node ID {node_id} not found in AGP graph"))?;

        // 1. Static Security Invariant: Reject Charm XVIII (CAP_ROOT_SEAL) leak
        if (node.cap_mask & CAP_ROOT_SEAL) != 0 {
            return Err(format!(
                "Security Violation: Node '{name}' asserts forbidden CAP_ROOT_SEAL",
                name = node.name
            ));
        }

        // 2. Authority Check: Ensure declared capabilities are within host grant
        if (node.cap_mask & self.granted_caps) != node.cap_mask {
            return Err(format!(
                "Authority Violation: Node '{name}' demands cap {mask:#x} outside grant {grant:#x}",
                name = node.name,
                mask = node.cap_mask,
                grant = self.granted_caps
            ));
        }

        let mut output_beats = Vec::new();

        for insn in &node.instructions {
            let flag = insn.flag();
            let is_terminal = flag == AGP_FLAG_TERMINAL;

            match insn.opcode() {
                AGP_OP_NOP | AGP_OP_SYNC_BARRIER => continue,

                AGP_OP_YIELD => {
                    output_beats.push(AxisBeat512 {
                        tdata: [0u8; 64],
                        tuser: AXIS_CMD_EXEC_TGEMM_RAW as u16,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_CAP_ASSERT => {
                    let mut data = [0u8; 64];
                    data[0..4].copy_from_slice(&(node.cap_mask as u32).to_le_bytes());
                    output_beats.push(AxisBeat512 {
                        tdata: data,
                        tuser: 0,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_LD_WEIGHT => {
                    let weights = weights_data.as_ref().ok_or("Weight payload missing for LD_WEIGHT")?;
                    let key = RokCipherKey {
                        shift_stave_k0: insn.src1() % 3,
                        shift_row_k1: insn.src2() % 3,
                        shift_aett_k2: 1,
                        bend_k: if insn.bend_enable() { 2 } else { 0 },
                        rok_invert_aett: insn.rok_invert(),
                    };

                    for r in 0..64 {
                        let beat = FpgaTernary64x64AxisCore::build_load_row_beat(
                            r,
                            &weights[r],
                            4, // E2M1 scale low (1.0)
                            4, // E2M1 scale high (1.0)
                            key.rok_invert_aett,
                            key,
                        );
                        output_beats.push(beat);
                    }
                }

                AGP_OP_LD_ACT_LO => {
                    let acts = acts_data.as_ref().ok_or("Activation payload missing for LD_ACT_LO")?;
                    let mut tdata = [0u8; 64];
                    for c in 0..32 {
                        tdata[c * 2..c * 2 + 2].copy_from_slice(&acts[c].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata,
                        tuser: AXIS_CMD_EXEC_TGEMM_RAW as u16,
                        tlast: false,
                    });
                }

                AGP_OP_LD_ACT_HI => {
                    let acts = acts_data.as_ref().ok_or("Activation payload missing for LD_ACT_HI")?;
                    let mut tdata = [0u8; 64];
                    for c in 0..32 {
                        tdata[c * 2..c * 2 + 2].copy_from_slice(&acts[c + 32].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata,
                        tuser: 0,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_TGEMM_RAW => {
                    let acts = acts_data.as_ref().ok_or("Activation payload missing for TGEMM_RAW")?;
                    // Stream Beat 0 (Cols 0..31)
                    let mut tdata_lo = [0u8; 64];
                    for c in 0..32 {
                        tdata_lo[c * 2..c * 2 + 2].copy_from_slice(&acts[c].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata: tdata_lo,
                        tuser: AXIS_CMD_EXEC_TGEMM_RAW as u16,
                        tlast: false,
                    });

                    // Stream Beat 1 (Cols 32..63)
                    let mut tdata_hi = [0u8; 64];
                    for c in 0..32 {
                        tdata_hi[c * 2..c * 2 + 2].copy_from_slice(&acts[c + 32].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata: tdata_hi,
                        tuser: 0,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_TGEMM_PQ => {
                    let acts = acts_data.as_ref().ok_or("Activation payload missing for TGEMM_PQ")?;
                    let mut tdata_lo = [0u8; 64];
                    for c in 0..32 {
                        tdata_lo[c * 2..c * 2 + 2].copy_from_slice(&acts[c].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata: tdata_lo,
                        tuser: AXIS_CMD_EXEC_TGEMM_PQ as u16,
                        tlast: false,
                    });

                    let mut tdata_hi = [0u8; 64];
                    for c in 0..32 {
                        tdata_hi[c * 2..c * 2 + 2].copy_from_slice(&acts[c + 32].to_le_bytes());
                    }
                    output_beats.push(AxisBeat512 {
                        tdata: tdata_hi,
                        tuser: 0,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_FUTHARK_DEC => {
                    let bytes = raw_futhark_bytes.as_ref().ok_or("Payload missing for FUTHARK_DEC")?;
                    let mut beat = AxisBeat512 {
                        tdata: *bytes,
                        tuser: AXIS_CMD_DECODE_FUTHARK27 as u16,
                        tlast: is_terminal,
                    };
                    beat.tdata[14] = (if insn.rok_invert() { 0x40 } else { 0x00 })
                        | 0x80
                        | ((if insn.bend_enable() { 2 } else { 0 }) << 4);
                    beat.tdata[15] = (insn.src1() % 3) | ((insn.src2() % 3) << 2) | (1 << 4);
                    output_beats.push(beat);
                }

                AGP_OP_NAND_VEC => {
                    output_beats.push(AxisBeat512 {
                        tdata: [0u8; 64],
                        tuser: AXIS_CMD_EXEC_NAND_ARRAY as u16,
                        tlast: is_terminal,
                    });
                }

                AGP_OP_TERM_FLUSH => {
                    output_beats.push(AxisBeat512 {
                        tdata: [0u8; 64],
                        tuser: 0,
                        tlast: true,
                    });
                }

                _ => return Err(format!("Unrecognized AGP opcode: {:#x}", insn.opcode())),
            }
        }

        Ok(output_beats)
    }

    /// Dispatches an entire wave of DAG nodes into the hardware core model,
    /// tracking clock execution and output verification.
    pub fn execute_graph_pipeline(
        &mut self,
        hardware_core: &mut FpgaTernary64x64AxisCore,
        weights: &[[i8; 64]; 64],
        acts: &[i16; 64],
        futhark_sample: &[u8; 64],
    ) -> Result<Vec<[i32; 64]>, String> {
        let mut results = Vec::new();

        for node_idx in 0..self.graph.nodes.len() {
            let beats = self.packetize_node(
                node_idx,
                &Some(*weights),
                &Some(*acts),
                &Some(*futhark_sample),
            )?;

            for beat in beats {
                let cmd = (beat.tuser & 0x0F) as u8;

                if cmd == AXIS_CMD_EXEC_TGEMM_RAW || cmd == AXIS_CMD_EXEC_TGEMM_PQ {
                    let use_pq = cmd == AXIS_CMD_EXEC_TGEMM_PQ;
                    let (row_res, _) = hardware_core
                        .execute_tgemm_64x64_stream(acts, use_pq)
                        .map_err(|status| format!("Hardware fault: status {status:#x}"))?;
                    results.push(row_res);
                } else if let Some(reply) = hardware_core.send_single_beat(&beat) {
                    let status = (reply.tuser & 0x0F) as u8;
                    if status != AXIS_STATUS_OK {
                        return Err(format!("Hardware execution status error: {status:#x}"));
                    }
                }
            }
        }

        Ok(results)
    }
}

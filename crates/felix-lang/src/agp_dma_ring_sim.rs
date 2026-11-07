//! `felix/crates/felix-lang/src/agp_dma_ring_sim.rs`
//!
//! 512-Bit (64-Byte) Scatter-Gather DMA Descriptor Ring (`AgpSgDescriptor512`) & Host AXI4 Memory Simulator.

use crate::{agp_isa_packetizer::AgpInstruction, felix_phoenix::CapEffectMask};
use std::collections::BTreeMap;

/// Simulated 64-byte-cache-line-addressed Host AXI4 Memory.
#[derive(Clone, Debug, Default)]
pub struct SimulatedHostAxiMemory {
    pub cache_lines: BTreeMap<u64, [u8; 64]>,
}

impl SimulatedHostAxiMemory {
    pub fn new() -> Self {
        Self {
            cache_lines: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn read_cache_line(&self, phys_addr: u64) -> [u8; 64] {
        let aligned = phys_addr & !0x3F;
        self.cache_lines.get(&aligned).copied().unwrap_or([0u8; 64])
    }

    #[inline]
    pub fn write_cache_line(&mut self, phys_addr: u64, line: [u8; 64]) {
        let aligned = phys_addr & !0x3F;
        self.cache_lines.insert(aligned, line);
    }
}

/// 512-bit (`64-byte`) Scatter-Gather DMA Descriptor (`AgpSgDescriptor512`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgpSgDescriptor512 {
    pub src_phys_addr: u64,
    pub xfer_beats: u32,
    pub req_cap_mask: CapEffectMask,
    pub agp_insn: AgpInstruction,
    pub dep_wait_mask: u16,
    pub node_done_mask: u16,
    pub pcc_fnv1a_seal: u64,
    pub next_desc_off: u32,
    pub inline_meta_128: [u8; 16],
}

impl AgpSgDescriptor512 {
    pub fn to_cache_line_bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[0..8].copy_from_slice(&self.src_phys_addr.to_le_bytes());
        out[8..12].copy_from_slice(&self.xfer_beats.to_le_bytes());
        out[12..16].copy_from_slice(&self.req_cap_mask.to_le_bytes());
        out[16..24].copy_from_slice(&self.agp_insn.0.to_le_bytes());
        out[24..26].copy_from_slice(&self.dep_wait_mask.to_le_bytes());
        out[26..28].copy_from_slice(&self.node_done_mask.to_le_bytes());
        out[28..32].copy_from_slice(&self.next_desc_off.to_le_bytes());
        out[32..40].copy_from_slice(&self.pcc_fnv1a_seal.to_le_bytes());
        out[40..56].copy_from_slice(&self.inline_meta_128);
        out
    }
}

pub struct AgpDmaSgRingEngineSim {
    pub ring_base_phys: u64,
    pub ring_mask: u32,
    pub granted_caps: CapEffectMask,
}

impl AgpDmaSgRingEngineSim {
    pub fn new(ring_base_phys: u64, ring_mask: u32, granted_caps: CapEffectMask) -> Self {
        Self {
            ring_base_phys,
            ring_mask,
            granted_caps,
        }
    }
}

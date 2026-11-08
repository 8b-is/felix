//! Memory-Safe 64-Byte Aligned Wire Frame (`WireC1Frame`) & Idempotent Gossip Ingestion
//!
//! Ensures soundness when casting and transmitting fixed 64-byte wire frames:
//! 1. **No Enum Discriminant UB**: `WireC1Frame` stores `kind: u8` instead of `FrameKind`
//!    and validates via `FrameKind::from_u8` after casting.
//! 2. **Zero Implicit Padding UB**: Exact 64-byte field layout (`2 + 1 + 1 + 2 + 2 + 8 + 16 + 32 = 64` bytes)
//!    with `#[repr(C, align(8))]`, so `as_bytes()` never exposes uninitialized padding bytes.
//! 3. **Idempotent Causal Clock Guard**: `LocalClusterNode::ingest_wire_frame` verifies
//!    `incoming_clock == self.clocks[i].wrapping_add(1)` before applying deltas, ignoring duplicate packets.

use core::mem::{align_of, size_of};
use core::slice;

pub const WIRE_MAGIC: u16 = 0x8B15;
pub const MAX_NODES: usize = 4;
pub const DYADIC_SHIFT_DEFAULT: u8 = 16;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameKind {
    State0Cochain = 0x01,
    Soldering1Form = 0x02,
    Torsion2Form = 0x03,
}

impl FrameKind {
    #[inline(always)]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::State0Cochain),
            0x02 => Some(Self::Soldering1Form),
            0x03 => Some(Self::Torsion2Form),
            _ => None,
        }
    }
}

/// Aligned, zero-copy wire frame with zero implicit padding bytes.
/// Total size: 64 bytes (1 CPU cache line), 8-byte aligned.
#[repr(C, align(8))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WireC1Frame {
    pub magic: u16,                      // Offset 0..2
    pub kind: u8,                        // Offset 2..3 (stored as u8 to avoid Enum UB)
    pub dyadic_shift: u8,                // Offset 3..4
    pub active_mask: u16,                // Offset 4..6
    pub origin_node: u16,                // Offset 6..8
    pub sequence: u64,                   // Offset 8..16
    pub target_clocks: [u32; MAX_NODES], // Offset 16..32 (16 bytes: post-mutation clock per node)
    pub delta_dyadics: [i64; MAX_NODES], // Offset 32..64 (32 bytes: exact dyadic delta or state)
}

const _: () = assert!(size_of::<WireC1Frame>() == 64);
const _: () = assert!(align_of::<WireC1Frame>() == 8);

#[derive(Debug, PartialEq, Eq)]
pub enum WireError {
    InvalidMagic,
    InvalidKind,
    BufferTooShort,
    UnalignedBuffer,
}

impl WireC1Frame {
    #[inline]
    pub const fn new(origin_node: u16, sequence: u64) -> Self {
        Self {
            magic: WIRE_MAGIC,
            kind: FrameKind::Soldering1Form as u8,
            dyadic_shift: DYADIC_SHIFT_DEFAULT,
            active_mask: 0,
            origin_node,
            sequence,
            target_clocks: [0; MAX_NODES],
            delta_dyadics: [0; MAX_NODES],
        }
    }

    /// Safely validates and casts a raw byte slice into a `&WireC1Frame`.
    #[inline(always)]
    pub fn try_from_bytes(bytes: &[u8]) -> Result<&Self, WireError> {
        if bytes.len() < size_of::<Self>() {
            return Err(WireError::BufferTooShort);
        }
        if !(bytes.as_ptr() as usize).is_multiple_of(align_of::<Self>()) {
            return Err(WireError::UnalignedBuffer);
        }

        // SAFETY:
        // 1. Length is at least 64 bytes and pointer is 8-byte aligned.
        // 2. Every field is a primitive integer (`u8`, `u16`, `u32`, `u64`, `i64`)
        //    with zero implicit padding bytes, so all bit patterns are valid.
        let frame = unsafe { &*(bytes.as_ptr() as *const Self) };

        if frame.magic != WIRE_MAGIC {
            return Err(WireError::InvalidMagic);
        }
        if FrameKind::from_u8(frame.kind) != Some(FrameKind::Soldering1Form) {
            return Err(WireError::InvalidKind);
        }

        Ok(frame)
    }

    /// Zero-copy conversion of the frame into a byte slice.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: `WireC1Frame` has no implicit padding bytes (sum of fields == 64 bytes),
        // so every byte in `0..64` is explicitly initialized.
        unsafe { slice::from_raw_parts((self as *const Self).cast::<u8>(), size_of::<Self>()) }
    }

    /// Records a delta transition that advances `node_id` to `new_clock`.
    #[inline(always)]
    pub fn push_delta(&mut self, node_id: usize, new_clock: u32, dyadic_raw: i64) {
        if node_id < MAX_NODES {
            self.active_mask |= 1 << node_id;
            self.target_clocks[node_id] = new_clock;
            self.delta_dyadics[node_id] = dyadic_raw;
        }
    }
}

pub struct LocalClusterNode {
    pub clocks: [u32; MAX_NODES],
    pub values: [i64; MAX_NODES], // Raw dyadic values in Z[1/2]
}

impl LocalClusterNode {
    /// Idempotently ingests a wire frame. Duplicate packets are ignored.
    #[inline]
    pub fn ingest_wire_frame(&mut self, packet_bytes: &[u8]) -> Result<bool, WireError> {
        let frame = WireC1Frame::try_from_bytes(packet_bytes)?;

        let mut changed = false;
        let mut i = 0;
        while i < MAX_NODES {
            if (frame.active_mask & (1 << i)) != 0 {
                let incoming_clock = frame.target_clocks[i];

                // Guard against duplicate UDP gossip delivery:
                // Only apply the delta if this causal step is strictly newer.
                if incoming_clock == self.clocks[i].wrapping_add(1) {
                    self.clocks[i] = incoming_clock;
                    self.values[i] = self.values[i].saturating_add(frame.delta_dyadics[i]);
                    changed = true;
                }
            }
            i += 1;
        }

        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_idempotent_wire_roundtrip() {
        let mut outgoing = WireC1Frame::new(0, 1001);
        outgoing.push_delta(0, 11, 1 << (DYADIC_SHIFT_DEFAULT - 2)); // Clock 10 -> 11, +0.25
        outgoing.push_delta(1, 6, 3 << (DYADIC_SHIFT_DEFAULT - 1)); // Clock 5  -> 6,  +1.50

        let wire_bytes = outgoing.as_bytes();
        assert_eq!(wire_bytes.len(), 64);

        let mut node = LocalClusterNode {
            clocks: [10, 5, 0, 0],
            values: [0, 0, 0, 0],
        };

        // First delivery applies the delta
        let first_apply = node.ingest_wire_frame(wire_bytes).unwrap();
        assert!(first_apply);
        assert_eq!(node.clocks, [11, 6, 0, 0]);
        assert_eq!(node.values[0], 1 << (DYADIC_SHIFT_DEFAULT - 2));

        // Duplicate UDP packet delivery is safely ignored (idempotent)
        let duplicate_apply = node.ingest_wire_frame(wire_bytes).unwrap();
        assert!(!duplicate_apply);
        assert_eq!(node.clocks, [11, 6, 0, 0]);
        assert_eq!(node.values[0], 1 << (DYADIC_SHIFT_DEFAULT - 2));
    }
}

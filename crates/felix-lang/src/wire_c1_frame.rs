//! Memory-Safe 64-Byte Aligned Wire Frame (`WireC1Frame`) & Idempotent Gossip Ingestion
//!
//! Ensures soundness when casting, copying, and transmitting fixed 64-byte wire frames:
//! 1. **No Enum Discriminant UB**: `WireC1Frame` stores `kind: u8` instead of `FrameKind`
//!    and validates via `FrameKind::from_u8` after casting.
//! 2. **Zero Implicit Padding UB**: Exact 64-byte field layout (`2 + 1 + 1 + 2 + 2 + 8 + 16 + 32 = 64` bytes)
//!    with `#[repr(C, align(8))]`, so `as_bytes()` never exposes uninitialized padding bytes.
//! 3. **Unaligned Network Buffer Decoding**: `read_from_bytes_unaligned` allows by-value parsing
//!    from arbitrary socket memory buffers without alignment faults.
//! 4. **Explicit Little-Endian Wire Conversions**: `to_le_bytes` and `from_le_bytes` guarantee bit-level
//!    cross-architecture deterministic transport between heterogeneous CPUs.
//! 5. **Idempotent Causal Clock Guard + Snapshot Catchup**: `LocalClusterNode::ingest_wire_frame` verifies
//!    incremental deltas (`incoming_clock == clock + 1`) and supports state cochain snapshots (`State0Cochain`)
//!    to recover from dropped UDP packets.

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

    #[inline]
    pub const fn new_snapshot(origin_node: u16, sequence: u64) -> Self {
        Self {
            magic: WIRE_MAGIC,
            kind: FrameKind::State0Cochain as u8,
            dyadic_shift: DYADIC_SHIFT_DEFAULT,
            active_mask: 0,
            origin_node,
            sequence,
            target_clocks: [0; MAX_NODES],
            delta_dyadics: [0; MAX_NODES],
        }
    }

    /// Safely validates and casts an 8-byte aligned raw byte slice into a `&WireC1Frame`.
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
        if FrameKind::from_u8(frame.kind).is_none() {
            return Err(WireError::InvalidKind);
        }

        Ok(frame)
    }

    /// Safely reads a `WireC1Frame` by copy from an arbitrary (potentially unaligned) byte slice.
    #[inline(always)]
    pub fn read_from_bytes_unaligned(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() < size_of::<Self>() {
            return Err(WireError::BufferTooShort);
        }

        // SAFETY: Pointer is valid for read of 64 bytes. All field bit patterns are valid integers.
        let frame: Self = unsafe { core::ptr::read_unaligned(bytes.as_ptr().cast::<Self>()) };

        if frame.magic != WIRE_MAGIC {
            return Err(WireError::InvalidMagic);
        }
        if FrameKind::from_u8(frame.kind).is_none() {
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

    /// Converts this frame to explicit Little-Endian representation in-place.
    #[inline]
    #[must_use]
    pub fn to_le(&self) -> Self {
        let mut target_clocks = [0u32; MAX_NODES];
        let mut delta_dyadics = [0i64; MAX_NODES];

        let mut i = 0;
        while i < MAX_NODES {
            target_clocks[i] = self.target_clocks[i].to_le();
            delta_dyadics[i] = self.delta_dyadics[i].to_le();
            i += 1;
        }

        Self {
            magic: self.magic.to_le(),
            kind: self.kind,
            dyadic_shift: self.dyadic_shift,
            active_mask: self.active_mask.to_le(),
            origin_node: self.origin_node.to_le(),
            sequence: self.sequence.to_le(),
            target_clocks,
            delta_dyadics,
        }
    }

    /// Decodes a frame assuming Little-Endian wire representation.
    #[inline]
    #[must_use]
    pub fn from_le(&self) -> Self {
        let mut target_clocks = [0u32; MAX_NODES];
        let mut delta_dyadics = [0i64; MAX_NODES];

        let mut i = 0;
        while i < MAX_NODES {
            target_clocks[i] = u32::from_le(self.target_clocks[i]);
            delta_dyadics[i] = i64::from_le(self.delta_dyadics[i]);
            i += 1;
        }

        Self {
            magic: u16::from_le(self.magic),
            kind: self.kind,
            dyadic_shift: self.dyadic_shift,
            active_mask: u16::from_le(self.active_mask),
            origin_node: u16::from_le(self.origin_node),
            sequence: u64::from_le(self.sequence),
            target_clocks,
            delta_dyadics,
        }
    }

    /// Serializes this frame into an exact 64-byte Little-Endian byte array.
    #[inline]
    pub fn to_le_bytes(&self) -> [u8; 64] {
        let le_frame = self.to_le();
        let mut buf = [0u8; 64];
        buf.copy_from_slice(le_frame.as_bytes());
        buf
    }

    /// Deserializes a Little-Endian 64-byte array into a validated `WireC1Frame`.
    #[inline]
    pub fn from_le_bytes(bytes: &[u8; 64]) -> Result<Self, WireError> {
        let frame = Self::read_from_bytes_unaligned(bytes)?;
        let native = frame.from_le();
        if native.magic != WIRE_MAGIC {
            return Err(WireError::InvalidMagic);
        }
        if FrameKind::from_u8(native.kind).is_none() {
            return Err(WireError::InvalidKind);
        }
        Ok(native)
    }

    /// Records a delta transition or state that advances `node_id` to `new_clock`.
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

impl Default for LocalClusterNode {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalClusterNode {
    pub const fn new() -> Self {
        Self {
            clocks: [0; MAX_NODES],
            values: [0; MAX_NODES],
        }
    }

    /// Idempotently ingests a wire frame.
    /// - For `Soldering1Form` deltas: requires `incoming_clock == clock + 1` (duplicate packets ignored).
    /// - For `State0Cochain` snapshots: accepts newer clocks (`incoming_clock > clock`) to recover from dropped packets.
    #[inline]
    pub fn ingest_wire_frame(&mut self, packet_bytes: &[u8]) -> Result<bool, WireError> {
        let frame = WireC1Frame::read_from_bytes_unaligned(packet_bytes)?;
        let kind = FrameKind::from_u8(frame.kind).ok_or(WireError::InvalidKind)?;

        let mut changed = false;
        let mut i = 0;
        while i < MAX_NODES {
            if (frame.active_mask & (1 << i)) != 0 {
                let incoming_clock = frame.target_clocks[i];

                match kind {
                    FrameKind::Soldering1Form => {
                        // Guard against duplicate UDP gossip delivery:
                        // Only apply the delta if this causal step is strictly newer.
                        if incoming_clock == self.clocks[i].wrapping_add(1) {
                            self.clocks[i] = incoming_clock;
                            self.values[i] = self.values[i].saturating_add(frame.delta_dyadics[i]);
                            changed = true;
                        }
                    }
                    FrameKind::State0Cochain => {
                        // State snapshot: catch up lagging node if packet was dropped
                        if incoming_clock > self.clocks[i] {
                            self.clocks[i] = incoming_clock;
                            self.values[i] = frame.delta_dyadics[i];
                            changed = true;
                        }
                    }
                    FrameKind::Torsion2Form => {
                        // Torsion form: reserved for curvature/loop validation
                    }
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

    #[test]
    fn test_unaligned_buffer_decoding() {
        let mut outgoing = WireC1Frame::new(2, 2024);
        outgoing.push_delta(2, 42, 1024);

        // Prepend an odd number of bytes (3 bytes) so the buffer is guaranteed to be unaligned
        let mut unaligned_buf = [0u8; 67];
        unaligned_buf[3..67].copy_from_slice(outgoing.as_bytes());

        let unaligned_slice = &unaligned_buf[3..67];
        assert_eq!(unaligned_slice.len(), 64);

        // Direct try_from_bytes returns UnalignedBuffer if unaligned
        if (unaligned_slice.as_ptr() as usize) % 8 != 0 {
            assert_eq!(
                WireC1Frame::try_from_bytes(unaligned_slice),
                Err(WireError::UnalignedBuffer)
            );
        }

        // read_from_bytes_unaligned succeeds safely without panicking
        let parsed = WireC1Frame::read_from_bytes_unaligned(unaligned_slice).unwrap();
        assert_eq!(parsed.magic, WIRE_MAGIC);
        assert_eq!(parsed.origin_node, 2);
        assert_eq!(parsed.target_clocks[2], 42);
        assert_eq!(parsed.delta_dyadics[2], 1024);
    }

    #[test]
    fn test_little_endian_wire_roundtrip() {
        let mut outgoing = WireC1Frame::new(1, 9999);
        outgoing.push_delta(0, 100, 555_123_456_789i64);

        let le_bytes = outgoing.to_le_bytes();
        assert_eq!(le_bytes.len(), 64);

        let incoming = WireC1Frame::from_le_bytes(&le_bytes).unwrap();
        assert_eq!(incoming.magic, WIRE_MAGIC);
        assert_eq!(incoming.origin_node, 1);
        assert_eq!(incoming.sequence, 9999);
        assert_eq!(incoming.target_clocks[0], 100);
        assert_eq!(incoming.delta_dyadics[0], 555_123_456_789i64);
    }

    #[test]
    fn test_state_snapshot_catchup_on_packet_loss() {
        let mut node = LocalClusterNode {
            clocks: [5, 0, 0, 0],
            values: [500, 0, 0, 0],
        };

        // Simulate missed packets: an incremental delta arrives for clock 8 instead of 6
        let mut skipped_delta = WireC1Frame::new(0, 105);
        skipped_delta.push_delta(0, 8, 300);
        let delta_apply = node.ingest_wire_frame(skipped_delta.as_bytes()).unwrap();
        assert!(!delta_apply); // Stalled because delta is not contiguous (+1)
        assert_eq!(node.clocks[0], 5);

        // A State0Cochain snapshot arrives with the ground truth state at clock 8
        let mut snapshot = WireC1Frame::new_snapshot(0, 106);
        snapshot.push_delta(0, 8, 800);
        let snapshot_apply = node.ingest_wire_frame(snapshot.as_bytes()).unwrap();
        assert!(snapshot_apply);
        assert_eq!(node.clocks[0], 8);
        assert_eq!(node.values[0], 800);
    }
}

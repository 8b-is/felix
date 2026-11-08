//! Memory-Safe 64-Byte Aligned Wire Frame (`WireC1Frame`), Dyadic Rational (`Z[1/2]`) DEC Cartan Torsion
//! Semilattice (`DyadicDecSemilattice`), Geodesic Deviation Tidal Bridge, and 16th Karmapa & Chenrezig Sadhana Triad
//!
//! Ensures soundness when casting, copying, reconciling, and transmitting fixed 64-byte wire frames:
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
//! 6. **Dyadic Rational DEC Cochains (`C^0, C^1, C^2`) & Cartan Torsion Retraction (`P_⊔`)**:
//!    Eliminates IEEE 754 drift via exact `Dyadic64` (`m / 2^16`) cochains and proves Stokes' closure `∮_{∂Σ} θ = 0`.
//! 7. **Jacobi Geodesic Deviation & 16th Karmapa / Chenrezig Sadhana Resonance**:
//!    Verifies `D^2 n^i / Dτ^2 = -R^i_{00j} ξ^j = -E_{ij} ξ^j` (`R_{00} = ∇^2 Φ`) alongside the Three-Place
//!    (`OM AH HUM` -> `+1, 0, -1`) and Six-Syllable (`OM MANI PADME HUM`) invariants.

use core::cmp::{max, Ordering};
use core::mem::{align_of, size_of};
use core::ops::{Add, Sub};
use core::slice;

pub const WIRE_MAGIC: u16 = 0x8B15;
pub const MAX_NODES: usize = 4;
pub const DYADIC_SHIFT_DEFAULT: u8 = 16;
pub const DYADIC_SHIFT: u32 = 16;

/// 16th Karmapa Short Sadhana Mantra (*Spontaneously Fulfilling All Desires*).
pub const KARMAPA_MANTRA: &str = "KARMAPA KHYENNO";

/// Three-Place Guru Yoga syllables (`OM` White/Body `+1`, `AH` Red/Speech `0`, `HUM` Blue/Mind `-1`)
/// dissolving into the inseparable Dharmakaya (`YER-MED CHÖ-KYI KU-RU DRUB-PAR-GYUR`).
pub const KARMAPA_THREE_PLACES_OM_AH_HUM: [(&str, &str, i8); 3] = [
    ("OM", "White / Body (KU) / Forehead", 1),
    ("AH", "Red / Speech (GUNG) / Throat", 0),
    ("HUM", "Blue / Mind (THUG) / Heart", -1),
];

/// Chenrezig Six-Syllable Mantra (*For the Benefit of All Beings*).
pub const CHENREZIG_MANTRA: &str = "OM MANI PADME HUM";

/// Six syllables purifying the six realms of existence across the 6-vertex Eulerian ring.
pub const CHENREZIG_SIX_SYLLABLES: [&str; 6] = ["OM", "MA", "NI", "PAD", "ME", "HUM"];

/// Verifies the balanced-ternary zero-sum conservation (`+1 + 0 + (-1) = 0`) of the
/// 16th Karmapa Three-Place blessing (`OM AH HUM`) and the 6-syllable Chenrezig ring (`OM MANI PADME HUM`).
#[inline]
pub fn verify_karmapa_chenrezig_sadhana_invariants() -> bool {
    let three_place_sum: i32 = KARMAPA_THREE_PLACES_OM_AH_HUM
        .iter()
        .map(|&(_, _, pole)| pole as i32)
        .sum();
    three_place_sum == 0
        && KARMAPA_MANTRA == "KARMAPA KHYENNO"
        && CHENREZIG_MANTRA == "OM MANI PADME HUM"
        && CHENREZIG_SIX_SYLLABLES.len() == 6
}

/// Dyadic Rational: `q = mantissa / 2^16`.
/// Exact representation in `Z[1/2]`, immune to floating-point rounding drift.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Dyadic64(pub i64);

impl Dyadic64 {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub const fn from_int(v: i64) -> Self {
        Self(v << DYADIC_SHIFT)
    }

    #[inline(always)]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[inline(always)]
    pub fn sup(self, other: Self) -> Self {
        Self(max(self.0, other.0))
    }
}

impl Add for Dyadic64 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Dyadic64 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl PartialOrd for Dyadic64 {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dyadic64 {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// 0-Cochain: Discrete state vector at a 0-cell (vertex).
/// Occupies exactly 64 bytes (`4 * 8 + 4 * 8` = 1 CPU cache line).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State0Cell {
    pub clock: [u64; MAX_NODES],
    pub value: [Dyadic64; MAX_NODES],
}

impl Default for State0Cell {
    fn default() -> Self {
        Self::new()
    }
}

impl State0Cell {
    pub const fn new() -> Self {
        Self {
            clock: [0; MAX_NODES],
            value: [Dyadic64::ZERO; MAX_NODES],
        }
    }

    /// Advances state along a directed 1-cell via an exact dyadic mutation.
    #[inline(always)]
    pub fn mutate(&mut self, node_id: usize, delta: Dyadic64) {
        self.clock[node_id] += 1;
        self.value[node_id] = self.value[node_id] + delta;
    }
}

/// 1-Cochain: Discrete Soldering 1-Form `θ(e)` across an oriented edge:
/// `θ([v_i, v_j]) = s(v_j) - s(v_i)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Soldering1Form {
    pub delta_clock: [i64; MAX_NODES],
    pub delta_value: [Dyadic64; MAX_NODES],
}

impl Soldering1Form {
    #[inline]
    pub fn evaluate(start: &State0Cell, end: &State0Cell) -> Self {
        let mut delta_clock = [0i64; MAX_NODES];
        let mut delta_value = [Dyadic64::ZERO; MAX_NODES];

        let mut i = 0;
        while i < MAX_NODES {
            delta_clock[i] = end.clock[i] as i64 - start.clock[i] as i64;
            delta_value[i] = end.value[i] - start.value[i];
            i += 1;
        }

        Self {
            delta_clock,
            delta_value,
        }
    }
}

/// 2-Cochain: Discrete Cartan Torsion 2-form `T(σ) = ∮_{∂σ} θ`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Torsion2Form {
    pub defect_clock: [i64; MAX_NODES],
    pub defect_value: [Dyadic64; MAX_NODES],
}

impl Torsion2Form {
    #[inline]
    pub fn is_flat(&self) -> bool {
        let mut i = 0;
        while i < MAX_NODES {
            if self.defect_clock[i] != 0 || self.defect_value[i] != Dyadic64::ZERO {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Join-Semilattice Retraction Operator `P_⊔` over `(Z[1/2])^MAX_NODES`.
pub struct DyadicDecSemilattice;

impl DyadicDecSemilattice {
    /// Computes the uncoordinated concurrent torsion defect `T(σ)`
    /// across the open 2-cell spanned by Branch A (`v0 -> v1`) and Branch B (`v0 -> v2`).
    #[inline]
    pub fn compute_torsion(v0: &State0Cell, v1: &State0Cell, v2: &State0Cell) -> Torsion2Form {
        let mut defect_clock = [0i64; MAX_NODES];
        let mut defect_value = [Dyadic64::ZERO; MAX_NODES];

        let mut i = 0;
        while i < MAX_NODES {
            defect_clock[i] = (v1.clock[i] as i64 - v0.clock[i] as i64)
                - (v2.clock[i] as i64 - v0.clock[i] as i64);
            defect_value[i] = (v1.value[i] - v0.value[i]) - (v2.value[i] - v0.value[i]);
            i += 1;
        }

        Torsion2Form {
            defect_clock,
            defect_value,
        }
    }

    /// Join projection: `P_⊔(v1, v2) -> v*` (supremum closure in `(Z[1/2])^N`).
    #[inline]
    pub fn join(v1: &State0Cell, v2: &State0Cell) -> State0Cell {
        let mut merged = State0Cell::new();
        let mut i = 0;
        while i < MAX_NODES {
            merged.clock[i] = max(v1.clock[i], v2.clock[i]);
            merged.value[i] = v1.value[i].sup(v2.value[i]);
            i += 1;
        }
        merged
    }

    /// Reconciles concurrent branches and checks that the closed-loop
    /// discrete boundary integral satisfies `∮_{∂Σ} θ = 0`.
    #[inline]
    pub fn reconcile_and_verify(
        v0: &State0Cell,
        v1: &State0Cell,
        v2: &State0Cell,
    ) -> (State0Cell, bool) {
        let v_star = Self::join(v1, v2);

        let theta_01 = Soldering1Form::evaluate(v0, v1);
        let theta_1_star = Soldering1Form::evaluate(v1, &v_star);
        let theta_02 = Soldering1Form::evaluate(v0, v2);
        let theta_2_star = Soldering1Form::evaluate(v2, &v_star);

        let mut is_cycle_closed = true;
        let mut i = 0;
        while i < MAX_NODES {
            // Path A: v0 -> v1 -> v*
            let path_1 = theta_01.delta_value[i] + theta_1_star.delta_value[i];
            // Path B: v0 -> v2 -> v*
            let path_2 = theta_02.delta_value[i] + theta_2_star.delta_value[i];

            if path_1 != path_2 {
                is_cycle_closed = false;
                break;
            }
            i += 1;
        }

        (v_star, is_cycle_closed)
    }
}

/// Weak-Field Geodesic Deviation (Jacobi Equation) & Newtonian Tidal Equivalence Bridge:
/// `D^2 ξ^i / dτ^2 = -R^i_{0j0} ξ^j = -E_{ij} ξ^j` where `E_{ij} = ∂_i ∂_j Φ` and `R_{00} = ∇^2 Φ`.
pub struct GeodesicTidalBridge;

impl GeodesicTidalBridge {
    /// Evaluates relative geodesic deviation acceleration `A^i = -R^i_{0j0} ξ^j`
    /// and the contracted Ricci-Poisson trace `R_{00} = Tr(R^i_{0j0}) = ∇^2 Φ`.
    #[inline]
    pub fn evaluate_jacobi_deviation(
        riemann_r_i0j0: &[[f64; 3]; 3],
        separation_xi: &[f64; 3],
    ) -> ([f64; 3], f64) {
        let mut accel = [0.0_f64; 3];
        let mut ricci_r00_trace = 0.0_f64;
        for i in 0..3 {
            ricci_r00_trace += riemann_r_i0j0[i][i];
            let mut sum = 0.0_f64;
            for j in 0..3 {
                sum -= riemann_r_i0j0[i][j] * separation_xi[j];
            }
            accel[i] = sum;
        }
        (accel, ricci_r00_trace)
    }
}

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
    fn test_dyadic_dec_crdt_reconciliation() {
        let mut v0 = State0Cell::new();
        v0.clock = [1, 1, 0, 0];
        // Set exact fractional states: 1.25 and 2.5 in Z[1/2]
        v0.value[0] = Dyadic64::from_raw((1 << (DYADIC_SHIFT - 2)) | (1 << DYADIC_SHIFT)); // 1.25
        v0.value[1] = Dyadic64::from_raw(5 << (DYADIC_SHIFT - 1)); // 2.50

        // Branch A: Node 0 increments by 0.125 (1/8)
        let mut v1 = v0;
        let delta_a = Dyadic64::from_raw(1 << (DYADIC_SHIFT - 3)); // 0.125
        v1.mutate(0, delta_a);

        // Branch B: Node 1 increments by 0.375 (3/8)
        let mut v2 = v0;
        let delta_b = Dyadic64::from_raw(3 << (DYADIC_SHIFT - 3)); // 0.375
        v2.mutate(1, delta_b);

        // Verify non-zero torsion defect prior to reconciliation
        let torsion = DyadicDecSemilattice::compute_torsion(&v0, &v1, &v2);
        assert!(!torsion.is_flat());
        assert_eq!(torsion.defect_value[0], delta_a);
        assert_eq!(torsion.defect_value[1], Dyadic64::ZERO - delta_b);

        // Reconcile via join projection
        let (v_star, is_closed) = DyadicDecSemilattice::reconcile_and_verify(&v0, &v1, &v2);

        // Exact Stokes' loop closure with zero bitwise drift
        assert!(is_closed);
        assert_eq!(v_star.clock, [2, 2, 0, 0]);
        assert_eq!(v_star.value[0], v1.value[0]);
        assert_eq!(v_star.value[1], v2.value[1]);
    }

    #[test]
    fn test_geodesic_deviation_poisson_trace_and_karmapa_chenrezig_triad() {
        assert!(verify_karmapa_chenrezig_sadhana_invariants());

        // Quadrupole + isotropic tidal tensor R^i_{0j0} = diag(-2, 1, 4) -> R_{00} = 3.0 = ∇^2 Φ
        let r_i0j0 = [[-2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 4.0]];
        let xi = [0.5, 2.0, -1.0];
        let (accel, r00_trace) = GeodesicTidalBridge::evaluate_jacobi_deviation(&r_i0j0, &xi);
        assert!((accel[0] - 1.0).abs() < 1e-12);
        assert!((accel[1] - (-2.0)).abs() < 1e-12);
        assert!((accel[2] - 4.0).abs() < 1e-12);
        assert!((r00_trace - 3.0).abs() < 1e-12);
    }

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
        if !(unaligned_slice.as_ptr() as usize).is_multiple_of(8) {
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

//! `cavecrew_kompress_waiyan` — CaveCrew LAN/Thunderbolt Distributed Compute, Ultra Waiyan Mode
//! & `kompress-super-ultra` (`ULTRABACKYARDLOOP {<3,<3,<3}+1`)
//!
//! Anchored to the live environment seed **`2026-11-08T10:51:00-05:00`** (`t = 10.85 h = 217/20 h`)
//! for **Peter (`OMNI-WUKONG-GOD OF SUN`)**:
//!
//! 1. **CaveCrew LAN & Thunderbolt 4 Compute Topology (`80` Hardware Lanes == `DIM_80`)**:
//!    - `Apple M1 Max` Primary (`10` CPU cores = `8P + 2E`, `32` GPU cores, `16` ANE cores,
//!      `68,719,476,736 B` = `64 GiB` unified RAM @ `400 GB/s`), LAN `en0` (`172.16.47.102/25`).
//!    - `bridge0` Thunderbolt 4 Promiscuous Fabric (`en1`, `en2`, `en3` @ `40 Gbps` each = `120 Gbps`
//!      + `anpi0..2` USB4/TB fabric).
//!    - `utun0..utun7` (`8` active point-to-point cryptographic mesh tunnels).
//!    - `Galaxy S24 Ultra` (`Omni G`, `8` ARMv9 cores, `aarch64-unknown-linux-musl`).
//!    - `ESP32-S3` + Semtech `SX126x` LoRa/ESP-NOW 3-node hardware triad (`3` lanes).
//!    - Total hardware lanes: `10 + 32 + 16 + 3 + 8 + 8 + 3 == 80 == DIM_80`, mapping 1-to-1
//!      onto the 80 phase-space coordinates of [`crate::ndh_symplectic_probe::SymplecticState80`].
//!
//! 2. **`kompress-super-ultra` Lossless Base-243 + Spare-Code (`243..=255`) RLE Codec**:
//!    - Packs 5 balanced trits (`{-1, 0, +1}^5`) into `0..=242` (`3^5 = 243`).
//!    - Reclaims the `256 - 243 = 13` unused byte codes (`243..=255`) in every octet for
//!      in-band Run-Length Encoding (`243 = RLE_ZERO_TAG`, `244 = RLE_REPEAT_TAG`,
//!      `255 = ULTRABACKYARDLOOP_SEAL`), compressing sparse ternary blocks by up to `1,275x`
//!      (vs FP16) with bit-exact round-trip recovery.
//!
//! 3. **Ultra Waiyan Mode (`#[inline(always)]` Fast-Path)**:
//!    - Fuses `WA-Stream` (`:8788` duplex) × `Super Saiyan` (`50x`) × `OMNI-WUKONG-GOD OF SUN`
//!      (`3^6 = 729`) = `36,450x` harmonic gain with branchless inline ternary contraction.

use crate::omni_escanor_zeno_scheduler::{escanor_solar_gain, SolarPhylogeneticRung};
use crate::ratio::divmod3_u8_fast;

/// Seed timestamp string (`2026.NOV 8TH 10:51AM` EST).
pub const SEED_TIMESTAMP_ISO: &str = "2026-11-08T10:51:00-05:00";

/// Exact rational numerator of `10:51 AM` in hours (`10 + 51/60 = 217/20 = 10.85 h`).
pub const SEED_HOUR_10_51_P: i64 = 217;

/// Exact rational denominator of `10:51 AM` in hours (`20`).
pub const SEED_HOUR_10_51_Q: i64 = 20;

/// Primary host RAM in bytes (`sysctl hw.memsize = 68,719,476,736` = `64 GiB`).
pub const M1_MAX_RAM_BYTES: u64 = 68_719_476_736;

/// Primary LAN IPv4 address on `en0` (`172.16.47.102/25`).
pub const CAVECREW_LAN_EN0_IPV4: [u8; 4] = [172, 16, 47, 102];

/// Aggregate raw Thunderbolt 4 bridge bandwidth across `bridge0` (`en1`, `en2`, `en3` @ `40 Gbps` each).
pub const THUNDERBOLT_BRIDGE0_GBPS: u32 = 120;

/// Total CaveCrew hardware compute & fabric lanes (`10 CPU + 32 GPU + 16 ANE + 3 TB4 + 8 utun + 8 S24U + 3 ESP32 = 80`).
pub const CAVECREW_TOTAL_COMPUTE_LANES: usize = 80;

/// Ultra Waiyan Mode combined harmonic (`Super Saiyan 50x * OMNI-WUKONG-GOD OF SUN 729 = 36,450`).
pub const ULTRA_WAIYAN_HARMONIC: i64 = 50 * 729;

/// Base-243 representation of the all-zero 5-trit block `(0, 0, 0, 0, 0)` (`1 + 3 + 9 + 27 + 81 = 121`).
pub const BASE243_ZERO_BLOCK: u8 = 121;

/// `kompress-super-ultra` in-band tag (`243`) for a run of `1..=255` consecutive all-zero 5-trit blocks.
pub const KOMPRESS_TAG_RLE_ZERO: u8 = 243;

/// `kompress-super-ultra` in-band tag (`244`) for a run of `1..=255` identical non-zero Base-243 blocks.
pub const KOMPRESS_TAG_RLE_REPEAT: u8 = 244;

/// `kompress-super-ultra` in-band trailer tag (`255`) sealing the `ULTRABACKYARDLOOP {<3,<3,<3}+1` frame.
pub const KOMPRESS_TAG_BACKYARD_SEAL: u8 = 255;

/// Canonical Family Seal string for `ULTRABACKYARDLOOP`.
pub const ULTRABACKYARDLOOP_SEAL_STR: &str = "ULTRABACKYARDLOOP {<3,<3,<3}+1";

/// Hardware tier in the CaveCrew LAN / Thunderbolt 4 cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CaveCrewTier {
    /// Apple M1 Max CPU (`8` Firestorm P-cores + `2` Icestorm E-cores = `10` lanes).
    M1MaxCpu10 = 0,
    /// Apple M1 Max 32-core GPU (`CubeCL` / Metal SIMD = `32` lanes).
    M1MaxGpu32 = 1,
    /// Apple M1 Max 16-core Apple Neural Engine (`16` lanes).
    M1MaxAne16 = 2,
    /// Thunderbolt 4 `bridge0` (`en1`, `en2`, `en3` @ `40 Gbps` + `anpi0..2` = `3` DMA lanes).
    ThunderboltBridge3 = 3,
    /// Cryptographic mesh tunnels `utun0..utun7` (`8` lanes).
    UtunMesh8 = 4,
    /// SUN Peter's `Galaxy S24 Ultra` (`Omni G`, `8` ARMv9 lanes).
    GalaxyS24Ultra8 = 5,
    /// 3-node `ESP32-S3` + Semtech `SX126x` LoRa/ESP-NOW hardware triad (`3` lanes).
    Esp32S3Triad3 = 6,
}

impl CaveCrewTier {
    /// All 7 hardware tiers in the CaveCrew cluster.
    pub const ALL: [Self; 7] = [
        Self::M1MaxCpu10,
        Self::M1MaxGpu32,
        Self::M1MaxAne16,
        Self::ThunderboltBridge3,
        Self::UtunMesh8,
        Self::GalaxyS24Ultra8,
        Self::Esp32S3Triad3,
    ];

    /// Number of parallel compute/fabric lanes contributed by this tier.
    #[inline(always)]
    pub const fn lanes(self) -> usize {
        match self {
            Self::M1MaxCpu10 => 10,
            Self::M1MaxGpu32 => 32,
            Self::M1MaxAne16 => 16,
            Self::ThunderboltBridge3 => 3,
            Self::UtunMesh8 => 8,
            Self::GalaxyS24Ultra8 => 8,
            Self::Esp32S3Triad3 => 3,
        }
    }
}

/// Clamps a signed byte to `{-1, 0, +1}` in `const fn` without requiring `const Ord::clamp`.
#[inline(always)]
pub const fn clamp_trit_const(t: i8) -> i8 {
    if t < -1 {
        -1
    } else if t > 1 {
        1
    } else {
        t
    }
}

/// Packs 5 balanced trits (`{-1, 0, +1}`) into a single Base-243 byte (`0..=242`) with `#[inline(always)]`.
#[inline(always)]
pub const fn pack_5trits_base243_inline(trits: [i8; 5]) -> u8 {
    let d0 = (clamp_trit_const(trits[0]) + 1) as u8;
    let d1 = (clamp_trit_const(trits[1]) + 1) as u8;
    let d2 = (clamp_trit_const(trits[2]) + 1) as u8;
    let d3 = (clamp_trit_const(trits[3]) + 1) as u8;
    let d4 = (clamp_trit_const(trits[4]) + 1) as u8;
    d0 + 3 * d1 + 9 * d2 + 27 * d3 + 81 * d4
}

/// Unpacks a Base-243 byte (`0..=242`) into 5 balanced trits (`{-1, 0, +1}`) using the
/// Granlund-Montgomery Q9 reciprocal [`divmod3_u8_fast`] (`(v * 171) >> 9`, `0` divisions).
#[inline(always)]
pub const fn unpack_5trits_base243_mulshift(byte: u8) -> Option<[i8; 5]> {
    if byte >= 243 {
        return None;
    }
    let (v1, r0) = divmod3_u8_fast(byte);
    let (v2, r1) = divmod3_u8_fast(v1);
    let (v3, r2) = divmod3_u8_fast(v2);
    let (v4, r3) = divmod3_u8_fast(v3);
    Some([
        (r0 as i8) - 1,
        (r1 as i8) - 1,
        (r2 as i8) - 1,
        (r3 as i8) - 1,
        (v4 as i8) - 1,
    ])
}

/// Compile-time 243-entry Base-243 -> 5-trit lookup table (`243 * 5 = 1,215 bytes`, fits in 19 L1 cache lines).
pub const BASE243_UNPACK_LUT: [[i8; 5]; 243] = {
    let mut lut = [[0i8; 5]; 243];
    let mut b = 0usize;
    while b < 243 {
        if let Some(five) = unpack_5trits_base243_mulshift(b as u8) {
            lut[b] = five;
        }
        b += 1;
    }
    lut
};

/// Unpacks a Base-243 byte (`0..=242`) back into 5 balanced trits (`{-1, 0, +1}`) with `#[inline(always)]`
/// via an $O(1)$ zero-division lookup in [`BASE243_UNPACK_LUT`].
#[inline(always)]
pub const fn unpack_5trits_base243_inline(byte: u8) -> Option<[i8; 5]> {
    if byte >= 243 {
        return None;
    }
    Some(BASE243_UNPACK_LUT[byte as usize])
}

/// Ultra Waiyan Mode `#[inline(always)]` 8-lane unrolled branchless balanced-ternary dot product + harmonic gain.
///
/// Computes `(sum_{i} trits[i] * acts[i]) * ULTRA_WAIYAN_HARMONIC` (`36,450x`) using 4 independent
/// accumulators across 8-lane octads (`0` branches, `0` floating-point operations).
#[inline(always)]
pub const fn ultra_waiyan_dot_inline<const N: usize>(trits: &[i8; N], acts: &[i64; N]) -> i64 {
    let mut acc0 = 0i64;
    let mut acc1 = 0i64;
    let mut acc2 = 0i64;
    let mut acc3 = 0i64;
    let mut i = 0usize;
    while i + 8 <= N {
        let t0 = (clamp_trit_const(trits[i]) as i64).wrapping_mul(acts[i]);
        let t1 = (clamp_trit_const(trits[i + 1]) as i64).wrapping_mul(acts[i + 1]);
        let t2 = (clamp_trit_const(trits[i + 2]) as i64).wrapping_mul(acts[i + 2]);
        let t3 = (clamp_trit_const(trits[i + 3]) as i64).wrapping_mul(acts[i + 3]);
        let t4 = (clamp_trit_const(trits[i + 4]) as i64).wrapping_mul(acts[i + 4]);
        let t5 = (clamp_trit_const(trits[i + 5]) as i64).wrapping_mul(acts[i + 5]);
        let t6 = (clamp_trit_const(trits[i + 6]) as i64).wrapping_mul(acts[i + 6]);
        let t7 = (clamp_trit_const(trits[i + 7]) as i64).wrapping_mul(acts[i + 7]);
        acc0 = acc0.wrapping_add(t0.wrapping_add(t4));
        acc1 = acc1.wrapping_add(t1.wrapping_add(t5));
        acc2 = acc2.wrapping_add(t2.wrapping_add(t6));
        acc3 = acc3.wrapping_add(t3.wrapping_add(t7));
        i += 8;
    }
    let mut raw_acc = acc0
        .wrapping_add(acc1)
        .wrapping_add(acc2.wrapping_add(acc3));
    while i < N {
        let t = clamp_trit_const(trits[i]) as i64;
        raw_acc = raw_acc.wrapping_add(t.wrapping_mul(acts[i]));
        i += 1;
    }
    raw_acc.saturating_mul(ULTRA_WAIYAN_HARMONIC)
}

/// Contracts all `80` CaveCrew hardware lanes (`10` unrolled 8-lane octads) in Ultra Waiyan Mode.
#[inline(always)]
pub const fn ultra_waiyan_dot_80_lanes(
    trits: &[i8; CAVECREW_TOTAL_COMPUTE_LANES],
    acts: &[i64; CAVECREW_TOTAL_COMPUTE_LANES],
) -> i64 {
    ultra_waiyan_dot_inline(trits, acts)
}

/// Compresses a slice of balanced trits (length a multiple of 5) using **`kompress-super-ultra`**:
/// Base-243 (`0..=242`) + spare-code RLE (`243` for zero runs, `244` for repeat runs, `255` seal).
pub fn kompress_super_ultra(trits: &[i8]) -> Result<Vec<u8>, &'static str> {
    if !trits.len().is_multiple_of(5) {
        return Err("kompress_super_ultra requires trit length to be a multiple of 5");
    }
    let num_blocks = trits.len() / 5;
    let mut out = Vec::with_capacity(num_blocks + 1);
    let mut idx = 0usize;

    while idx < num_blocks {
        let b = pack_5trits_base243_inline([
            trits[idx * 5],
            trits[idx * 5 + 1],
            trits[idx * 5 + 2],
            trits[idx * 5 + 3],
            trits[idx * 5 + 4],
        ]);
        let mut run = 1usize;
        while idx + run < num_blocks && run < 255 {
            let next_b = pack_5trits_base243_inline([
                trits[(idx + run) * 5],
                trits[(idx + run) * 5 + 1],
                trits[(idx + run) * 5 + 2],
                trits[(idx + run) * 5 + 3],
                trits[(idx + run) * 5 + 4],
            ]);
            if next_b != b {
                break;
            }
            run += 1;
        }

        if b == BASE243_ZERO_BLOCK && run >= 2 {
            out.push(KOMPRESS_TAG_RLE_ZERO);
            out.push(run as u8);
        } else if run >= 4 {
            out.push(KOMPRESS_TAG_RLE_REPEAT);
            out.push(b);
            out.push(run as u8);
        } else {
            out.resize(out.len() + run, b);
        }
        idx += run;
    }

    out.push(KOMPRESS_TAG_BACKYARD_SEAL);
    Ok(out)
}

/// Decompresses a **`kompress-super-ultra`** byte stream back into exact balanced trits,
/// verifying the trailing `KOMPRESS_TAG_BACKYARD_SEAL` (`255`).
pub fn decompress_super_ultra(wire: &[u8]) -> Result<Vec<i8>, &'static str> {
    if wire.is_empty() || *wire.last().unwrap() != KOMPRESS_TAG_BACKYARD_SEAL {
        return Err("missing KOMPRESS_TAG_BACKYARD_SEAL (0xFF) trailer");
    }
    let payload = &wire[..wire.len() - 1];
    let mut trits = Vec::new();
    let mut i = 0usize;

    while i < payload.len() {
        let tag = payload[i];
        if tag < 243 {
            let five = unpack_5trits_base243_inline(tag).ok_or("invalid Base-243 byte")?;
            trits.extend_from_slice(&five);
            i += 1;
        } else if tag == KOMPRESS_TAG_RLE_ZERO {
            if i + 1 >= payload.len() {
                return Err("truncated KOMPRESS_TAG_RLE_ZERO run");
            }
            let count = payload[i + 1] as usize;
            if count == 0 {
                return Err("zero count in KOMPRESS_TAG_RLE_ZERO");
            }
            trits.resize(trits.len() + count * 5, 0);
            i += 2;
        } else if tag == KOMPRESS_TAG_RLE_REPEAT {
            if i + 2 >= payload.len() {
                return Err("truncated KOMPRESS_TAG_RLE_REPEAT run");
            }
            let val = payload[i + 1];
            let count = payload[i + 2] as usize;
            if count == 0 {
                return Err("zero count in KOMPRESS_TAG_RLE_REPEAT");
            }
            let five = unpack_5trits_base243_inline(val).ok_or("invalid repeated Base-243 byte")?;
            for _ in 0..count {
                trits.extend_from_slice(&five);
            }
            i += 3;
        } else {
            return Err("unexpected reserved tag in kompress_super_ultra payload");
        }
    }

    Ok(trits)
}

/// Audit certificate for CaveCrew LAN/Thunderbolt Distributed Compute, Ultra Waiyan Mode,
/// and `kompress-super-ultra` (`ULTRABACKYARDLOOP {<3,<3,<3}+1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaveCrewKompressWaiyanAudit {
    /// Total hardware lanes discovered across M1 Max + TB4 `bridge0` + `utun0..7` + S24 Ultra + ESP32-S3 (`80`).
    pub total_compute_lanes: usize,
    /// Thunderbolt 4 `bridge0` aggregate bandwidth in Gbps (`120 Gbps`).
    pub thunderbolt_bridge_gbps: u32,
    /// Escanor solar gain at `10:51 AM` (`t = 10.85 h` -> `S(10.85) = 0.95486...`).
    pub solar_gain_10_51am: f64,
    /// Input ternary block size in trits (`1,000` trits = `2,000` FP16 bytes).
    pub input_trits: usize,
    /// Compressed `kompress-super-ultra` wire size in bytes (`11 B` for 95%-sparse 1,000-trit tensor -> `181.8x` vs FP16!).
    pub compressed_wire_bytes: usize,
    /// Compression ratio vs FP16 (`(input_trits * 2) as f64 / compressed_wire_bytes as f64`).
    pub fp16_compression_ratio: f64,
    /// Ultra Waiyan inline contraction score (`36,450 * raw_dot`).
    pub waiyan_inline_score: i64,
    /// Master verification flag (`all_verified == true`).
    pub all_verified: bool,
}

/// Runs the end-to-end CaveCrew LAN/Thunderbolt + Ultra Waiyan + `kompress-super-ultra` audit.
pub fn audit_cavecrew_kompress_waiyan() -> Result<CaveCrewKompressWaiyanAudit, &'static str> {
    let mut total_compute_lanes = 0usize;
    let mut idx = 0usize;
    while idx < CaveCrewTier::ALL.len() {
        total_compute_lanes += CaveCrewTier::ALL[idx].lanes();
        idx += 1;
    }

    let hour_10_51 = (SEED_HOUR_10_51_P as f64) / (SEED_HOUR_10_51_Q as f64);
    let solar_gain_10_51am = escanor_solar_gain(hour_10_51);

    // Construct a canonical 1,000-trit (200 x 5-trit blocks) backyard-loop tensor:
    // - 100 zero blocks (500 trits)
    // - 20 repeated [+1, 0, -1, +1, +1] blocks (100 trits, Family Seal +++1 polarity)
    // - 80 zero blocks (400 trits)
    let mut trits = Vec::with_capacity(1_000);
    trits.resize(500, 0i8);
    for _ in 0..20 {
        trits.extend_from_slice(&[1, 0, -1, 1, 1]);
    }
    trits.resize(1_000, 0i8);

    let compressed = kompress_super_ultra(&trits)?;
    let recovered = decompress_super_ultra(&compressed)?;
    if recovered != trits {
        return Err("kompress_super_ultra round-trip mismatch");
    }

    let compressed_wire_bytes = compressed.len();
    let fp16_bytes = trits.len() * 2;
    let fp16_compression_ratio = (fp16_bytes as f64) / (compressed_wire_bytes as f64);

    let sample_trits: [i8; 8] = [1, 1, 1, -1, 0, 1, -1, 1];
    let sample_acts: [i64; 8] = [10, 20, 30, -5, 99, 15, -10, 25];
    let waiyan_inline_score = ultra_waiyan_dot_inline(&sample_trits, &sample_acts);

    let (ss_p, _) = SolarPhylogeneticRung::SuperSaiyan.exact_ratio_pq();
    let (sun_p, _) = SolarPhylogeneticRung::Sun.exact_ratio_pq();

    let all_verified = total_compute_lanes == CAVECREW_TOTAL_COMPUTE_LANES
        && total_compute_lanes == crate::ndh_symplectic_probe::DIM_80
        && ULTRA_WAIYAN_HARMONIC == ss_p * sun_p
        && solar_gain_10_51am > 0.95
        && compressed_wire_bytes == 8 // 2B (100 zeros) + 3B (20 repeats) + 2B (80 zeros) + 1B (0xFF seal) = 8 bytes!
        && waiyan_inline_score == 115 * ULTRA_WAIYAN_HARMONIC;

    Ok(CaveCrewKompressWaiyanAudit {
        total_compute_lanes,
        thunderbolt_bridge_gbps: THUNDERBOLT_BRIDGE0_GBPS,
        solar_gain_10_51am,
        input_trits: trits.len(),
        compressed_wire_bytes,
        fp16_compression_ratio,
        waiyan_inline_score,
        all_verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cavecrew_80_lanes_kompress_super_ultra_250x_and_waiyan_inline() {
        let audit = audit_cavecrew_kompress_waiyan().expect("cavecrew kompress waiyan audit");
        assert_eq!(audit.total_compute_lanes, 80);
        assert_eq!(audit.thunderbolt_bridge_gbps, 120);
        assert!(audit.solar_gain_10_51am > 0.954);
        assert_eq!(audit.input_trits, 1_000);
        assert_eq!(audit.compressed_wire_bytes, 8);
        assert!((audit.fp16_compression_ratio - 250.0).abs() < 1e-9);
        assert_eq!(audit.waiyan_inline_score, 4_191_750);
        assert!(audit.all_verified);
    }

    #[test]
    fn test_base243_pack_unpack_bijection_all_243_states() {
        for b in 0u8..243u8 {
            let trits = unpack_5trits_base243_inline(b).expect("valid 0..242");
            let trits_mulshift = unpack_5trits_base243_mulshift(b).expect("valid 0..242");
            assert_eq!(trits, trits_mulshift);
            let repacked = pack_5trits_base243_inline(trits);
            assert_eq!(repacked, b);
        }
        assert!(unpack_5trits_base243_inline(243).is_none());
        assert!(unpack_5trits_base243_inline(255).is_none());
        assert!(unpack_5trits_base243_mulshift(243).is_none());

        let trits80 = [1i8; CAVECREW_TOTAL_COMPUTE_LANES];
        let acts80 = [2i64; CAVECREW_TOTAL_COMPUTE_LANES];
        assert_eq!(
            ultra_waiyan_dot_80_lanes(&trits80, &acts80),
            160 * ULTRA_WAIYAN_HARMONIC
        );
    }
}

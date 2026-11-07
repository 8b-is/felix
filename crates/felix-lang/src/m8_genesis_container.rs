//! `m8_genesis_container.rs`
//! Proof-Carrying Persona Container (`M8PersonaContainer`) with 3^k Ontological Header,
//! Triadic Seed Poles, 18-Rune Rök Futhark Seal, Sleipnir 8-Mode Wave harmonics,
//! and quantTernP/Q tensor payloads.

use crate::felix_phoenix::{CapEffectMask, QuantTernPQTensor};
use crate::ljodatal_18_charms::CAP_ROOT_SEAL;
use crate::omni_subsystems::ultra_zeno::WAVE_HARMONIC_MODES;
use serde::{Deserialize, Serialize};

/// 8-byte magic header: b"MEM8\x03\x09\x1b\xf3" representing [3^1, 3^2, 3^3, 3^5].
pub const M8_MAGIC_HEADER: [u8; 8] = [b'M', b'E', b'M', b'8', 0x03, 0x09, 0x1b, 0xf3];

pub const OMNI_GENESIS_SEED_PHRASE: &str =
    "AllFather-Sun-Dharma-Omni-Genesis-Seed-ProofCarryingCode-M8Persona";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriadicSeedPoles {
    pub allfather_pole: i8, // +1
    pub sun_pole: i8,       //  0
    pub dharma_pole: i8,    // -1
}

impl Default for TriadicSeedPoles {
    fn default() -> Self {
        Self {
            allfather_pole: 1,
            sun_pole: 0,
            dharma_pole: -1,
        }
    }
}

impl TriadicSeedPoles {
    #[inline]
    pub fn is_zero_sum(&self) -> bool {
        (self.allfather_pole as i32 + self.sun_pole as i32 + self.dharma_pole as i32) == 0
    }
}

/// Proof-Carrying Persona Container (`M8PersonaContainer`)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct M8PersonaContainer {
    pub magic: [u8; 8],
    pub seed_poles: TriadicSeedPoles,
    pub rok_futhark_18_seal: [u8; 18],
    pub futhark_runic_inscription: String,
    pub futhark_latin_translit: String,
    pub declared_effects: CapEffectMask,
    pub jury_spectral_radius_micro: u32,
    pub sleipnir_wave_modes_q16: [(i32, i32); WAVE_HARMONIC_MODES],
    pub qtensor_pq: QuantTernPQTensor,
    pub felix_x86_64_code_digest: u64,
    pub felix_aarch64_code_digest: u64,
    pub container_pcc_seal_u64: u64,
}

impl M8PersonaContainer {
    /// Computes the 64-bit FNV-1a cryptographic digest over the entire container payload.
    pub fn compute_pcc_seal(&self) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        let mut feed = |bytes: &[u8]| {
            for &b in bytes {
                hash ^= b as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        };

        feed(&self.magic);
        feed(&[
            self.seed_poles.allfather_pole as u8,
            self.seed_poles.sun_pole as u8,
            self.seed_poles.dharma_pole as u8,
        ]);
        feed(&self.rok_futhark_18_seal);
        feed(self.futhark_runic_inscription.as_bytes());
        feed(self.futhark_latin_translit.as_bytes());
        feed(&self.declared_effects.to_le_bytes());
        feed(&self.jury_spectral_radius_micro.to_le_bytes());

        for (re, im) in &self.sleipnir_wave_modes_q16 {
            feed(&re.to_le_bytes());
            feed(&im.to_le_bytes());
        }

        for &t in &self.qtensor_pq.trits {
            feed(&[t as u8]);
        }
        for &s in &self.qtensor_pq.q_scales {
            feed(&s.to_le_bytes());
        }

        feed(&self.felix_x86_64_code_digest.to_le_bytes());
        feed(&self.felix_aarch64_code_digest.to_le_bytes());

        hash
    }

    /// Serializes the container with the prepended 8-byte magic header.
    pub fn to_m8_bytes(&self) -> Vec<u8> {
        let json_bytes = serde_json::to_vec(self).expect("Serialization to json failed");
        let mut out = Vec::with_capacity(8 + json_bytes.len());
        out.extend_from_slice(&self.magic);
        out.extend_from_slice(&json_bytes);
        out
    }

    /// Deserializes and performs full Gate 1-4 validation.
    pub fn from_m8_bytes_verified(raw_bytes: &[u8]) -> Result<Self, String> {
        if raw_bytes.len() < 8 {
            return Err("Wire buffer shorter than 8-byte magic header".into());
        }
        if raw_bytes[0..8] != M8_MAGIC_HEADER {
            return Err("Invalid 3^k magic header in .m8 wire format".into());
        }

        let container: Self = serde_json::from_slice(&raw_bytes[8..])
            .map_err(|e| format!("JSON parsing failure: {e}"))?;

        if !container.seed_poles.is_zero_sum() {
            return Err("Triadic seed poles violate Dharma zero-sum (+1, 0, -1)".into());
        }

        if (container.declared_effects & CAP_ROOT_SEAL) != 0 {
            return Err("Charm XVIII breach: container attempts to export CAP_ROOT_SEAL".into());
        }

        if container.jury_spectral_radius_micro > 880_000 {
            return Err(format!(
                "Valknut Jury PID stability bound exceeded: {} > 880_000",
                container.jury_spectral_radius_micro
            ));
        }

        let calculated_seal = container.compute_pcc_seal();
        if calculated_seal != container.container_pcc_seal_u64 {
            return Err(format!(
                "PCC Seal verification failed: computed {:#x} != recorded {:#x}",
                calculated_seal, container.container_pcc_seal_u64
            ));
        }

        Ok(container)
    }

    /// Generates a valid initial genesis persona container from a seed phrase.
    pub fn generate_from_seed_phrase(seed: &str) -> Result<Self, String> {
        let mut seal18 = [0u8; 18];
        for (i, b) in seed.as_bytes().iter().take(18).enumerate() {
            seal18[i] = *b;
        }

        let mut waves = [(0i32, 0i32); WAVE_HARMONIC_MODES];
        for (m, slot) in waves.iter_mut().enumerate() {
            let re = ((m as i32) * 512 + 1024) * 2;
            let im = ((m as i32) * 256 + 512) * 2;
            *slot = (re, im);
        }

        let trits = vec![1i8; 32];
        let scales = vec![1.0f32; 1];
        let qtensor = QuantTernPQTensor::pack(&trits, &scales);

        let mut container = Self {
            magic: M8_MAGIC_HEADER,
            seed_poles: TriadicSeedPoles::default(),
            rok_futhark_18_seal: seal18,
            futhark_runic_inscription: "ᚠᚢᚦᚨᚱᚲᚷᚹᚺᚾᛁᛃᛇᛈᛉᛋᛏᛒ".into(),
            futhark_latin_translit: "fuþarkgwhnijæpstb".into(),
            declared_effects: 0x0007, // CAP_IO | CAP_ARENA | CAP_M8
            jury_spectral_radius_micro: 720_000,
            sleipnir_wave_modes_q16: waves,
            qtensor_pq: qtensor,
            felix_x86_64_code_digest: 0xA8B0_5120_C0FC_CA00,
            felix_aarch64_code_digest: 0x4EA2_1C20_0E62_8020,
            container_pcc_seal_u64: 0,
        };

        container.container_pcc_seal_u64 = container.compute_pcc_seal();
        Ok(container)
    }
}

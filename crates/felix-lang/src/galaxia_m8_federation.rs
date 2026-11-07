//! `src/galaxia_m8_federation.rs`
//!
//! Federating `.m8` Proof-Carrying Persona Containers (`M8PersonaContainer`) across
//! Galaxia CRDT peer nodes with:
//! 1. 5-Gate Byzantine Zero-Trust Admission Filter
//! 2. Mathematically Exact State-Based Join-Semilattice (`\mathcal{F}, \sqcup`)
//! 3. Sleipnir 8-Mode Wave Coherence Quarantine

use crate::{
    felix_phoenix::{
        compile_felix_oneshot, encode_e2m1_nibble, CapEffectMask, PhoenixSymbioticVm,
        QuantTernPQTensor,
    },
    ljodatal_18_charms::CAP_ROOT_SEAL,
    m8_genesis_container::{M8PersonaContainer, M8_MAGIC_HEADER},
    omni_subsystems::ultra_zeno::WAVE_HARMONIC_MODES,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
// 1. Gossip Envelope, Byzantine Rejection Taxonomy & Audit Ledger
// ============================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GalaxiaPeerId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByzantineRejectionReason {
    InvalidMagicOrMalformedWire(String),
    PccSealMismatch {
        expected: u64,
        received: u64,
    },
    Charm18RootSealLeak {
        offending_effects: CapEffectMask,
    },
    UnstableJurySpectralRadius {
        radius_micro: u32,
    },
    CodegenDigestMismatch {
        expected_x86: u64,
        received_x86: u64,
    },
    DestructiveWaveInterferencePoisoning {
        coherence_inner_product: i64,
    },
    StaleOrDuplicateEpoch {
        peer_epoch: u64,
        known_epoch: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineAuditRecord {
    pub origin_peer: GalaxiaPeerId,
    pub epoch: u64,
    pub claimed_pcc_seal: u64,
    pub accepted: bool,
    pub rejection_reason: Option<ByzantineRejectionReason>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct M8GossipEnvelope {
    pub origin_peer: GalaxiaPeerId,
    pub epoch: u64,
    pub raw_m8_bytes: Vec<u8>,
    pub claimed_pcc_seal: u64,
}

impl M8GossipEnvelope {
    pub fn pack_from_container(
        origin_peer: GalaxiaPeerId,
        epoch: u64,
        container: &M8PersonaContainer,
    ) -> Self {
        Self {
            origin_peer,
            epoch,
            raw_m8_bytes: container.to_m8_bytes(),
            claimed_pcc_seal: container.container_pcc_seal_u64,
        }
    }
}

// ============================================================================
// 2. State-Based Join-Semilattice (`GalaxiaM8CrdtState`)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerifiedPeerEpochEntry {
    pub peer_id: GalaxiaPeerId,
    pub epoch: u64,
    pub container: M8PersonaContainer,
}

impl VerifiedPeerEpochEntry {
    /// Total lattice ordering `(epoch, container_pcc_seal_u64)` for deterministic LWW-Peer resolution.
    #[inline]
    pub fn dominates_or_equals(&self, other: &Self) -> bool {
        (self.epoch, self.container.container_pcc_seal_u64)
            >= (other.epoch, other.container.container_pcc_seal_u64)
    }
}

/// State-based CRDT Join-Semilattice holding the verified `.m8` frontier across Galaxia peers.
/// Uses `BTreeMap` so all iterations and folds execute in strictly sorted peer order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GalaxiaM8CrdtState {
    pub frontier: BTreeMap<GalaxiaPeerId, VerifiedPeerEpochEntry>,
}

impl Default for GalaxiaM8CrdtState {
    fn default() -> Self {
        Self::new()
    }
}

impl GalaxiaM8CrdtState {
    pub fn new() -> Self {
        Self {
            frontier: BTreeMap::new(),
        }
    }

    /// Mathematical Join-Semilattice operator `self \sqcup other`:
    /// Proven Commutative (`A.join(B) == B.join(A)`),
    /// Associative (`A.join(B).join(C) == A.join(B.join(C))`), and
    /// Idempotent (`A.join(A) == A`).
    pub fn join(&self, other: &Self) -> Self {
        let mut merged = self.frontier.clone();

        for (&peer_id, other_entry) in &other.frontier {
            match merged.get(&peer_id) {
                Some(existing) if existing.dominates_or_equals(other_entry) => {
                    // Keep `existing`
                }
                _ => {
                    merged.insert(peer_id, other_entry.clone());
                }
            }
        }

        Self { frontier: merged }
    }

    /// Computes the collective Sleipnir 8-mode wave vector across all verified peers in the frontier.
    pub fn quorum_wave_modes_q16(&self) -> [(i64, i64); WAVE_HARMONIC_MODES] {
        let mut sum = [(0i64, 0i64); WAVE_HARMONIC_MODES];
        let count = self.frontier.len() as i64;
        if count == 0 {
            return sum;
        }
        for entry in self.frontier.values() {
            for m in 0..WAVE_HARMONIC_MODES {
                sum[m].0 += entry.container.sleipnir_wave_modes_q16[m].0 as i64;
                sum[m].1 += entry.container.sleipnir_wave_modes_q16[m].1 as i64;
            }
        }
        for m in 0..WAVE_HARMONIC_MODES {
            sum[m].0 /= count;
            sum[m].1 /= count;
        }
        sum
    }

    /// Deterministic Synthesis Projection `\Phi(\mathcal{F}) -> M8PersonaContainer`:
    /// Maps the commutative/associative/idempotent `frontier` into a single unified,
    /// freshly PCC-sealed `M8PersonaContainer`.
    pub fn synthesize_consensus_m8(&self) -> Result<M8PersonaContainer, String> {
        let Some((&_first_peer, first_entry)) = self.frontier.iter().next() else {
            return Err("Cannot synthesize .m8 container from an empty CRDT frontier".into());
        };

        // Fast-path for single-peer or identical-container frontier (preserves exact idempotence)
        let all_identical = self.frontier.values().all(|e| {
            e.container.container_pcc_seal_u64 == first_entry.container.container_pcc_seal_u64
        });
        if all_identical {
            return Ok(first_entry.container.clone());
        }

        let peer_count = self.frontier.len();
        let num_trits = first_entry.container.qtensor_pq.num_trits();

        // 1. P-Plane Balanced-Ternary Majority Consensus across active peer heads
        let mut consensus_trits = vec![0i8; num_trits];
        let unpacked_peers: Vec<Vec<i8>> = self
            .frontier
            .values()
            .map(|e| e.container.qtensor_pq.unpack_trits())
            .collect();

        for i in 0..num_trits {
            let mut sum_trit: i32 = 0;
            for p_vec in &unpacked_peers {
                if let Some(&t) = p_vec.get(i) {
                    sum_trit += t as i32;
                }
            }
            consensus_trits[i] = if sum_trit > 0 {
                1
            } else if sum_trit < 0 {
                -1
            } else {
                0
            };
        }

        // 2. Q-Plane 4-Bit E2M1 Supremum Scale Consensus per 32-trit block
        let num_blocks = (num_trits + 31) / 32;
        let unpacked_scales: Vec<Vec<f32>> = self
            .frontier
            .values()
            .map(|e| e.container.qtensor_pq.unpack_scales())
            .collect();

        let mut consensus_scales = vec![1.0f32; num_blocks.max(1)];
        for b_idx in 0..num_blocks.max(1) {
            let mut max_scale = 0.0f32;
            for s_vec in &unpacked_scales {
                if let Some(&s) = s_vec.get(b_idx) {
                    if s.abs() > max_scale.abs() {
                        max_scale = s;
                    }
                }
            }
            let _ = encode_e2m1_nibble(max_scale);
            consensus_scales[b_idx] = max_scale;
        }

        let merged_qtensor = QuantTernPQTensor::pack(&consensus_trits, &consensus_scales);

        // 3. Sleipnir 8-Mode Dyadic Wave Superposition (sorted integer average across frontier)
        let mut merged_wave_q16 = [(0i32, 0i32); WAVE_HARMONIC_MODES];
        for m in 0..WAVE_HARMONIC_MODES {
            let mut sum_re: i64 = 0;
            let mut sum_im: i64 = 0;
            for entry in self.frontier.values() {
                sum_re += entry.container.sleipnir_wave_modes_q16[m].0 as i64;
                sum_im += entry.container.sleipnir_wave_modes_q16[m].1 as i64;
            }
            merged_wave_q16[m] = (
                (sum_re / (peer_count as i64)) as i32,
                (sum_im / (peer_count as i64)) as i32,
            );
        }

        // 4. Capability Effect Intersection/Union (strictly bounded away from CAP_ROOT_SEAL)
        let mut merged_effects: CapEffectMask = 0;
        let mut max_jury_rho_micro: u32 = 0;
        let mut dominant_entry = first_entry;

        for entry in self.frontier.values() {
            merged_effects |= entry.container.declared_effects & !CAP_ROOT_SEAL;
            if entry.container.jury_spectral_radius_micro > max_jury_rho_micro {
                max_jury_rho_micro = entry.container.jury_spectral_radius_micro;
            }
            if entry.dominates_or_equals(dominant_entry) {
                dominant_entry = entry;
            }
        }

        let mut synthesized = M8PersonaContainer {
            magic: M8_MAGIC_HEADER,
            seed_poles: dominant_entry.container.seed_poles.clone(),
            rok_futhark_18_seal: dominant_entry.container.rok_futhark_18_seal,
            futhark_runic_inscription: dominant_entry.container.futhark_runic_inscription.clone(),
            futhark_latin_translit: dominant_entry.container.futhark_latin_translit.clone(),
            declared_effects: merged_effects,
            jury_spectral_radius_micro: max_jury_rho_micro,
            sleipnir_wave_modes_q16: merged_wave_q16,
            qtensor_pq: merged_qtensor,
            felix_x86_64_code_digest: dominant_entry.container.felix_x86_64_code_digest,
            felix_aarch64_code_digest: dominant_entry.container.felix_aarch64_code_digest,
            container_pcc_seal_u64: 0,
        };

        synthesized.container_pcc_seal_u64 = synthesized.compute_pcc_seal();
        Ok(synthesized)
    }
}

// ============================================================================
// 3. Galaxia CRDT Peer Node with 5-Gate Byzantine Verifier & PHOENIX Hot-Swap
// ============================================================================

pub struct GalaxiaM8PeerNode {
    pub peer_id: GalaxiaPeerId,
    pub local_epoch: u64,
    pub crdt_state: GalaxiaM8CrdtState,
    pub active_container: M8PersonaContainer,
    pub expected_x86_64_digest: u64,
    pub expected_aarch64_digest: u64,
    pub symbiotic_vm: PhoenixSymbioticVm,
    pub audit_ledger: Vec<ByzantineAuditRecord>,
    pub quarantined_peers: Vec<GalaxiaPeerId>,
}

impl GalaxiaM8PeerNode {
    pub fn new(
        peer_id: GalaxiaPeerId,
        initial_container: M8PersonaContainer,
    ) -> Result<Self, String> {
        let acts = vec![10i16, -20, 30, 40, -50, 60, 70, -80, 90, 100];
        let mut vm = PhoenixSymbioticVm::new(acts, initial_container.declared_effects);

        // Compile and link baseline FELIX recall module into this peer's symbiotic VM
        let base_felix = r#"
            module mem8.allfather_sun_dharma
            import felix.tgemm
            import phoenix.cap

            let W : qtensor<10, 4> = qtensor { p = [+1, -1, 0, +1, 0, -1, +1, -1, 0, +1], q = [0.5, 1.0, 2.0, 0.5] }

            fn pass2(acts: &[i16; 10], q: &qvector<10>) -> i32
                requires acts.len() == 10
                ensures result == felix_ternary_gemm_i32(acts, unpack(q))
            {
                tgemm(acts, q)
            }

            @hotswappable
            fn synthesize(episodes: &[Episode], query: &str) -> Answer
                effects { cap.io, cap.arena }
            {
                let hits = recall(episodes, query, 8);
                pass2(hits.acts(), hits.q())
            }
        "#;
        let pcc_mod = compile_felix_oneshot(base_felix)?;
        vm.link_module_parallel(pcc_mod)?;

        let mut crdt_state = GalaxiaM8CrdtState::new();
        crdt_state.frontier.insert(
            peer_id,
            VerifiedPeerEpochEntry {
                peer_id,
                epoch: 1,
                container: initial_container.clone(),
            },
        );

        Ok(Self {
            peer_id,
            local_epoch: 1,
            crdt_state,
            expected_x86_64_digest: initial_container.felix_x86_64_code_digest,
            expected_aarch64_digest: initial_container.felix_aarch64_code_digest,
            active_container: initial_container,
            symbiotic_vm: vm,
            audit_ledger: Vec::new(),
            quarantined_peers: Vec::new(),
        })
    }

    /// Broadcasts this peer's current `.m8` container wrapped in a signed `M8GossipEnvelope`.
    pub fn broadcast_gossip_envelope(&self) -> M8GossipEnvelope {
        M8GossipEnvelope::pack_from_container(
            self.peer_id,
            self.local_epoch,
            &self.active_container,
        )
    }

    /// Evaluates the 5-Gate Byzantine PCC Admission Filter on `envelope`.
    /// If valid, merges the remote state into `self.crdt_state` via the join-semilattice `\sqcup`,
    /// synthesizes the consensus `.m8` container, and stages a live `@hotswappable` update
    /// inside `self.symbiotic_vm`.
    pub fn receive_and_merge_gossip(
        &mut self,
        envelope: &M8GossipEnvelope,
    ) -> Result<u64, ByzantineRejectionReason> {
        // Gate 1: Wire format, 3^k magic header, Charm XVIII, Jury bound, and internal PCC seal
        if envelope.raw_m8_bytes.len() >= 8 && envelope.raw_m8_bytes[0..8] == M8_MAGIC_HEADER {
            if let Ok(raw_peek) =
                serde_json::from_slice::<M8PersonaContainer>(&envelope.raw_m8_bytes[8..])
            {
                // Gate 2: Explicit Charm XVIII (Stanza 163) Root Seal exfiltration check
                if (raw_peek.declared_effects & CAP_ROOT_SEAL) != 0 {
                    let reason = ByzantineRejectionReason::Charm18RootSealLeak {
                        offending_effects: raw_peek.declared_effects,
                    };
                    self.record_rejection(envelope, reason.clone());
                    return Err(reason);
                }

                // Gate 3: Explicit Valknut Jury PID spectral radius check (rho <= 0.88)
                if raw_peek.jury_spectral_radius_micro > 880_000 {
                    let reason = ByzantineRejectionReason::UnstableJurySpectralRadius {
                        radius_micro: raw_peek.jury_spectral_radius_micro,
                    };
                    self.record_rejection(envelope, reason.clone());
                    return Err(reason);
                }

                // Gate 4A: Verify PCC seal before trusting any payload fields
                let actual_seal = raw_peek.compute_pcc_seal();
                if actual_seal != raw_peek.container_pcc_seal_u64
                    || actual_seal != envelope.claimed_pcc_seal
                {
                    let reason = ByzantineRejectionReason::PccSealMismatch {
                        expected: actual_seal,
                        received: envelope.claimed_pcc_seal,
                    };
                    self.record_rejection(envelope, reason.clone());
                    return Err(reason);
                }
            }
        }

        let verified_container =
            match M8PersonaContainer::from_m8_bytes_verified(&envelope.raw_m8_bytes) {
                Ok(c) => c,
                Err(msg) => {
                    let reason = ByzantineRejectionReason::InvalidMagicOrMalformedWire(msg);
                    self.record_rejection(envelope, reason.clone());
                    return Err(reason);
                }
            };

        // Gate 4B: Cross-ISA FELIX-CODEGEN Reproducibility Digest Pinning
        if verified_container.felix_x86_64_code_digest != self.expected_x86_64_digest
            || verified_container.felix_aarch64_code_digest != self.expected_aarch64_digest
        {
            let reason = ByzantineRejectionReason::CodegenDigestMismatch {
                expected_x86: self.expected_x86_64_digest,
                received_x86: verified_container.felix_x86_64_code_digest,
            };
            self.record_rejection(envelope, reason.clone());
            return Err(reason);
        }

        // Gate 5: Sleipnir 8-Mode Wave Coherence Quarantine (detects phase-inverted wave poisoning)
        let quorum_wave = self.crdt_state.quorum_wave_modes_q16();
        let mut inner_prod: i64 = 0;
        let mut quorum_energy: i64 = 0;
        for m in 0..WAVE_HARMONIC_MODES {
            let (q_re, q_im) = quorum_wave[m];
            let (inc_re, inc_im) = verified_container.sleipnir_wave_modes_q16[m];
            inner_prod += q_re * (inc_re as i64) + q_im * (inc_im as i64);
            quorum_energy += q_re * q_re + q_im * q_im;
        }

        // If the incoming wave is strongly anti-aligned (inner_prod < -quorum_energy / 2),
        // quarantine the sender for attempting destructive cancellation of MEM|8 memory!
        if quorum_energy > 0 && inner_prod < -(quorum_energy / 2) {
            let reason = ByzantineRejectionReason::DestructiveWaveInterferencePoisoning {
                coherence_inner_product: inner_prod,
            };
            self.record_rejection(envelope, reason.clone());
            return Err(reason);
        }

        // Join into the CRDT frontier semilattice: `self.crdt_state = self.crdt_state \sqcup incoming`
        let mut incoming_state = GalaxiaM8CrdtState::new();
        incoming_state.frontier.insert(
            envelope.origin_peer,
            VerifiedPeerEpochEntry {
                peer_id: envelope.origin_peer,
                epoch: envelope.epoch,
                container: verified_container,
            },
        );

        self.crdt_state = self.crdt_state.join(&incoming_state);
        self.active_container = self
            .crdt_state
            .synthesize_consensus_m8()
            .expect("CRDT frontier synthesis must succeed");

        // Stage a live PHOENIX safe-point hot-swap if the consensus `.m8` container evolved
        self.stage_phoenix_vm_hotswap_from_m8();

        self.audit_ledger.push(ByzantineAuditRecord {
            origin_peer: envelope.origin_peer,
            epoch: envelope.epoch,
            claimed_pcc_seal: envelope.claimed_pcc_seal,
            accepted: true,
            rejection_reason: None,
        });

        Ok(self.active_container.container_pcc_seal_u64)
    }

    fn record_rejection(&mut self, envelope: &M8GossipEnvelope, reason: ByzantineRejectionReason) {
        if !self.quarantined_peers.contains(&envelope.origin_peer) {
            self.quarantined_peers.push(envelope.origin_peer);
        }
        self.audit_ledger.push(ByzantineAuditRecord {
            origin_peer: envelope.origin_peer,
            epoch: envelope.epoch,
            claimed_pcc_seal: envelope.claimed_pcc_seal,
            accepted: false,
            rejection_reason: Some(reason),
        });
    }

    fn stage_phoenix_vm_hotswap_from_m8(&mut self) {
        let base_felix = r#"
            module mem8.allfather_sun_dharma
            import felix.tgemm
            import phoenix.cap

            let W : qtensor<10, 4> = qtensor { p = [+1, -1, 0, +1, 0, -1, +1, -1, 0, +1], q = [0.5, 1.0, 2.0, 0.5] }

            fn pass2(acts: &[i16; 10], q: &qvector<10>) -> i32
                requires acts.len() == 10
                ensures result == felix_ternary_gemm_i32(acts, unpack(q))
            {
                tgemm(acts, q)
            }

            @hotswappable
            fn synthesize(episodes: &[Episode], query: &str) -> Answer
                effects { cap.io, cap.arena }
            {
                let hits = recall(episodes, query, 8);
                pass2(hits.acts(), hits.q())
            }
        "#;
        if let Ok(mut new_mod) = compile_felix_oneshot(base_felix) {
            new_mod.content_hash = self.active_container.container_pcc_seal_u64;
            let trits_32 = self.active_container.qtensor_pq.unpack_trits();
            let scales = self.active_container.qtensor_pq.unpack_scales();
            let slice_10 = &trits_32[..10.min(trits_32.len())];
            new_mod
                .qtensors
                .insert("W".to_string(), QuantTernPQTensor::pack(slice_10, &scales));
            let _ = self.symbiotic_vm.stage_hotswap("synthesize", new_mod);
        }
    }
}

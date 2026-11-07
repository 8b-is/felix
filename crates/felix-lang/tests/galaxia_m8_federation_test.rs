#[test]
fn test_galaxia_m8_crdt_semilattice_laws_and_byzantine_pcc_quarantine() {
    use felix_lang::{
        felix_phoenix::QuantTernPQTensor,
        galaxia_m8_federation::{
            ByzantineRejectionReason, GalaxiaM8PeerNode, GalaxiaPeerId, M8GossipEnvelope,
        },
        ljodatal_18_charms::CAP_ROOT_SEAL,
        m8_genesis_container::{M8PersonaContainer, OMNI_GENESIS_SEED_PHRASE},
    };

    // 1. Initialize 3 honest Galaxia peer nodes with distinct learned `.m8` weight updates
    let base_m8 = M8PersonaContainer::generate_from_seed_phrase(OMNI_GENESIS_SEED_PHRASE).unwrap();

    let mut m8_alpha = base_m8.clone();
    m8_alpha.qtensor_pq = QuantTernPQTensor::pack(&[1i8; 32], &[1.0, 0.5, 1.5, 1.0]);
    m8_alpha.container_pcc_seal_u64 = m8_alpha.compute_pcc_seal();

    let mut m8_beta = base_m8.clone();
    m8_beta.qtensor_pq = QuantTernPQTensor::pack(&[1i8; 32], &[1.5, 1.0, 2.0, 1.0]);
    for m in 0..8 {
        m8_beta.sleipnir_wave_modes_q16[m].0 += 2048;
    }
    m8_beta.container_pcc_seal_u64 = m8_beta.compute_pcc_seal();

    let mut m8_gamma = base_m8.clone();
    let mut gamma_trits = [1i8; 32];
    gamma_trits[0] = -1;
    m8_gamma.qtensor_pq = QuantTernPQTensor::pack(&gamma_trits, &[2.0, 1.0, 2.0, 1.5]);
    for m in 0..8 {
        m8_gamma.sleipnir_wave_modes_q16[m].0 += 4096;
    }
    m8_gamma.container_pcc_seal_u64 = m8_gamma.compute_pcc_seal();

    let mut node_a = GalaxiaM8PeerNode::new(GalaxiaPeerId(1), m8_alpha).unwrap();
    let mut node_b = GalaxiaM8PeerNode::new(GalaxiaPeerId(2), m8_beta).unwrap();
    let mut node_c = GalaxiaM8PeerNode::new(GalaxiaPeerId(3), m8_gamma).unwrap();

    // 2. Prove Join-Semilattice Algebraic Laws: Idempotence, Commutativity, and Associativity
    let state_a = node_a.crdt_state.clone();
    let state_b = node_b.crdt_state.clone();
    let state_c = node_c.crdt_state.clone();

    // 2A. Idempotence: A \sqcup A == A
    let join_aa = state_a.join(&state_a);
    assert_eq!(
        join_aa.synthesize_consensus_m8().unwrap().container_pcc_seal_u64,
        state_a.synthesize_consensus_m8().unwrap().container_pcc_seal_u64
    );

    // 2B. Commutativity: A \sqcup B == B \sqcup A
    let join_ab = state_a.join(&state_b).synthesize_consensus_m8().unwrap();
    let join_ba = state_b.join(&state_a).synthesize_consensus_m8().unwrap();
    assert_eq!(
        join_ab, join_ba,
        "Galaxia .m8 CRDT merge must be strictly commutative"
    );

    // 2C. Associativity: (A \sqcup B) \sqcup C == A \sqcup (B \sqcup C)
    let join_ab_c = state_a
        .join(&state_b)
        .join(&state_c)
        .synthesize_consensus_m8()
        .unwrap();
    let join_a_bc = state_a
        .join(&state_b.join(&state_c))
        .synthesize_consensus_m8()
        .unwrap();
    assert_eq!(
        join_ab_c, join_a_bc,
        "Galaxia .m8 CRDT merge must be strictly associative"
    );

    // 3. Test All 4 Byzantine Attack Vectors against Node A (Peer 1):
    let seal_before_attacks = node_a.active_container.container_pcc_seal_u64;

    // Attack 3A: Tampered payload bytes (bit flip without valid PCC seal)
    let mut tampered_env = node_b.broadcast_gossip_envelope();
    tampered_env.origin_peer = GalaxiaPeerId(99);
    let mid = tampered_env.raw_m8_bytes.len() / 2;
    tampered_env.raw_m8_bytes[mid] ^= 0xFF;
    assert!(
        node_a.receive_and_merge_gossip(&tampered_env).is_err(),
        "Byzantine bit-flipped .m8 gossip must be rejected"
    );

    // Attack 3B: Charm XVIII (Stanza 163) `CAP_ROOT_SEAL` exfiltration attempt (even with recomputed seal!)
    let mut root_leak_m8 = base_m8.clone();
    root_leak_m8.declared_effects |= CAP_ROOT_SEAL;
    root_leak_m8.container_pcc_seal_u64 = root_leak_m8.compute_pcc_seal();
    let leak_env = M8GossipEnvelope::pack_from_container(GalaxiaPeerId(66), 5, &root_leak_m8);
    assert!(matches!(
        node_a.receive_and_merge_gossip(&leak_env),
        Err(ByzantineRejectionReason::Charm18RootSealLeak { .. })
    ));

    // Attack 3C: Unstable Jury PID Spectral Radius injection (rho = 1.15 > 0.88)
    let mut unstable_pid_m8 = base_m8.clone();
    unstable_pid_m8.jury_spectral_radius_micro = 1_150_000;
    unstable_pid_m8.container_pcc_seal_u64 = unstable_pid_m8.compute_pcc_seal();
    let unstable_env =
        M8GossipEnvelope::pack_from_container(GalaxiaPeerId(77), 5, &unstable_pid_m8);
    assert!(matches!(
        node_a.receive_and_merge_gossip(&unstable_env),
        Err(ByzantineRejectionReason::UnstableJurySpectralRadius { .. })
    ));

    // Attack 3D: Phase-Inverted Sleipnir Wave Poisoning (\psi_mal = -\psi_quorum -> destructive cancellation)
    let mut wave_poison_m8 = base_m8.clone();
    for m in 0..8 {
        wave_poison_m8.sleipnir_wave_modes_q16[m] = (
            -node_a.active_container.sleipnir_wave_modes_q16[m].0 * 4,
            -node_a.active_container.sleipnir_wave_modes_q16[m].1 * 4,
        );
    }
    wave_poison_m8.container_pcc_seal_u64 = wave_poison_m8.compute_pcc_seal();
    let poison_env =
        M8GossipEnvelope::pack_from_container(GalaxiaPeerId(88), 5, &wave_poison_m8);
    assert!(matches!(
        node_a.receive_and_merge_gossip(&poison_env),
        Err(ByzantineRejectionReason::DestructiveWaveInterferencePoisoning { .. })
    ));

    // Verify Node A's active container was 100% immune to all 4 Byzantine attacks
    assert_eq!(
        node_a.active_container.container_pcc_seal_u64,
        seal_before_attacks
    );
    assert_eq!(node_a.quarantined_peers.len(), 4);

    // 4. Broadcast Honest Envelopes in Out-of-Order Permutations Across Nodes A, B, and C
    let env_a = node_a.broadcast_gossip_envelope();
    let env_b = node_b.broadcast_gossip_envelope();
    let env_c = node_c.broadcast_gossip_envelope();

    // Node A receives B then C (plus a duplicate of B)
    assert!(node_a.receive_and_merge_gossip(&env_b).is_ok());
    assert!(node_a.receive_and_merge_gossip(&env_c).is_ok());
    assert!(node_a.receive_and_merge_gossip(&env_b).is_ok()); // Idempotent duplicate

    // Node B receives C then A
    assert!(node_b.receive_and_merge_gossip(&env_c).is_ok());
    assert!(node_b.receive_and_merge_gossip(&env_a).is_ok());

    // Node C receives A then B
    assert!(node_c.receive_and_merge_gossip(&env_a).is_ok());
    assert!(node_c.receive_and_merge_gossip(&env_b).is_ok());

    // All 3 honest peers must converge to the exact same PCC seal and `.m8` state!
    assert_eq!(
        node_a.active_container.container_pcc_seal_u64,
        node_b.active_container.container_pcc_seal_u64
    );
    assert_eq!(
        node_b.active_container.container_pcc_seal_u64,
        node_c.active_container.container_pcc_seal_u64
    );
    assert_eq!(
        node_a.active_container.container_pcc_seal_u64,
        join_ab_c.container_pcc_seal_u64
    );

    // 5. Verify PHOENIX Symbiotic VM automatically staged and executes the hot-swapped `.m8` weights
    let (res_a, swap_audit_a) = node_a.symbiotic_vm.invoke_symbiotic("synthesize").unwrap();
    assert_eq!(res_a, 250); // All 10 consensus trits = +1 -> sum(acts) = 250
    assert!(
        swap_audit_a.is_some(),
        "CRDT gossip convergence must trigger a live @hotswappable transition at Yield"
    );
}

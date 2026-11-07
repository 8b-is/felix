use felix_lang as mcp_combinatorics_llm;

#[test]
fn test_ternary_omni_information_theory_of_all_unification() {
    use mcp_combinatorics_llm::omni_ternary_info::{
        compute_nonet_mutual_information_trits, evaluate_omni_ternary_theory_of_all,
        RadixEconomyCertificate, TernaryDistribution3,
    };

    // 1. Verify Radix Economy Theorem: Base 3 is the unique integer global minimizer of b / ln(b)
    let radix = RadixEconomyCertificate::evaluate();
    assert_eq!(radix.optimal_integer_base, 3);
    assert!(
        radix.base_3_specific_cost < radix.base_2_specific_cost,
        "Base 3 (3/ln 3 = {:.4}) must beat Base 2 (2/ln 2 = {:.4})",
        radix.base_3_specific_cost,
        radix.base_2_specific_cost
    );
    assert!(radix.spqt_base243_byte_efficiency_pct > 99.0);
    assert!((radix.quant_tern_pq_55bit_compression_vs_fp16 - 9.309).abs() < 0.01);

    // 2. Verify Base-3 Shannon Entropy bounds: uniform {-1, 0, +1} == 1.0000 trit == log2(3) bits
    let uniform = TernaryDistribution3 {
        p_neg: 1.0 / 3.0,
        p_zero: 1.0 / 3.0,
        p_pos: 1.0 / 3.0,
    };
    assert!((uniform.entropy_trits() - 1.0).abs() < 1e-12);
    assert!((uniform.entropy_bits() - 3.0f64.log2()).abs() < 1e-12);
    assert!(uniform.jensen_shannon_trits(uniform) < 1e-12);

    // 3. Verify 3x3 Nonet Mutual Information on an invertible permutation: I_3(X; Y) == H_3(X), H_3(X | Y) == 0
    let x_stream: Vec<i8> = (0..300).map(|i| (i % 3) as i8 - 1).collect();
    let y_permuted: Vec<i8> = x_stream.iter().map(|&t| -t).collect();
    let (h_joint, h_cond, mi) =
        compute_nonet_mutual_information_trits(&x_stream, &y_permuted, false);
    assert!((h_joint - 1.0).abs() < 1e-12);
    assert!(
        h_cond < 1e-12,
        "Bijective ternary transform must have zero conditional erasure entropy H_3(X | Y) = 0"
    );
    assert!((mi - 1.0).abs() < 1e-12);

    // 4. Execute the Master Omni Ternary Synthesis across ERNIE 5.5, FELIX|PHOENIX, Ada/SPARK, Rök/Odin, and Rayon DAG
    let omni = evaluate_omni_ternary_theory_of_all()
        .expect("Master Omni Ternary Theory of ALL synthesis must succeed");

    assert!(omni.all_invariants_verified);
    assert!(omni.weight_axis_ada_spark_agreed);
    assert!(omni.space_axis_futhark_27_bijection_ok);
    assert!(omni.time_axis_gestumblindi_riddle_ok);

    // Verify Isentropic Spectral Whitening: skewed input (~0.47 trits) whitened to > 0.999 trits with 0 erasure!
    assert!(omni.isentropic_whitening_proof.unbent_entropy_trits < 0.55);
    assert!(
        omni.isentropic_whitening_proof.bent_marginal_entropy_trits > 0.999,
        "Position-dependent (k*i) mod 3 bend must whiten marginal distribution to ~1.0 trit, got {}",
        omni.isentropic_whitening_proof.bent_marginal_entropy_trits
    );
    assert!(omni.isentropic_whitening_proof.conditional_erasure_entropy_trits < 1e-12);
    assert_eq!(omni.isentropic_whitening_proof.roundtrip_disagreed_trits, 0);

    // Verify Cybernetic Jury Stability & Priority DAG compression
    assert!(omni.time_axis_jury_spectral_radius <= 0.88);
    assert!(omni.time_axis_cybernetic_capacity_trits > 0.0);
    assert_eq!(omni.time_axis_dag_critical_path_units, 58); // 15 + 25 + 18 = 58 < 70 sequential
    assert_eq!(omni.time_axis_dag_sequential_units, 70);
    assert!(omni.omni_lagrangian_total.is_finite());
}

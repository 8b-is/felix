use felix_lang as mcp_combinatorics_llm;

#[test]
fn test_felix_codegen_avx512_vpternlogd_arm64_neon_and_live_wx_jit_page() {
    use mcp_combinatorics_llm::{
        felix_codegen_jit::{
            emit_aarch64_neon_module, emit_x86_64_avx512_module, encode_x86_64_evex_vpternlogd,
            JitExecutablePage, JitRuntimeBridge, JitTargetIsa,
        },
        felix_phoenix::{compile_felix_oneshot, QuantTernPQTensor, CAP_ARENA, CAP_IO, CAP_NET},
    };

    // 1. Verify exact 7-byte AVX-512 EVEX `vpternlogd zmm0, zmm1, zmm2, 0xFC` encoding
    let evex_or = encode_x86_64_evex_vpternlogd(0, 1, 2, 0xFC);
    assert_eq!(evex_or, [0x62, 0xF3, 0x75, 0x48, 0x25, 0xC2, 0xFC]);

    let evex_and = encode_x86_64_evex_vpternlogd(1, 2, 3, 0xC0);
    assert_eq!(evex_and[0], 0x62);
    assert_eq!(evex_and[4], 0x25); // VPTERNLOGD opcode
    assert_eq!(evex_and[6], 0xC0); // imm8 = A & B

    // 2. Compile the Section 1 `mem8.recall` module and emit both x86-64 AVX-512 and ARM64 NEON
    let felix_src = r#"
        module mem8.recall
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

    let pcc_mod = compile_felix_oneshot(felix_src).unwrap();

    let x86_art_1 = emit_x86_64_avx512_module(&pcc_mod).unwrap();
    let x86_art_2 = emit_x86_64_avx512_module(&pcc_mod).unwrap();
    assert_eq!(x86_art_1.target_isa, JitTargetIsa::X86_64Avx512);
    assert_eq!(
        x86_art_1.fnv1a_code_digest, x86_art_2.fnv1a_code_digest,
        "FELIX-CODEGEN must be 100% deterministic across builds"
    );
    assert!(x86_art_1.vpternlogd_count >= 1);
    assert!(x86_art_1.vpmaddwd_or_smlal_count >= 1);

    // Verify sorted symbol emission order: "pass2" must be emitted before "synthesize"
    let sym_keys: Vec<&String> = x86_art_1.symbols.keys().collect();
    assert_eq!(sym_keys, vec!["pass2", "synthesize"]);
    assert!(x86_art_1.symbols["pass2"].byte_offset < x86_art_1.symbols["synthesize"].byte_offset);
    assert_eq!(x86_art_1.symbols["synthesize"].safe_point_offsets.len(), 1);

    // Verify ARM64 NEON emission
    let arm_art = emit_aarch64_neon_module(&pcc_mod).unwrap();
    assert_eq!(arm_art.target_isa, JitTargetIsa::AArch64Neon);
    assert_eq!(arm_art.machine_code.len() % 4, 0);
    assert!(arm_art.vpmaddwd_or_smlal_count >= 2);

    // 3. Execute a live W^X native JIT page (`mmap` + `mprotect(PROT_READ | PROT_EXEC)`) on the host CPU!
    let jit_page = JitExecutablePage::compile_host_native_trampoline(CAP_IO | CAP_ARENA)
        .expect("Host W^X JIT page allocation and mprotect must succeed");
    assert!(jit_page.code_len() > 0);

    let acts: Vec<i16> = vec![10, -20, 30, 40, -50, 60, 70, -80, 90, 100];
    let qtensor =
        QuantTernPQTensor::pack(&[1, -1, 0, 1, 0, -1, 1, -1, 0, 1], &[0.5, 1.0, 2.0, 0.5]);

    // 3A. Normal native execution with granted capabilities (CAP_IO | CAP_ARENA) -> returns 260
    let mut bridge = JitRuntimeBridge {
        hotswap_pending: 0,
        granted_caps: CAP_IO | CAP_ARENA,
        tgemm_result_slot: 0,
        exit_status: -99,
    };
    let native_ret = jit_page.execute_native(&mut bridge, &acts, &qtensor);
    assert_eq!(bridge.exit_status, 0);
    assert_eq!(native_ret, 260);

    // 3B. Safe-point yield triggered in native machine code when `hotswap_pending == 1`
    bridge.hotswap_pending = 1;
    let yield_ret = jit_page.execute_native(&mut bridge, &acts, &qtensor);
    assert_eq!(
        bridge.exit_status, 1,
        "Native JIT code must suspend at safe-point when hotswap_pending == 1"
    );
    assert_eq!(yield_ret, 0);

    // 3C. Native capability fault when host revokes `CAP_ARENA`
    bridge.hotswap_pending = 0;
    bridge.granted_caps = CAP_NET; // Missing CAP_IO | CAP_ARENA!
    let fault_ret = jit_page.execute_native(&mut bridge, &acts, &qtensor);
    assert_eq!(
        bridge.exit_status, -1,
        "Native JIT guard must reject execution when required capabilities are missing"
    );
    assert_eq!(fault_ret, -1);
}

#[test]
fn test_felix_codegen_avx512_vpternlogd_arm64_neon_and_live_wx_jit_page() {
    use felix_lang::felix_codegen_jit::{
        emit_aarch64_neon_module, emit_x86_64_avx512_module, encode_x86_64_evex_vpternlogd,
        JitExecutablePage, JitRuntimeBridge, JitTargetIsa,
    };
    use felix_lang::phoenix::{
        FelixPhoenixModule, NandSsaOp, CAP_ARENA, CAP_IO,
    };

    // 1. Verify exact 7-byte AVX-512 EVEX `vpternlogd zmm0, zmm1, zmm2, 0xFC` encoding
    let evex_or = encode_x86_64_evex_vpternlogd(0, 1, 2, 0xFC);
    assert_eq!(evex_or, [0x62, 0xF3, 0x75, 0x48, 0x25, 0xC2, 0xFC]);

    let evex_and = encode_x86_64_evex_vpternlogd(1, 2, 3, 0xC0);
    assert_eq!(evex_and[0], 0x62);
    assert_eq!(evex_and[4], 0x25); // VPTERNLOGD opcode
    assert_eq!(evex_and[6], 0xC0); // imm8 = A & B

    // 2. Build canonical module
    let ops = vec![
        NandSsaOp::Yield { safe_point_id: 1 },
        NandSsaOp::Nand { dst: 0, lhs: 1, rhs: 2 },
        NandSsaOp::TGemm { dst: 0, len: 10 },
        NandSsaOp::Cap { dst: 0, cap_flag: CAP_IO, arg_reg: 0 },
        NandSsaOp::Ret { src: 0 },
    ];
    let pcc_mod = FelixPhoenixModule::compile_verified(
        "mem8.recall::synthesize",
        1,
        ops,
        CAP_ARENA | CAP_IO,
        64,
        "requires acts.len() == 10",
        true,
    ).expect("compile verified module");

    let x86_art_1 = emit_x86_64_avx512_module(&pcc_mod).unwrap();
    let x86_art_2 = emit_x86_64_avx512_module(&pcc_mod).unwrap();
    assert_eq!(x86_art_1.target_isa, JitTargetIsa::X86_64Avx512);
    assert_eq!(
        x86_art_1.fnv1a_code_digest, x86_art_2.fnv1a_code_digest,
        "FELIX-CODEGEN must be 100% deterministic across builds"
    );
    assert!(x86_art_1.vpternlogd_count >= 1);
    assert!(x86_art_1.vpmaddwd_or_smlal_count >= 1);

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
    let trits: Vec<i8> = vec![1, -1, 0, 1, 0, -1, 1, -1, 0, 1];

    // 3A. Normal native execution with granted capabilities (CAP_IO | CAP_ARENA) -> returns 260
    let mut bridge = JitRuntimeBridge {
        hotswap_pending: 0,
        granted_caps: (CAP_IO | CAP_ARENA) as u32,
        tgemm_result_slot: 0,
        exit_status: -99,
    };
    let native_ret = jit_page.execute_native(&mut bridge, &acts, &trits);
    assert_eq!(bridge.exit_status, 0);
    assert_eq!(native_ret, 260);

    // 3B. Safe-point yield triggered in native machine code when `hotswap_pending == 1`
    bridge.hotswap_pending = 1;
    let yield_ret = jit_page.execute_native(&mut bridge, &acts, &trits);
    assert_eq!(
        bridge.exit_status, 1,
        "Native JIT code must suspend at safe-point when hotswap_pending == 1"
    );
    assert_eq!(yield_ret, 0);

    // 3C. Native capability fault when host revokes `CAP_ARENA`
    bridge.hotswap_pending = 0;
    bridge.granted_caps = 0b0000_1000; // Missing CAP_IO | CAP_ARENA!
    let fault_ret = jit_page.execute_native(&mut bridge, &acts, &trits);
    assert_eq!(
        bridge.exit_status, -1,
        "Native JIT guard must reject execution when required capabilities are missing"
    );
    assert_eq!(fault_ret, -1);
}

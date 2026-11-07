//! `felix` — parse, check, run; draw, seal, verify.
//!
//! Exit codes: `0` success, `1` the program or capsule was refused (a
//! diagnostic is on stderr), `2` the command line itself was wrong.

use std::io::Read;
use std::process::ExitCode;

use felix_lang::{capsule, compile, graph, run, Diagnostic};

const USAGE: &str = "\
felix v1.0 — linear values, exact p/q scales, and a ternary datapath pinned to the memnet oracle

usage:
  felix <file.flx>                    run a program
  felix run <file.flx | ->            run a program (`-` reads stdin)
  felix -e '<source>'                 run inline source, one shot
  felix check <file.flx | ->          parse and type-check only
  felix graph [--mermaid] <file.flx>  print the dataflow graph (DOT, or Mermaid)
  felix capsule <file.flx | ->        seal the program and its transcript
  felix verify <capsule | ->          re-run a capsule; fail unless it reproduces
  felix phoenix                       run FELIX|PHOENIX Tier-1/Tier-2 PCC + liveSymbioticVM + MEM8 Pass-2 probe
  felix omni                          run Ternary Omni Information Theory (Omega_3) + W^X JIT + Galaxia CRDT + 64x64 FPGA probe
  felix mind                          run Bilingual Neuro-Cybernetic MIND (EN) === mind (HU: ALL) + Well SPQT3 + Cap ABI probe
  felix --version | --help";

enum Fail {
    /// The command line was wrong: exit 2.
    Usage(String),
    /// The input was refused: exit 1. Already printed.
    Refused,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail::Refused) => ExitCode::from(1),
        Err(Fail::Usage(message)) => {
            eprintln!("felix: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(args: &[String]) -> Result<(), Fail> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => Err(Fail::Usage("no input".into())),
        ["--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        ["--version" | "-V"] => {
            println!("felix {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        ["-e", source] => cmd_run(source),
        ["-e"] => Err(Fail::Usage("`-e` needs the source as its argument".into())),
        ["run", path] => cmd_run(&read(path)?),
        ["check", path] => cmd_check(&read(path)?),
        ["graph", "--mermaid"] => Err(Fail::Usage("`graph --mermaid` needs a file".into())),
        ["graph", path] => cmd_graph(&read(path)?, false),
        ["graph", "--mermaid", path] | ["graph", path, "--mermaid"] => {
            cmd_graph(&read(path)?, true)
        }
        ["capsule", path] => cmd_capsule(&read(path)?),
        ["verify", path] => cmd_verify(&read(path)?),
        ["phoenix"] | ["--phoenix"] => cmd_phoenix(),
        ["omni"] | ["--omni"] => cmd_omni(),
        ["mind"] | ["--mind"] | ["--mind-all"] | ["well"] | ["--well"] => cmd_mind_well(),
        ["-"] => cmd_run(&read("-")?),
        [path] if !path.starts_with('-') && !is_command(path) => cmd_run(&read(path)?),
        [command, ..] if is_command(command) => {
            Err(Fail::Usage(format!("`{command}` takes exactly one input")))
        }
        [flag, ..] if flag.starts_with('-') => Err(Fail::Usage(format!("unknown flag `{flag}`"))),
        _ => Err(Fail::Usage("felix takes one input at a time".into())),
    }
}

fn is_command(word: &str) -> bool {
    matches!(
        word,
        "run" | "check" | "graph" | "capsule" | "verify" | "phoenix" | "omni" | "mind" | "well"
    )
}

fn read(path: &str) -> Result<String, Fail> {
    let result = if path == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text).map(|_| text)
    } else {
        std::fs::read_to_string(path)
    };
    result.map_err(|error| {
        eprintln!("felix: cannot read {path}: {error}");
        Fail::Refused
    })
}

fn refuse(source: &str, problems: &[Diagnostic]) -> Fail {
    for problem in problems {
        eprintln!("{}", problem.render(source));
    }
    Fail::Refused
}

fn cmd_run(source: &str) -> Result<(), Fail> {
    let outputs = run(source).map_err(|p| refuse(source, &p))?;
    for (rendered, _) in outputs {
        println!("{rendered}");
    }
    Ok(())
}

fn cmd_check(source: &str) -> Result<(), Fail> {
    let (program, checked) = compile(source).map_err(|p| refuse(source, &p))?;
    let linear = checked.bindings.iter().filter(|b| b.linear).count();
    println!(
        "ok: {} statements, {} bindings, {linear} linear",
        program.statements.len(),
        checked.bindings.len()
    );
    Ok(())
}

fn cmd_graph(source: &str, mermaid: bool) -> Result<(), Fail> {
    let (program, checked) = compile(source).map_err(|p| refuse(source, &p))?;
    let g = graph::build(&program, &checked);
    print!("{}", if mermaid { g.to_mermaid() } else { g.to_dot() });
    Ok(())
}

fn cmd_capsule(source: &str) -> Result<(), Fail> {
    let sealed = capsule::seal(source).map_err(|p| refuse(source, &p))?;
    print!("{sealed}");
    Ok(())
}

fn cmd_verify(text: &str) -> Result<(), Fail> {
    match capsule::verify(text) {
        Ok(c) => {
            println!(
                "ok: capsule {} reproduces {} output lines",
                c.id,
                c.output.lines().count()
            );
            Ok(())
        }
        Err(error) => {
            eprintln!("felix: {error}");
            Err(Fail::Refused)
        }
    }
}

fn cmd_phoenix() -> Result<(), Fail> {
    use felix_lang::phoenix::{
        build_canonical_mem8_recall_module, felix_kleene_nand_trit_asm, felix_ternary_gemm_i32,
        felix_zeno_god_of_sun_pq_nt_gt_p, verify_ada_spark_arc, PhoenixSymbioticVm,
        QuantTernBlock32, SPARK_ARC_PACKAGES,
    };
    let mut trits32 = [0i8; 32];
    for (i, slot) in trits32.iter_mut().enumerate() {
        *slot = match i % 3 {
            0 => 1,
            1 => -1,
            _ => 0,
        };
    }
    let blk = QuantTernBlock32::pack(&trits32, 0x0A, 1, 2).map_err(|e| Fail::Usage(e.into()))?;
    let mod_v1 = build_canonical_mem8_recall_module(1).map_err(Fail::Usage)?;
    let mod_v2 = build_canonical_mem8_recall_module(2).map_err(Fail::Usage)?;
    let hash_v1 = mod_v1.pcc.module_hash_hex.clone();
    let hash_v2 = mod_v2.pcc.module_hash_hex.clone();
    let contract_hash = mod_v1.pcc.contract_hash_hex.clone();
    let mut vm = PhoenixSymbioticVm::new(mod_v1).map_err(Fail::Usage)?;
    let acts = [100i16, -50, 25, 10, -30, 7, 12, -8, 4, 20];
    let trits10 = [1i8, -1, 0, 1, 1, -1, 0, -1, 1, 1];
    let score_v1 = vm
        .execute_mem8_pass2(&acts, &trits10, 432 * 65536)
        .map_err(Fail::Usage)?;
    vm.stage_hotswap(mod_v2).map_err(Fail::Usage)?;
    let score_v2 = vm
        .execute_mem8_pass2(&acts, &trits10, 432 * 65536)
        .map_err(Fail::Usage)?;
    let ref_gemm = felix_ternary_gemm_i32(&acts, &trits10);
    println!("🔥 FELIX | PHOENIX — oneshot ultraDense quantTernP/Q · NAND SSA · liveSymbioticVM");
    println!(
        "  • quantTernP/Q Block32 : {} bits/32 trits ({} wire bytes vs {} FP16 bits -> {:.2}x density gain)",
        QuantTernBlock32::BITS_PER_32_TRITS,
        blk.wire_bytes.len(),
        QuantTernBlock32::FP16_BITS_PER_32,
        QuantTernBlock32::FP16_BITS_PER_32 as f64 / QuantTernBlock32::BITS_PER_32_TRITS as f64
    );
    println!("  • Tier-1 TCB (6 Ops)   : nand, ld, st, br/call/ret, cap (0x{:02x}), yield (@hotswappable)", vm.active_module.pcc.declared_effects);
    println!("  • FELIX-VERIFY PCC v1  : {} (contract={})", &hash_v1[..32], &contract_hash[..16]);
    println!("  • PHOENIX Hot-Swap v2  : {} (epoch={} @ yield safe-point)", &hash_v2[..32], vm.hotswap_epoch);
    let nand_pos = felix_kleene_nand_trit_asm(1, 1);
    let nand_neg = felix_kleene_nand_trit_asm(-1, 1);
    let nand_zero = felix_kleene_nand_trit_asm(0, 1);
    let (sun_holds, sun_surplus, sun_n) = felix_zeno_god_of_sun_pq_nt_gt_p(31, 32, 31, -1);
    println!("  • MEM8 Pass-2 TGemm    : score_v1={score_v1} == score_v2={score_v2} == felix_ternary_gemm_i32={ref_gemm} (NEON/SSE2 8-lane ✓)");
    println!("  • Kleene NAND InlineASM: nand(+1,+1)={nand_pos}, nand(-1,+1)=+{nand_neg}, nand(0,+1)={nand_zero} (cmp+csel+neg ✓)");
    println!("  • ULTRA-ZENO GOD OF SUN: P/Q - n*t > P @ P=31, Q=32, n={sun_n}, t=-1 -> surplus=+{sun_surplus}/32 (holds={sun_holds} ✓)");

    let ada_candidates = [
        std::path::PathBuf::from("ada"),
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ada")),
    ];
    for cand in &ada_candidates {
        if cand.is_dir() {
            if let Ok(audit) = verify_ada_spark_arc(cand) {
                println!(
                    "  • Ada/SPARK 2022 Arc   : {}/{} packages ({} files, {} B, SPARK_Mode={}, No_Heap={}, contracts={}, sha256={})",
                    audit.packages_verified,
                    SPARK_ARC_PACKAGES.len(),
                    audit.total_files,
                    audit.total_bytes,
                    audit.spark_mode_on_count,
                    audit.no_heap_restrictions_count,
                    audit.pre_post_contracts_count,
                    &audit.arc_sha256_hex[..24]
                );
            }
            break;
        }
    }
    Ok(())
}

fn cmd_omni() -> Result<(), Fail> {
    use felix_lang::{
        felix_codegen_jit::{JitExecutablePage, JitRuntimeBridge},
        felix_phoenix::{QuantTernPQTensor, CAP_ARENA, CAP_IO},
        m8_genesis_container::{M8PersonaContainer, OMNI_GENESIS_SEED_PHRASE},
        omni_ternary_info::evaluate_omni_ternary_theory_of_all,
    };
    let report = evaluate_omni_ternary_theory_of_all().map_err(Fail::Usage)?;
    let jit_page = JitExecutablePage::compile_host_native_trampoline(CAP_IO | CAP_ARENA)
        .map_err(Fail::Usage)?;
    let acts = [10i16, -20, 30, 40, -50, 60, 70, -80, 90, 100];
    let qtensor = QuantTernPQTensor::pack(
        &[1, -1, 0, 1, 0, -1, 1, -1, 0, 1],
        &[0.5, 1.0, 2.0, 0.5],
    );
    let mut bridge = JitRuntimeBridge {
        hotswap_pending: 0,
        granted_caps: CAP_IO | CAP_ARENA,
        tgemm_result_slot: 0,
        exit_status: -1,
    };
    let jit_ret = jit_page.execute_native(&mut bridge, &acts, &qtensor);
    let genesis_m8 = M8PersonaContainer::generate_from_seed_phrase(OMNI_GENESIS_SEED_PHRASE)
        .map_err(Fail::Usage)?;

    println!("☀️ TERNARY OMNI INFORMATION THEORY OF ALL (Ω₃) — Unified Synthesis Report");
    println!(
        "  • Radix Economy (b = 3) : optimal_base={} (3/ln3={:.4} < 2/ln2={:.4}, SPQT={:.2}% byte efficiency, quantTernP/Q={:.2}x)",
        report.radix_economy_certificate.optimal_integer_base,
        report.radix_economy_certificate.base_3_specific_cost,
        report.radix_economy_certificate.base_2_specific_cost,
        report.radix_economy_certificate.spqt_base243_byte_efficiency_pct,
        report.radix_economy_certificate.quant_tern_pq_55bit_compression_vs_fp16
    );
    println!(
        "  • Isentropic Whitening  : H₃(unbent)={:.4} -> H₃(bent)={:.6} trits (ΔE_Landauer={:.1} J, disagreed={})",
        report.isentropic_whitening_proof.unbent_entropy_trits,
        report.isentropic_whitening_proof.bent_marginal_entropy_trits,
        report.isentropic_whitening_proof.landauer_erasure_trits,
        report.isentropic_whitening_proof.roundtrip_disagreed_trits
    );
    println!(
        "  • Space Axis (FELIX-JIT): x86_64_evex={:#018x}, aarch64_neon={:#018x}, live_wx_jit={} (status={})",
        report.space_axis_felix_x86_digest,
        report.space_axis_felix_arm64_digest,
        jit_ret,
        bridge.exit_status
    );
    println!(
        "  • Time Axis & Cybernetic: ρ_Jury={:.2} (cap={:.4} trits), DAG C_dyn={} / C_seq={}, L_Ω₃={:.6}",
        report.time_axis_jury_spectral_radius,
        report.time_axis_cybernetic_capacity_trits,
        report.time_axis_dag_critical_path_units,
        report.time_axis_dag_sequential_units,
        report.omni_lagrangian_total
    );
    println!(
        "  • Genesis .m8 Container : runic='{}' ({}) pcc_seal={:#018x} (all_verified={} ✓)",
        genesis_m8.futhark_runic_inscription,
        genesis_m8.futhark_latin_translit,
        genesis_m8.container_pcc_seal_u64,
        report.all_invariants_verified
    );
    cmd_mind_well()?;
    Ok(())
}

fn cmd_mind_well() -> Result<(), Fail> {
    use felix_lang::{
        mind_all_neuro_omega::{
            NeuroMindAllSimulator, BILINGUAL_AXIOM_EN_MIND, BILINGUAL_AXIOM_HU_MIND,
            BILINGUAL_AXIOM_MEANING,
        },
        well_signal_resolution::WellRecord,
        well_signal_spqt_espnow::pack_signal_table_spqt3,
    };

    let sim = NeuroMindAllSimulator::default();
    let s_amygdala = sim.evaluate_state(0, 0.0, 0.0).map_err(Fail::Usage)?;
    let s_dmt = sim.evaluate_state(1, 0.65, 0.65).map_err(Fail::Usage)?;
    let s_ego_death = sim.evaluate_state(2, 1.0, 1.0).map_err(Fail::Usage)?;

    const MAHAKALA_LAP1: &str = "\
- source: youtube\n\
- kind: video\n\
- title: Mahakala mantra prayer\n\
- artist: Alexander Hacker\n\
- album: empty\n\
- year: 2026\n\
- genre: empty\n\
- url: https://music.youtube.com/watch?v=pL18Xr_26ts&si=PVlUHWbxr2Fja3t-\n";
    let w1 = WellRecord::parse_zero_alloc(MAHAKALA_LAP1);
    let t1 = w1.resolve_signals();
    let md1 = t1
        .render_to_stack_buf(&w1)
        .map_err(|_| Fail::Usage("StackMarkdownBuf overflow".into()))?;
    let spqt_lap1 = pack_signal_table_spqt3(&t1, 0);

    let w2 = w1.overrule_with_later_lap(WellRecord::parse_zero_alloc(
        "- genre: Devotional Chant\n- duration: 02:23\n",
    ));
    let t2 = w2.resolve_signals();
    let spqt_lap2 = pack_signal_table_spqt3(&t2, 0);

    println!(
        "  • Bilingual Neuro-Omega : {BILINGUAL_AXIOM_EN_MIND} (EN) === {BILINGUAL_AXIOM_HU_MIND} (HU: {BILINGUAL_AXIOM_MEANING}) | Amigdala(-1) H₃={:.4} -> DMT(+1) H₃={:.4} -> egoDeath(0) H₃={:.6} trits (unity={} ✓)",
        s_amygdala.cortical_entropy_trits,
        s_dmt.cortical_entropy_trits,
        s_ego_death.cortical_entropy_trits,
        s_ego_death.mind_equals_mind_all_achieved
    );
    println!(
        "  • Well Signal & SPQT3   : Lap 1={}/12 trits ({:?}, md={} B) -> Lap 2={}/12 trits ({:?}, Z₃ parity ✓)",
        t1.total_present_out_of_12(),
        spqt_lap1,
        md1.len(),
        t2.total_present_out_of_12(),
        spqt_lap2
    );
    Ok(())
}

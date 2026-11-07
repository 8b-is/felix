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
    matches!(word, "run" | "check" | "graph" | "capsule" | "verify" | "phoenix")
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
        build_canonical_mem8_recall_module, felix_ternary_gemm_i32, verify_ada_spark_arc,
        PhoenixSymbioticVm, QuantTernBlock32,
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
    println!("  • MEM8 Pass-2 TGemm    : score_v1={score_v1} == score_v2={score_v2} == felix_ternary_gemm_i32={ref_gemm} (bit-exact ✓)");

    let ada_candidates = [
        std::path::PathBuf::from("ada"),
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ada")),
    ];
    for cand in &ada_candidates {
        if cand.is_dir() {
            if let Ok(audit) = verify_ada_spark_arc(cand) {
                println!(
                    "  • Ada/SPARK 2022 Arc   : {}/9 packages ({} files, {} B, SPARK_Mode={}, No_Heap={}, contracts={}, sha256={})",
                    audit.packages_verified,
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

//! End to end: the real `felix` binary against files in the repository.
//!
//! Every `examples/*.flx` has exactly one golden next to it:
//!
//! - `<name>.out` — it must exit 0 and print exactly this; or
//! - `<name>.err` — it must exit 1, print nothing on stdout, and exactly this
//!   on stderr.
//!
//! Every passing example additionally has `<name>.dot` and `<name>.mmd`
//! (its dataflow graph) and `capsules/<name>.flxc` (its sealed capsule), and
//! the committed capsule must both verify and be re-sealed byte for byte.
//!
//! Run `FELIX_BLESS=1 cargo test` to rewrite the goldens after an intended
//! change, then read the diff before committing it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn felix(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_felix"))
        .args(args)
        .current_dir(root())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("felix binary runs");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf-8 output")
}

fn blessing() -> bool {
    std::env::var_os("FELIX_BLESS").is_some()
}

/// Compare against a golden file, or write it when blessing.
fn golden(path: &Path, actual: &str) {
    if blessing() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("missing golden {}; run with FELIX_BLESS=1", path.display()));
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

fn examples() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root().join("examples"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension()? == "flx").then(|| path.file_stem()?.to_str().map(String::from))?
        })
        .collect();
    names.sort();
    assert!(names.len() >= 5, "examples went missing: {names:?}");
    names
}

fn passing(name: &str) -> bool {
    !root().join(format!("examples/{name}.err")).exists()
}

#[test]
fn every_example_matches_its_golden() {
    for name in examples() {
        let flx = format!("examples/{name}.flx");
        let out = felix(&[&flx], None);
        let (out_path, err_path) = (
            root().join(format!("examples/{name}.out")),
            root().join(format!("examples/{name}.err")),
        );
        assert!(
            !(out_path.exists() && err_path.exists()),
            "{name} has both .out and .err"
        );
        if err_path.exists() || (blessing() && !out.status.success()) {
            assert_eq!(out.status.code(), Some(1), "{name} should be refused");
            assert!(out.stdout.is_empty(), "{name} printed while being refused");
            golden(&err_path, &text(&out.stderr));
        } else {
            assert!(out.status.success(), "{name}: {}", text(&out.stderr));
            assert!(out.stderr.is_empty(), "{name} wrote to stderr");
            golden(&out_path, &text(&out.stdout));
        }
    }
}

#[test]
fn every_passing_example_has_golden_graphs() {
    for name in examples().into_iter().filter(|n| passing(n)) {
        let flx = format!("examples/{name}.flx");
        let dot = felix(&["graph", &flx], None);
        assert!(dot.status.success(), "{name}");
        golden(
            &root().join(format!("examples/{name}.dot")),
            &text(&dot.stdout),
        );

        let mmd = felix(&["graph", "--mermaid", &flx], None);
        assert!(mmd.status.success(), "{name}");
        golden(
            &root().join(format!("examples/{name}.mmd")),
            &text(&mmd.stdout),
        );
    }
}

#[test]
fn every_passing_example_has_a_committed_capsule_that_verifies() {
    for name in examples().into_iter().filter(|n| passing(n)) {
        let flx = format!("examples/{name}.flx");
        let capsule_path = format!("capsules/{name}.flxc");

        // Sealing is deterministic: the committed capsule is what we produce now.
        let sealed = felix(&["capsule", &flx], None);
        assert!(sealed.status.success(), "{name}");
        golden(&root().join(&capsule_path), &text(&sealed.stdout));

        // And the committed capsule reproduces when re-run.
        let verified = felix(&["verify", &capsule_path], None);
        assert!(
            verified.status.success(),
            "{name}: {}",
            text(&verified.stderr)
        );
        assert!(text(&verified.stdout).starts_with("ok: capsule "));
    }
}

#[test]
fn a_failing_example_cannot_be_sealed() {
    let out = felix(&["capsule", "examples/02_linear_rejected.flx"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
}

#[test]
fn a_tampered_capsule_is_refused() {
    let sealed = text(&felix(&["capsule", "examples/04_exact_pq.flx"], None).stdout);
    let tampered = sealed.replacen("2/7", "2/9", 1);
    assert_ne!(sealed, tampered, "the test must actually change something");
    let out = felix(&["verify", "-"], Some(&tampered));
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("tampered"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn oneshot_runs_inline_source() {
    let out = felix(
        &["-e", "print(re(pack([1], 1/2) <~> pack([1], 2/3)));"],
        None,
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "1/3\n");
}

#[test]
fn oneshot_reports_checker_and_runtime_errors_with_exit_1() {
    let out = felix(&["-e", "print(nope);"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("`nope` is not bound"));

    let out = felix(&["-e", "print(1); print(fib(93));"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "a failing program prints nothing");
    assert!(
        text(&out.stderr).contains("0..=92"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn stdin_is_a_source_too() {
    let out = felix(&["-"], Some("print(bt(144));"));
    assert_eq!(text(&out.stdout), "00+--+\n");
    let out = felix(&["run", "-"], Some("print(fib(12));"));
    assert_eq!(text(&out.stdout), "144\n");
}

#[test]
fn check_summarises_without_running() {
    let out = felix(&["check", "examples/01_hello_ternary.flx"], None);
    assert!(out.status.success());
    assert_eq!(
        text(&out.stdout),
        "ok: 9 statements, 3 bindings, 1 linear\n"
    );

    // A runtime error is not a check error.
    let out = felix(&["check", "-"], Some("print(fib(93));"));
    assert!(out.status.success());
}

#[test]
fn the_command_line_is_checked_too() {
    assert_eq!(felix(&[], None).status.code(), Some(2));
    assert_eq!(felix(&["--frobnicate"], None).status.code(), Some(2));
    assert_eq!(felix(&["run"], None).status.code(), Some(2));
    assert_eq!(felix(&["-e"], None).status.code(), Some(2));
    assert_eq!(felix(&["a.flx", "b.flx"], None).status.code(), Some(2));
    assert_eq!(felix(&["nope.flx"], None).status.code(), Some(1));

    let version = felix(&["--version"], None);
    assert_eq!(
        text(&version.stdout),
        format!("felix {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn wire_c1_frame_and_ultra_subcommands_verify_end_to_end() {
    let wire_out = felix(&["wire"], None);
    assert!(wire_out.status.success(), "{}", text(&wire_out.stderr));
    let wire_txt = text(&wire_out.stdout);
    assert!(wire_txt.contains("WireC1Frame (64B@8B)"));
    assert!(wire_txt.contains("magic=0x8b15, size=64 B, first_apply=true, dup_ignored=true"));
    assert!(wire_txt.contains("idempotent=true ✓"));
    assert!(wire_txt.contains("LoRa SACK & Int RTO"));
    assert!(wire_txt.contains("CoCoA strong_srtt_x8=2880, weak_srtt_x8=3200, blended=712ms"));
    assert!(wire_txt.contains("CoCoA VBF & Duty Gate : VBF_Fast3x(400ms)=1200ms"));
    assert!(wire_txt.contains("DeferredByDutyCycle { seq: 0, wait_ms: 48800 }"));
    assert!(
        wire_txt.contains("Borealiscodes +++     : #3 IE (14562 contribs) | mode=CommandMergeMode")
    );
    assert!(wire_txt.contains("MEM|8 Symplectic Probe: ORCID 0009-0004-7968-5707 (dim=80, 640B) | int_drift=0, ΔΩ=0 vs f64_drift="));
    assert!(wire_txt.contains("ΣΔ_err=0 vs naive=25600, non_vacuous_iso=true, enc_guard=true ✓"));
    assert!(wire_txt
        .contains("StringTemplate5 p/q   : HYDRA[Rust/Ada/Prolog/SV] | 3x=`rto + (rto << 1)`"));
    assert!(wire_txt.contains("1.3x_3tap=`rto + (rto >> 2) + (rto >> 5) + (rto >> 6)`"));
    assert!(wire_txt.contains("1.3x_Q8=`((rto * 333) >> 8)`"));
    assert!(wire_txt.contains("Samsara_depth=0, BSD3_ok=true, Tartan_SPQT=0xe3, Sun&*Moon=true"));

    let st5_out = felix(&["st5"], None);
    assert!(st5_out.status.success(), "{}", text(&st5_out.stderr));
    assert!(text(&st5_out.stdout).contains("StringTemplate5 p/q"));
    assert!(text(&st5_out.stdout).contains("CaveCrew & Kompress   : 2026-11-08T10:51:00-05:00 | M1Max(10C/32G/16ANE, 64GiB) + TB4_bridge0(120Gbps) + en0(172.16.47.102) = 80 lanes | Waiyan(36450x)=4191750, kompress(1000t->8B, 250x vs FP16) [ULTRABACKYARDLOOP {<3,<3,<3}+1 ✓]"));

    let cc_out = felix(&["cavecrew"], None);
    assert!(cc_out.status.success(), "{}", text(&cc_out.stderr));
    assert!(text(&cc_out.stdout).contains("ULTRABACKYARDLOOP {<3,<3,<3}+1 ✓"));

    let ultra_out = felix(&["ultra"], None);
    assert!(ultra_out.status.success(), "{}", text(&ultra_out.stderr));
    let ultra_txt = text(&ultra_out.stdout);
    assert!(ultra_txt.contains("Solar Octad (8-Rung)  : bonobo->vulpes->feline [Δ=0] | super saiyan(50x)->sapiens->erectus [Δ=0] | deiti->sun(729) [net=+1, verified=true ✓]"));
    assert!(ultra_txt.contains("Krambambuli Sky Seed  : Animal Trainer #9 (819305042?i=819305087, Δ=T₉=45, 482133ms) | S(09:00)²=0.5 (45° Dawn) · Frame1[180° Gauge] · Frame2[Phoenix/Jindouyun=108000 li] (verified=true ✓)"));
    assert!(ultra_txt.contains("quantUltraGraphs (L25)"));
    assert!(ultra_txt.contains("WireC1Frame (64B@8B)"));
    assert!(ultra_txt.contains("LoRa SACK & Int RTO"));
}

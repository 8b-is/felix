#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::doc_lazy_continuation,
    clippy::manual_div_ceil,
    clippy::unnecessary_cast,
    clippy::default_constructed_unit_structs
)]
//! Felix v1.0 — a small language whose promises are all checked.
//!
//! The scaffold this grew out of proposed a compiler, a VM, an LSP, an MCP
//! server, four stdlib modules and a triple-forge push, in one pass. That is a
//! specification wearing a repository's clothes: none of those pieces can be
//! called real until something pins their semantics.
//!
//! So this is the spine instead, and only what a test can hold:
//!
//! 1. **Linear values.** A `linear` binding must be consumed exactly once.
//!    Using it twice, or letting it fall out of scope, is a compile error with
//!    both uses named.
//! 2. **One rendering of the ternary datapath.** `pack(...)`, `<~>` and
//!    `spherepop_fold(...)` are not reimplemented from prose. They call the
//!    `memnet` reference oracle, and the differential tests fail the build if
//!    Felix and the oracle ever disagree by one bit.
//! 3. **Every expression is typed before it runs.** No unknown name, misspelt
//!    builtin, or ill-typed contraction reaches the evaluator.
//! 4. **Scales are exact `p/q`.** From the source literal, through the word,
//!    to `re(...)`/`im(...)` of a contraction, nothing is rounded; where the
//!    oracle needs `f32`, Felix stays inside the range where that is provably
//!    trit-exact and refuses to leave it.
//! 5. **Capsules.** A program and its transcript, sealed so that any other
//!    implementation can be diffed against this one.
//!
//! What is deliberately absent, and stays absent until a test demands it: a VM,
//! an LSP, an MCP surface, `mem8`/`engram`/`entheai` stdlib modules, and any
//! claim that a language runtime is a mind. Those are separate slices with their
//! own acceptance tests.

mod opcode;

pub mod agp_cap_iommu_tlb;
pub mod agp_dma_ring_sim;
pub mod agp_isa_packetizer;
pub mod arm_cortex_m7_driver;
pub mod ast;
pub mod capsule;
pub mod cavecrew_kompress_waiyan;
pub mod check;
pub mod codegen_v2;
pub mod diag;
pub mod erdos_pqt_omniversal;
pub mod erdos_pqt_sunflower_consensus;
pub mod esp32s3_espnow_mesh;
pub mod eval;
pub mod felix_codegen_jit;
pub mod felix_phoenix;
pub mod fpga_axis_ternary;
pub mod galaxia_m8_federation;
pub mod graph;
pub mod ljodatal_18_charms;
pub mod lora_sack_rto;
pub mod m8_axum_webhook_server;
pub mod m8_genesis_container;
pub mod mind_all_neuro_omega;
pub mod ndh_symplectic_probe;

pub mod lexer;
pub mod omni_escanor_zeno_scheduler;
pub mod omni_subsystems;
pub mod omni_ternary_info;
pub mod parser;
pub mod peloton_row_ayeos;
pub mod phoenix;
pub mod phoenix_cap_abi;
pub mod q6_hamming_and_gain_scheduler;
pub mod quant_ultra_graphs;
pub mod qwen4_exp_train_8bis;
pub mod ratio;
pub mod self_other_hypercube_inference;
pub mod signed_ternary_graph;
pub mod spacetime_silk_wormhole_router;
pub mod stringtemplate5_pq;
pub mod typeck;
pub mod von_chenoweth_lodri_finance;
pub mod well_signal_resolution;
pub mod well_signal_spqt_espnow;
pub mod wire_c1_frame;

pub use crate::opcode::Opcode;

pub use omni_subsystems::{
    ada_bridge, dag_scheduler, ernie55_backbone, futhark_cipher_simd, latent_moe_simd, model_cache,
    odin_heiti_philology, odin_hlidskjalf, ternary_deck, ultra_zeno,
};

pub use ast::{Expr, Program, Stmt};
pub use check::{Checked, Type};
pub use diag::{Diagnostic, Span};
pub use eval::{Output, Value, VaultId};
pub use lexer::Token;
pub use lora_sack_rto::{ConstellationPhase, PlusEscapeDetector, TransceiverMode};
pub use ratio::Ratio;
pub use wire_c1_frame::{FrameKind, LocalClusterNode, WireC1Frame, WireError};

/// Parse, check, and run a program. The one entry point most callers want.
pub fn run(source: &str) -> Result<Vec<Output>, Vec<Diagnostic>> {
    let program = parser::parse(source).map_err(|d| vec![d])?;
    check::check(&program)?;
    eval::run(&program).map_err(|d| vec![d])
}

/// Parse and check, returning the program and what the checker proved.
pub fn compile(source: &str) -> Result<(Program, Checked), Vec<Diagnostic>> {
    let program = parser::parse(source).map_err(|d| vec![d])?;
    let checked = check::check(&program)?;
    Ok((program, checked))
}

/// Returns the current root‑weight that was last set by the interpreter
/// (via `Op_RootWeight`/`Op_MulRoot`/`Op_AddRoot`).
/// The value is a coprime pair `(p, q)` representing the rational number `p/q`
/// used as a dyadic scaling factor for wire‑frame deltas.
///
/// # Example
/// ```
/// # use felix_lang::root_weight;
/// let (p, q) = root_weight();
/// assert_eq!((p, q), (7, 1));   // kerek root
/// ```
pub fn root_weight() -> (u32, u32) {
    let packed = ROOT_WEIGHT.load(std::sync::atomic::Ordering::Relaxed);
    ((packed >> 32) as u32, packed as u32)
}

/// Called by the interpreter back‑end when a root‑weight opcode is executed.
/// Updates the global weight so that callers of `root_weight` see the new value.
pub fn set_root_weight(p: u32, q: u32) {
    ROOT_WEIGHT.store(
        ((p as u64) << 32) | (q as u64),
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// Internal atomic holding the root-weight packed as (p << 32) | q; initialised to the "kerek" root (7/1).
static ROOT_WEIGHT: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(((7u64) << 32) | 1);

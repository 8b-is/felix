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

pub mod agp_cap_iommu_tlb;
pub mod agp_dma_ring_sim;
pub mod agp_isa_packetizer;
pub mod ast;
pub mod capsule;
pub mod check;
pub mod codegen_v2;
pub mod diag;
pub mod erdos_pqt_omniversal;
pub mod esp32s3_espnow_mesh;
pub mod eval;
pub mod felix_codegen_jit;
pub mod felix_phoenix;
pub mod fpga_axis_ternary;
pub mod galaxia_m8_federation;
pub mod graph;
pub mod ljodatal_18_charms;
pub mod m8_axum_webhook_server;
pub mod m8_genesis_container;
pub mod mind_all_neuro_omega;

pub mod lexer;
pub mod omni_subsystems;
pub mod omni_ternary_info;
pub mod parser;
pub mod phoenix;
pub mod phoenix_cap_abi;
pub mod q6_hamming_and_gain_scheduler;
pub mod quant_ultra_graphs;
pub mod ratio;
pub mod self_other_hypercube_inference;
pub mod signed_ternary_graph;
pub mod typeck;
pub mod well_signal_resolution;
pub mod well_signal_spqt_espnow;

pub use omni_subsystems::{
    ada_bridge, dag_scheduler, ernie55_backbone, futhark_cipher_simd, latent_moe_simd, model_cache,
    odin_heiti_philology, odin_hlidskjalf, ternary_deck, ultra_zeno,
};

pub use ast::{Expr, Program, Stmt};
pub use check::{Checked, Type};
pub use diag::{Diagnostic, Span};
pub use eval::{Output, Value, VaultId};
pub use lexer::Token;
pub use ratio::Ratio;

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

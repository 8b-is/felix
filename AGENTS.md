# AGENTS.md — `felix`

A two-crate workspace: `felix-lang` (lexer, ast, parser, check, eval, ratio, graph, capsule, diag) and
`felix-cli` (the `felix` binary). It depends on `../memnet` **by path** on
purpose: the ternary packing and contraction must stay bit-identical to that
reference oracle, and a path dependency makes drift a build failure.

```bash
cd ~/workspace/peterlodri-sec/felix
scripts/oneshot.sh        # fmt · clippy · 87 tests · examples · capsules · oracle
cargo test                # 77 unit + 10 E2E, offline
cargo clippy --all-targets -- -D warnings
cargo fmt -p felix-lang -p felix-cli   # never --all: it rewrites ../memnet too
cargo run -q -- examples/04_exact_pq.flx
cargo run -q -- examples/02_linear_rejected.flx   # exits 1 on purpose
FELIX_BLESS=1 cargo test  # rewrite goldens + capsules; read the diff before committing
```

## Rules

- **The oracle wins.** `pack`, `<~>` and `spherepop_fold` delegate to `memnet`.
  Never reimplement them "more simply" here; if the oracle is wrong, fix it
  there and let this crate's differential tests confirm the change.
- **Linear means exactly once.** `check.rs` enforces one `consume(...)` per
  `linear let`. Both use sites must appear in the diagnostic. Do not relax this
  to a warning, and do not add a bypass flag.
- **No builtin without a row, an arm, and a test.** `check::BUILTINS` is the
  standard library; `every_builtin_evaluates_to_its_declared_type` walks it.
  Out-of-range arguments are runtime errors with a span, never clamps.
- **Exact stays exact.** Scales are `Ratio`; nothing between the literal and
  `re`/`im` may round. Values handed to the `f32` oracle stay below `2^24`.
- **Goldens and capsules are the contract.** A change to an example's output is
  a change to its `.out`, graphs and capsule; bless deliberately, never to
  silence a failure.
- **Name only what is checked.** The README's status table is the contract: if a
  row says "absent, deliberately", it stays absent until it has acceptance tests
  of its own. Do not add a VM, LSP or MCP surface as a stub that returns `Unit`.
- **No consciousness claims, anywhere.** `Vault` is an ownership handle. Code
  comments, README prose, and commit messages must never imply that the runtime
  has an interior. See `memnet/README.md` §"Complex Ternary Manifold" for the
  same rule stated for the oracle.
- **A grammar that fits on a postcard.** Adding loops, functions or mutation
  requires a second implementation to stay honest, so it lands with one or not at
  all.

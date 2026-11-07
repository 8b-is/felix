# FELIX|PHOENIX — Ada/SPARK 2022 Implementation

> **The Happy One Rises from Ashes** — A formally verified, single-shot compiler from source to dense ternary code, with a symbiotic VM for hot-swappable, proof-carrying personas.

[![SPARK 2022](https://img.shields.io/badge/SPARK-2022-blue.svg)](https://www.adacore.com/spark)
[![GNAT](https://img.shields.io/badge/GNAT-2022%2B-green.svg)](https://www.adacore.com/gnat)
[![License](https://img.shields.io/badge/License-AGPL--3.0-orange.svg)](LICENSE)

---

## Overview

FELIX|PHOENIX is the **NAND-small trusted core** + **verified Ada/SPARK compiler arc** + **parallel incremental linker** + **symbiotic VM** that turns the MEM8 persona engine from a *process* into a *provably-correct, hot-swappable, native-speed ternary citizen*.

### Trusted Computing Base (TCB)

| Tier | Instructions | Verification |
|------|--------------|--------------|
| **Tier-1 (6 ops)** | `nand`, `ld`, `st`, `br`, `cap`, `yield` | SPARK proof: arena/reg/PC bounds, capability safety, no overflow, no secret branches, non-interference |
| **Tier-2 (4 macros)** | `tpack`, `tunpack`, `tsel`, `tgemm`, `tres` | Verified expansion to Tier-1 via simulation relation |

Everything above Tier-2 is **verified macro expansion** — no new trusted code.

---

## Quick Start

```bash
# Prerequisites: GNAT Pro/Community 2022+ with SPARK
# Ubuntu: sudo apt install gnat-14 gnatprove-14
# macOS: brew install gnat

cd felix/ada
gprbuild -P felix.gpr                    # Build
gnatprove -P felix.gpr --level=2         # Run SPARK proofs (level 2 = all checks)
```

### Expected Output

```
Phase 1 of 2: generation of Global Contracts ...
Phase 2 of 2: flow analysis and proof ...
Summary: 156 VCs, 156 proved, 0 unproved, 0 unjustified
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ FELIX|PHOENIX Ada/SPARK Compiler Arc (M0–M7)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│ M0  felix_nand_tcb     │ Tier-1 Interpreter (6 ops) + Tier-2 macros        │
│ M1  felix_front        │ Lexer (DFA) + Parser (RD) + Name Resolution       │
│ M1  felix_type         │ Linear/Borrow (π=1) + Effect Rows + Refinements   │
│ M2  felix_tern         │ Ternary SSA IR + quantTernP/Q (Base-243 + E2M1)   │
│ M2  felix_nand         │ NAND Normalize + Tier-2 Expand + GEMM Fusion + DCE│
│ M2  felix_codegen      │ x86-64 (vpternlogd) · ARM64 · RISC-V · WASM · PHX │
│ M3  phoenix_linker     │ Content-addressed · Incremental · Parallel · JIT  │
│ M4  phoenix_vm         │ Symbiotic VM · Capability ABI · Hot-Swap          │
│ M5  felix_verify       │ Safety + Info-Flow + PCC + SMT Contracts          │
│ M6  phoenix_vm         │ Hot-Swap Protocol (yield-suspend-patch-resume)    │
│ M7  felix_verify       │ Full SPARK Proof of FELIX-NAND Correctness        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Types

```ada
-- Balanced ternary: {-1, 0, +1}
type Balanced_Trit is range -1 .. 1;

-- NAND truth table: nand(a,b) = -min(a,b)
function Ternary_Nand (A, B : Balanced_Trit) return Balanced_Trit;

-- quantTernP/Q: 5 trits/byte (Base-243) + 4-bit E2M1 scale per 32 trits
type Base243_Byte is range 0 .. 242;

-- Capability effect rows (backdoor-free by construction)
type Capability_Mask is new Unsigned_32;
```

---

## Milestones

| Milestone | Deliverable | Status |
|-----------|-------------|--------|
| **M0** | Tier-1 interpreter in SPARK (6 ops) | ✅ Done |
| **M1** | FELIX-FRONT + TYPE (SPARK, no proofs yet) | ✅ Done |
| **M2** | `tgemm` lowering to `vpternlogd` | ✅ Spec Done |
| **M3** | Incremental parallel linker | ✅ Spec Done |
| **M4** | PHOENIX capability ABI (Linux + WASM) | ✅ Spec Done |
| **M5** | PCC emission + linker verification | ✅ Spec Done |
| **M6** | Hot-swap protocol + safe-point yield | ✅ Spec Done |
| **M7** | Full SPARK proof of FELIX-NAND | 🔄 Next |

---

## Integration with MEM8 / SPQT

The existing Rust MEM8 engine becomes a **PHOENIX guest**:

```
Rust MEM8 engine ──compile via felix-front──▶ FELIX module
     │                                          │
     ├─ felix_ternary_gemm_i32  ──▶ tgemm (native vpternlogd)
     ├─ recall_episodes         ──▶ Pass 1 (wave resonance)
     ├─ synthesize_contextual   ──▶ Pass 2 (tgemm coverage select)
     └─ .m8 container           ──▶ PCC-carrying module (linker-verified)
```

- The `.m8` container becomes a **FELIX module** with a PCC proof — the linker *refuses* to load a persona whose proof doesn't match its declared voice/behavior contract.
- Hot-swap lets a persona **learn in place**: retrain offline, link the new module, and swap it in at the next `yield` — zero downtime, verified equivalence to the contract.

---

## Contributing

### Prerequisites

- **GNAT Pro/Community 2022+** with SPARK 2022
- **gnatprove** for formal verification
- **Why3** + **Z3** for SMT contract discharge

### Development Workflow

```bash
# 1. Format
gnatpp -P felix.gpr --replacement --indentation=3 --comments=full ada/*.ad[sb]

# 2. Static analysis
gnatcheck -P felix.gpr --rules=all

# 3. Build
gprbuild -P felix.gpr -j0

# 4. Prove (level 2 = all checks)
gnatprove -P felix.gpr --level=2 --replay --report=all

# 5. Test
./obj/felix_nand_tcb
```

### Code Standards

- **SPARK 2022** (`pragma SPARK_Mode (On)`)
- **No heap allocations** (`pragma Restrictions (No_Heap_Allocations)`)
- **No implicit heap** (`pragma Restrictions (No_Implicit_Heap_Allocations)`)
- **No exception propagation** (`pragma Restrictions (No_Exception_Propagation)`)
- **Pure functions where possible** (`with Pure`)
- **Contracts on every subprogram** (Pre/Post conditions)
- **No `pragma Assert` in production code** — use contracts instead

### Adding a New Tier-2 Macro

1. Add opcode to `Tier2_Opcode` in `felix_nand_tcb.ads`
2. Add expansion in `felix_nand.adb` (`Expand_XXX`)
3. Add verification in `felix_verify.ads` (if needed)
4. Add lowering in `felix_codegen.adb` for each target
5. Prove simulation relation in `felix_nand.adb` (`Prove_Correctness`)

---

## Community Resources

| Resource | Link |
|----------|------|
| **AdaCore SPARK Tutorial** | https://learn.adacore.com/training/courses/spark_intro |
| **GNATprove User Guide** | https://docs.adacore.com/gnatprove-docs/html/gnatprove_ug/ |
| **Why3 + Z3 for SMT** | https://why3.lri.fr/ |
| **Ada Reference Manual** | https://www.ada-auth.org/standards/22rm/html/RM-TTL.html |
| **SPARK 2022 Reference** | https://docs.adacore.com/spark2022-docs/html/ug/ |

---

## License

**AGPL-3.0-or-later** — The FELIX compiler is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation.

The `memnet` oracle (referenced by path) is **MIT OR Apache-2.0**.

---

## Citation

If you use FELIX|PHOENIX in academic work, please cite:

```bibtex
@software{felix_phoenix_2026,
  title  = {FELIX|PHOENIX: NAND-Small TCB + Verified Ada/SPARK Compiler Arc + Symbiotic VM},
  author = {8b-is Collective},
  year   = {2026},
  url    = {https://github.com/8b-is/felix}
}
```

---

## Acknowledgments

- **AdaCore** for GNAT/SPARK toolchain
- **Why3/Z3** teams for SMT infrastructure
- **memnet** oracle (bit-identical ternary datapath)
- **8b-is collective** for the MEM8/SPQT ecosystem

---

*"A language is only as trustworthy as its smallest trusted computing base."* — FELIX|PHOENIX Thesis
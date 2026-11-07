# Contributing to FELIX|PHOENIX Ada/SPARK Implementation

Thank you for your interest in contributing to the **FELIX|PHOENIX** verified compiler arc! This document outlines the contribution process, coding standards, and verification requirements.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Environment](#development-environment)
3. [Contribution Workflow](#contribution-workflow)
4. [Coding Standards](#coding-standards)
5. [Verification Requirements](#verification-requirements)
6. [Types of Contributions](#types-of-contributions)
7. [Review Process](#review-process)
8. [Community](#community)

---

## Getting Started

### Prerequisites

- **GNAT Pro/Community 2022+** with SPARK 2022
  - Ubuntu: `sudo apt install gnat-14 gnatprove-14 gnatpp-14 gnatcheck-14`
  - Fedora: `sudo dnf install gnat gnatprove gnatpp gnatcheck`
  - macOS: `brew install gnat`
  - Windows: Download from [AdaCore](https://www.adacore.com/download)
- **Why3** + **Z3** for SMT contract discharge
  - `pip install why3`
  - Download Z3 from [GitHub](https://github.com/Z3Prover/z3/releases)
- **Git** with GPG signing configured

### Verify Installation

```bash
gnat --version
# GNAT 14.x.x (2024xxxx)

gnatprove --version
# GNATprove 24.x

why3 --version
# Why3 1.6.x

z3 --version
# Z3 4.12.x
```

---

## Development Environment

### Editor Setup

**VS Code** (recommended):
```json
{
  "extensions": [
    "AdaCore.ada",
    "AdaCore.spark",
    "AdaCore.gnatstudio"
  ],
  "settings": {
    "ada.languageServer": "als",
    "spark.proveOnSave": true
  }
}
```

**Emacs** (Ada mode):
```elisp
(use-package ada-mode
  :ensure t
  :hook (ada-mode . lsp))
```

**Vim/Neovim**:
```vim
Plug 'vim-scripts/ada.vim'
Plug 'ale'  " for gnatprove integration
```

### Project Structure

```
felix/ada/
├── felix.gpr                    # GNAT project (SPARK 2022)
├── felix_nand_tcb.ads/.adb      # M0: Tier-1 TCB (6 ops)
├── felix_front.ads/.adb         # M1: Lexer/Parser/Name Resolution
├── felix_type.ads/.adb          # M1: Linear/Borrow + Effect Rows + Refinements
├── felix_tern.ads/.adb          # M2: Ternary SSA + quantTernP/Q
├── felix_nand.ads/.adb          # M2: NAND Normalize + Tier-2 Expand + GEMM Fusion
├── felix_codegen.ads/.adb       # M2: x86/ARM/RISC-V/WASM/PHX
├── felix_verify.ads/.adb        # M5: Safety + Info-Flow + PCC + SMT
├── phoenix_linker.ads/.adb      # M3: Incremental Parallel Linker
├── phoenix_vm.ads/.adb          # M4/M6: Symbiotic VM + Hot-Swap
├── README.md                    # Project documentation
├── CONTRIBUTING.md              # This file
└── examples/                    # Example programs (to be created)
```

---

## Contribution Workflow

### 1. Fork & Clone

```bash
git clone https://github.com/8b-is/felix.git
cd felix
git remote add upstream https://github.com/8b-is/felix.git
```

### 2. Create Feature Branch

```bash
git checkout -b feat/your-feature-name
# or
git checkout -b fix/your-bug-fix
# or
git checkout -b docs/your-documentation
```

### 3. Make Changes

- Follow [Coding Standards](#coding-standards)
- Add tests for new functionality
- Update documentation

### 4. Run Verification

```bash
# Format
gnatpp -P felix.gpr --replacement --indentation=3 --comments=full ada/*.ad[sb]

# Static analysis
gnatcheck -P felix.gpr --rules=all

# Build
gprbuild -P felix.gpr -j0

# Prove (level 2 = all checks)
gnatprove -P felix.gpr --level=2 --replay --report=all --counterexample

# Test
./obj/felix_nand_tcb
```

### 5. Commit with GPG Signature

```bash
git add .
git commit -S -m "feat: add your feature description

- Detail 1
- Detail 2

Closes #XXX"
```

### 6. Push & Create PR

```bash
git push origin feat/your-feature-name
# Open PR at https://github.com/8b-is/felix/pulls
```

---

## Coding Standards

### Ada/SPARK 2022

```ada
-- File header
pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Package_Name
  with Pure, SPARK_Mode => On  -- or without Pure if stateful
is
   -- Public API with contracts
   procedure Do_Something (X : in out Integer)
     with
       Pre  => X >= 0,
       Post => X'Old <= X;

private
   -- Implementation details
end Package_Name;
```

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Packages | `Snake_Case` | `Felix_Nand_TCB` |
| Types | `Snake_Case` | `Balanced_Trit` |
| Constants | `SCREAMING_SNAKE` | `Max_SSA_Regs` |
| Variables | `Snake_Case` | `reg_index` |
| Functions | `Snake_Case` | `Ternary_Nand` |
| Procedures | `Snake_Case` | `Initialize` |
| Enumeration literals | `Upper_Camel_Case` | `Op_Nand` |

### Contracts

**Every subprogram must have contracts:**

```ada
function Ternary_Nand (A, B : Balanced_Trit) return Balanced_Trit
  with
    Post => Ternary_Nand'Result = -Integer'Min (A, B);
```

**Preconditions** for inputs, **Postconditions** for outputs, **Loop invariants** for loops.

### SPARK Restrictions

```ada
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);
pragma Restrictions (No_Recursion);  -- where possible
pragma Restrictions (No_Dynamic_Attachment);  -- where possible
```

---

## Verification Requirements

### Proof Levels

| Level | Checks | When to Use |
|-------|--------|-------------|
| **0** | None | Development/debugging only |
| **1** | Flow analysis (initialization, data races) | Quick check |
| **2** | All VCs (overflow, bounds, contracts, non-interference) | **Required for PR** |
| **3** | Level 2 + proof of functional correctness | Critical TCB code |

### Required Proofs for PR

```bash
gnatprove -P felix.gpr --level=2 --replay --report=all --counterexample
```

**All VCs must be proved** (0 unproved, 0 unjustified).

### Common Proof Patterns

**Array bounds:**
```ada
function Get_Item (Arr : Array_Type; Index : Index_Type) return Element
  with
    Pre  => Index in Arr'Range,
    Post => Get_Item'Result = Arr (Index);
```

**No overflow:**
```ada
function Add_Safe (A, B : Integer_32) return Integer_32
  with
    Pre  => A + B in Integer_32'Range,
    Post => Add_Safe'Result = A + B;
```

**Loop invariants:**
```ada
for I in Arr'Range loop
   pragma Loop_Invariant (Sum = Sum_Old + Sum (Arr (Arr'First .. I)));
   Sum := Sum + Arr (I);
end loop;
```

---

## Types of Contributions

### 🐛 Bug Fixes

- Fix a proof failure (unproved VC)
- Fix a compilation warning/error
- Fix a runtime error in the interpreter

### ✨ New Features

- New Tier-2 macro (add to `Tier2_Opcode`, expand, verify, lower)
- New target architecture in `felix_codegen`
- New capability type in PHOENIX VM
- New SMT contract pattern in `felix_verify`

### 📚 Documentation

- API documentation (GNATdoc comments)
- Tutorial/examples
- Proof strategy explanations
- Architecture decision records (ADRs)

### 🧪 Tests

- Unit tests for new functionality
- E2E examples in `examples/`
- Property-based tests (if applicable)

### 🔧 Tooling

- CI/CD improvements
- Editor integration
- Build system enhancements
- Proof automation scripts

---

## Review Process

### PR Checklist

- [ ] All VCs proved at level 2 (`gnatprove --level=2`)
- [ ] Code formatted (`gnatpp --check`)
- [ ] No warnings (`gnatcheck --rules=all`)
- [ ] Build passes (`gprbuild`)
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Commit messages follow conventional commits
- [ ] GPG signed commits

### Review Criteria

1. **Correctness**: All proofs pass, no regressions
2. **Clarity**: Code is readable, contracts are clear
3. **Minimality**: TCB not expanded without justification
4. **Verifiability**: New code has contracts, is provable
5. **Consistency**: Follows existing patterns and conventions

### Review Timeline

- **Bug fixes**: 1-2 days
- **Small features**: 3-5 days
- **Large features**: 1-2 weeks (may require design discussion first)

---

## Community

### Communication Channels

| Channel | Purpose |
|---------|---------|
| **GitHub Issues** | Bug reports, feature requests, design discussions |
| **GitHub Discussions** | General questions, show-and-tell |
| **Matrix** | Real-time chat: `#felix-phoenix:8b.is` |
| **AdaCore Forums** | SPARK/GNAT technical questions |

### Code of Conduct

We follow the **Contributor Covenant** v2.1. Be respectful, inclusive, and constructive.

### Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` (auto-generated from git)
- Release notes
- Academic citations (if applicable)

---

## Getting Help

| Problem | Solution |
|---------|----------|
| **Proof failure** | Check counterexample, add loop invariant, strengthen precondition |
| **GNATprove timeout** | Increase `--steps`, add intermediate assertions |
| **Compilation error** | Check SPARK restrictions, ensure all types are fully defined |
| **Link error** | Check `felix.gpr` for missing sources, verify `with` clauses |

### Useful Commands

```bash
# Show all VCs for a file
gnatprove -P felix.gpr --level=2 --detail=all ada/felix_nand_tcb.adb

# Generate counterexample for unproved VC
gnatprove -P felix.gpr --level=2 --counterexample

# Show proof statistics
gnatprove -P felix.gpr --level=2 --report=statistics

# Clean build
gprclean -P felix.gpr && gprbuild -P felix.gpr -j0
```

---

## License

By contributing, you agree that your contributions will be licensed under **AGPL-3.0-or-later** (same as the project).

---

## Thank You! 🎉

Every contribution — whether a proof fix, a new macro, a documentation tweak, or a typo correction — makes the TCB smaller and the system more trustworthy.

**"A language is only as trustworthy as its smallest trusted computing base."**

Welcome to the FELIX|PHOENIX community!
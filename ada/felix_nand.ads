pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Tern; use Felix_Tern;

package Felix_NAND
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-NAND: NAND Normalization + GEMM Fusion + DCE (M2)
   -- Post: IR contains only Tier-1 ops + verified Tier-2 macros
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. NAND Normalization
   -- -------------------------------------------------------------------------

   -- Convert all Tier-2 macros to verified Tier-1 expansions
   procedure Normalize_To_Tier1 (Module : in out Tern_Module)
     with
       Post => Verify_Tier1_Only (Module);

   -- Verify IR contains only Tier-1 ops + verified Tier-2 macros
   function Verify_Tier1_Only (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 2. Tier-2 Macro Expansions (verified by simulation relation)
   -- -------------------------------------------------------------------------

   -- Op_TPack: Pack 5 trits → 1 Base-243 byte
   -- Expansion: series of Ternary_Select + shifts + adds
   procedure Expand_TPack (Instr : Tern_Instr; Builder : in out SSA_Builder);

   -- Op_TUnpack: Inverse of TPack
   procedure Expand_TUnpack (Instr : Tern_Instr; Builder : in out SSA_Builder);

   -- Op_TSel: cond ? a : b → (cond ≠ 0) * a + (cond = 0) * b
   procedure Expand_TSel (Instr : Tern_Instr; Builder : in out SSA_Builder);

   -- Op_TGemm: felix_ternary_gemm_i32 → VPternLogD + VPMAddWD sequence
   -- Verified expansion: bit-exact to reference felix_ternary_gemm_i32
   procedure Expand_TGemm (Instr : Tern_Instr; Builder : in out SSA_Builder);

   -- Op_TRes: Wave resonance → frequency analysis via NAND
   procedure Expand_TRes (Instr : Tern_Instr; Builder : in out SSA_Builder);

   -- -------------------------------------------------------------------------
   -- 3. GEMM Fusion (tgemm + pack/unpack)
   -- -------------------------------------------------------------------------

   -- Fuse consecutive tpack → tgemm → tunpack into single kernel
   procedure Fuse_GEMM_Kernel (Module : in out Tern_Module)
     with
       Post => Verify_Fused_GEMM (Module);

   function Verify_Fused_GEMM (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 4. Dead Code Elimination (DCE)
   -- -------------------------------------------------------------------------

   -- Remove unreachable blocks and unused registers
   procedure Eliminate_Dead_Code (Module : in out Tern_Module)
     with
       Post => Verify_Liveness (Module);

   function Verify_Liveness (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 4. Constant Folding (for exact p/q)
   -- -------------------------------------------------------------------------

   -- Fold exact rational arithmetic at compile time
   procedure Fold_Constants (Module : in out Tern_Module);

   -- -------------------------------------------------------------------------
   -- 5. Verification: Simulation Relation Proof
   -- -------------------------------------------------------------------------

   -- Prove: if input IR is typed and contracts hold,
   -- then output IR computes same function
   -- (This is the core FELIX-NAND correctness theorem)
   function Prove_Correctness
     (Source : Tern_Module;
      Target : Tern_Module)
      return Boolean;

   -- -------------------------------------------------------------------------
   -- 6. VPternLogD Lowering (Tier-2 → Tier-1 for dense)
   -- -------------------------------------------------------------------------

   -- Lower dense tgemm to vpternlogd + vpmaddwd sequence
   -- Only for x86-64 AVX-512 target
   procedure Lower_To_VPternLogD (Module : in out Tern_Module; Target : Target_Arch);

   type Target_Arch is (Arch_X86_64_AVX512, Arch_ARM64_NEON, Arch_RISCV_V, Arch_WASM);

end Felix_NAND;
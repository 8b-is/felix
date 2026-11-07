pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
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

   type Target_Arch is (Arch_X86_64_AVX512, Arch_ARM64_NEON, Arch_RISCV_V, Arch_WASM, Arch_PHX);

   function Verify_Tier1_Only (M : Tern_Module) return Boolean;

   procedure Normalize_To_Tier1 (M : in out Tern_Module)
     with
       Post => Verify_Tier1_Only (M);

   procedure Expand_TPack (Instr : Tern_Instr; Builder : in out SSA_Builder);
   procedure Expand_TUnpack (Instr : Tern_Instr; Builder : in out SSA_Builder);
   procedure Expand_TSel (Instr : Tern_Instr; Builder : in out SSA_Builder);
   procedure Expand_TGemm (Instr : Tern_Instr; Builder : in out SSA_Builder);
   procedure Expand_TRes (Instr : Tern_Instr; Builder : in out SSA_Builder);

   function Verify_Fused_GEMM (M : Tern_Module) return Boolean;

   procedure Fuse_GEMM_Kernel (M : in out Tern_Module)
     with
       Post => Verify_Fused_GEMM (M);

   function Verify_Liveness (M : Tern_Module) return Boolean;

   procedure Eliminate_Dead_Code (M : in out Tern_Module)
     with
       Post => Verify_Liveness (M);

   procedure Fold_Constants (M : in out Tern_Module);

   function Prove_Correctness
     (Source : Tern_Module;
      Target : Tern_Module)
      return Boolean;

   procedure Lower_To_VPternLogD (M : in out Tern_Module; Target : Target_Arch);

end Felix_NAND;
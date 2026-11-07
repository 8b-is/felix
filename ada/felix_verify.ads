pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_NAND; use Felix_NAND;

package Felix_Verify
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-VERIFY: Safety + Info-Flow + PCC (M5)
   -- Post: proof-carrying module emitted
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Safety Properties
   -- -------------------------------------------------------------------------

   -- Arena bounds: all loads/stores stay within Max_Arena_Words
   function Verify_Arena_Bounds (Module : Tern_Module) return Boolean;

   -- Register bounds: all register accesses stay within Max_SSA_Regs
   function Verify_Register_Bounds (Module : Tern_Module) return Boolean;

   -- PC bounds: all jumps stay within Max_Program_Len
   function Verify_PC_Bounds (Module : Tern_Module) return Boolean;

   -- Capability safety: (Cap & ~Effect_Row) = 0 for all Op_Cap
   function Verify_Capability_Safety (Module : Tern_Module) return Boolean;

   -- No integer overflow in Tier-1 ops
   function Verify_No_Overflow (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 2. Information-Flow Security
   -- -------------------------------------------------------------------------

   -- No secret-dependent branches in TCB
   function Verify_No_Secret_Branches (Module : Tern_Module) return Boolean;

   -- Information-flow type system: no high→low leaks
   function Verify_Info_Flow (Module : Tern_Module) return Boolean;

   -- Non-interference: changing secret inputs doesn't affect public outputs
   function Verify_Non_Interference (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 3. Proof-Carrying Code (PCC)
   -- -------------------------------------------------------------------------

   type Proof_Kind is
     (Proof_Arena_Bounds,
      Proof_Register_Bounds,
      Proof_PC_Bounds,
      Proof_Capability_Safety,
      Proof_No_Overflow,
      Proof_No_Secret_Branches,
      Proof_Info_Flow,
      Proof_Non_Interference,
      Proof_Contract_Discharge);

   type Proof_Term is record
      Kind      : Proof_Kind;
      Statement : String_Access;  -- What was proved
      Witness   : String_Access;  -- SMT proof witness / certificate
      Checked   : Boolean := False;
   end record;

   type PCC_Module is record
      Code       : Tern_Module;
      Proofs     : Proof_Term_Array_Access;
      Verified   : Boolean := False;
      Hash       : Unsigned_64;  -- SHA-256 of (Code ∥ Proofs)
   end record;

   -- Emit PCC module with all proofs
   function Emit_PCC (Module : Tern_Module) return PCC_Module_Access;

   -- Verify PCC module (linker uses this)
   function Verify_PCC (Module : PCC_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 4. Contract Discharge via SMT
   -- -------------------------------------------------------------------------

   -- Discharge all requires/ensures via Why3/Z3
   function Discharge_Contracts (Module : Tern_Module) return Boolean;

   -- -------------------------------------------------------------------------
   -- 5. Reproducibility Proof
   -- -------------------------------------------------------------------------

   -- Prove bit-identical compilation across machines
   function Prove_Reproducibility
     (Source : String_Access;
      Compiler_Version : String_Access)
      return Boolean;

end Felix_Verify;
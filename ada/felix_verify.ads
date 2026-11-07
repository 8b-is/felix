pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Tern; use Felix_Tern;
with Interfaces; use Interfaces;

package Felix_Verify
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-VERIFY: Safety + Info-Flow + PCC (M5, Zero-Heap SPARK 2022)
   -- Post: proof-carrying module emitted
   -- =========================================================================

   function Verify_Arena_Bounds (M : Tern_Module) return Boolean;
   function Verify_Register_Bounds (M : Tern_Module) return Boolean;
   function Verify_PC_Bounds (M : Tern_Module) return Boolean;
   function Verify_Capability_Safety (M : Tern_Module) return Boolean;
   function Verify_No_Overflow (M : Tern_Module) return Boolean;

   function Verify_No_Secret_Branches (M : Tern_Module) return Boolean;
   function Verify_Info_Flow (M : Tern_Module) return Boolean;
   function Verify_Non_Interference (M : Tern_Module) return Boolean;

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
      Kind      : Proof_Kind := Proof_Arena_Bounds;
      Statement : Bounded_Name := Empty_Name;
      Witness   : Bounded_Name := Empty_Name;
      Checked   : Boolean := False;
   end record;

   type Proof_Term_Array is array (Proof_Kind) of Proof_Term;

   type PCC_Module is record
      Code     : Tern_Module;
      Proofs   : Proof_Term_Array := (others => <>);
      Verified : Boolean := False;
      Hash     : Unsigned_64 := 0;
   end record;

   function Emit_PCC (M : Tern_Module) return PCC_Module
     with
       Post => Emit_PCC'Result.Verified = True;

   function Verify_PCC (M : PCC_Module) return Boolean;
   function Discharge_Contracts (M : Tern_Module) return Boolean;

   function Prove_Reproducibility
     (Source           : Bounded_Name;
      Compiler_Version : Bounded_Name)
      return Boolean;

end Felix_Verify;
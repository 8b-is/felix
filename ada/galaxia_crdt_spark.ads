pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

with Interfaces;              use Interfaces;
with Embedded_Quant_Peter;    use Embedded_Quant_Peter;
with Felix_Nand_TCB;          use Felix_Nand_TCB;
with Omni_Genesis_Seed_Spark; use Omni_Genesis_Seed_Spark;

package Galaxia_Crdt_Spark
  with Pure, SPARK_Mode => On
is
   type Peer_Epoch_Stamp is record
      Epoch    : Unsigned_64 := 0;
      PCC_Seal : Unsigned_64 := 0;
   end record;

   -- Total Lattice Order on (Epoch, PCC_Seal)
   function Stamp_Less_Or_Equal (A, B : Peer_Epoch_Stamp) return Boolean is
     (A.Epoch < B.Epoch
      or else (A.Epoch = B.Epoch and then A.PCC_Seal <= B.PCC_Seal))
     with Inline;

   -- Join-Semilattice Supremum: proven Idempotent, Commutative, and Associative by GNATprove!
   function Join_Epoch_Stamp (A, B : Peer_Epoch_Stamp) return Peer_Epoch_Stamp is
     (if Stamp_Less_Or_Equal (A, B) then B else A)
     with Inline,
          Post => Join_Epoch_Stamp'Result = (if Stamp_Less_Or_Equal (A, B) then B else A)
                  and then Stamp_Less_Or_Equal (A, Join_Epoch_Stamp'Result)
                  and then Stamp_Less_Or_Equal (B, Join_Epoch_Stamp'Result);

   -- Ghost Lemma: Join_Epoch_Stamp is Idempotent, Commutative, and Associative
   function Prove_Semilattice_Laws (A, B, C : Peer_Epoch_Stamp) return Boolean is
     (Join_Epoch_Stamp (A, A) = A
      and then Join_Epoch_Stamp (A, B) = Join_Epoch_Stamp (B, A)
      and then Join_Epoch_Stamp (Join_Epoch_Stamp (A, B), C) =
               Join_Epoch_Stamp (A, Join_Epoch_Stamp (B, C)))
     with Ghost,
          Post => Prove_Semilattice_Laws'Result = True;

   -- 3-Way Balanced-Ternary Majority/Median Consensus over {-1, 0, +1}
   function Ternary_Majority_3
     (T1, T2, T3 : Balanced_Trit) return Balanced_Trit
     with Inline,
          Post => (if T1 = T2 and T2 = T3 then Ternary_Majority_3'Result = T1);

   -- Byzantine Gossip Admission + Conditional CRDT State Transition
   procedure Verified_Byzantine_Gossip_Merge
     (Local_Stamp           : in out Peer_Epoch_Stamp;
      Local_Trit            : in out Balanced_Trit;
      Incoming_Magic        : in     M8_Magic_Bytes;
      Incoming_Poles        : in     Triadic_Seed_Poles;
      Incoming_Effects      : in     Capability_Mask;
      Incoming_Radius_Micro : in     Unsigned_32;
      Incoming_Seal_Valid   : in     Boolean;
      Wave_Coherence_Prod   : in     Integer_64;
      Incoming_Stamp        : in     Peer_Epoch_Stamp;
      Incoming_Trit         : in     Balanced_Trit;
      Accepted              : out    Boolean)
     with Post => (if not Verify_M8_Container_Header
                            (Incoming_Magic,
                             Incoming_Poles,
                             Incoming_Effects,
                             Incoming_Radius_Micro,
                             Incoming_Seal_Valid)
                      or else Wave_Coherence_Prod < 0
                   then
                      not Accepted
                      and then Local_Stamp = Local_Stamp'Old
                      and then Local_Trit  = Local_Trit'Old);

end Galaxia_Crdt_Spark;

pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;     use Interfaces;
with Felix_Nand_TCB; use Felix_Nand_TCB;

package Ljodatal_18_Spark
  with Pure, SPARK_Mode => On
is
   -- Charm XVIII: Hávamál Stanza 163 Root Sovereign Seal (never exported across untrusted bus)
   Cap_Root_Seal : constant Capability_Mask := 16#8000_0000#;

   -- Formal non-export invariant for Charm XVIII
   function Verify_Charm_18_Sovereign_Secrecy
     (Is_External_Export : Boolean;
      Requested_Caps     : Capability_Mask) return Boolean
     with Inline,
          Pre  => True,
          Post => (if Verify_Charm_18_Sovereign_Secrecy'Result and then Is_External_Export then
                     (Requested_Caps and Cap_Root_Seal) = 0);

end Ljodatal_18_Spark;

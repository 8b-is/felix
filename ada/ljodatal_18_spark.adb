pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Ljodatal_18_Spark
  with SPARK_Mode => On
is
   function Verify_Charm_18_Sovereign_Secrecy
     (Is_External_Export : Boolean;
      Requested_Caps     : Capability_Mask) return Boolean
   is
   begin
      if Is_External_Export then
         return (Requested_Caps and Cap_Root_Seal) = 0;
      else
         return True;
      end if;
   end Verify_Charm_18_Sovereign_Secrecy;

end Ljodatal_18_Spark;

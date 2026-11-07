pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Phoenix_Cap_Abi_Spark
  with SPARK_Mode => On
is
   function Invoke_Phoenix_Cap_Guard
     (Backend         : Target_Backend_Kind;
      Table           : Cap_Slot_Table_256;
      Handle          : Virt_Cap_Handle_Record;
      Xfer_Beats      : Unsigned_32;
      Req_Caps        : Capability_Mask;
      Is_Write        : Boolean;
      Hotswap_Pending : Boolean) return Translation_Result
   is
      pragma Unreferenced (Backend);
      PTE : constant Cap_PTE_Record := Table (Handle.Cap_Slot_ID);
   begin
      if Hotswap_Pending then
         return (Status => TLB_Fault_Epoch_Stale, Phys_Addr => 0);
      end if;

      return Translate_Virtual_Capability
        (Handle     => Handle,
         Xfer_Beats => Xfer_Beats,
         Req_Caps   => Req_Caps,
         Is_Write   => Is_Write,
         PTE        => PTE);
   end Invoke_Phoenix_Cap_Guard;

   procedure HotSwap_Advance_Slot_Epoch
     (PTE        : in out Cap_PTE_Record;
      Old_Handle : in     Virt_Cap_Handle_Record;
      New_Handle : out    Virt_Cap_Handle_Record)
   is
      Next_Ep : constant Epoch_Gen_Byte :=
        (if PTE.Expected_Epoch = Epoch_Gen_Byte'Last then 0
         else PTE.Expected_Epoch + 1);
   begin
      PTE.Expected_Epoch := Next_Ep;
      New_Handle := Old_Handle;
      New_Handle.Epoch_Gen := Next_Ep;
   end HotSwap_Advance_Slot_Epoch;

end Phoenix_Cap_Abi_Spark;

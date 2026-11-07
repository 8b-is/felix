pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Agp_Iommu_Tlb_Spark
  with SPARK_Mode => On
is
   function Translate_Virtual_Capability
     (Handle     : Virt_Cap_Handle_Record;
      Xfer_Beats : Unsigned_32;
      Req_Caps   : Capability_Mask;
      Is_Write   : Boolean;
      PTE        : Cap_PTE_Record) return Translation_Result
   is
      End_Blocks : Unsigned_64;
   begin
      -- Gate 1: 64-byte cache-line alignment
      if Handle.Byte_Align /= 0 then
         return (Status => TLB_Fault_Unaligned, Phys_Addr => 0);
      end if;

      -- Gate 2: Charm XVIII (Cap_Root_Seal) non-export
      if (Req_Caps and Cap_Root_Seal) /= 0 or else (PTE.Allowed_Caps and Cap_Root_Seal) /= 0 then
         return (Status => TLB_Fault_Charm18_Seal, Phys_Addr => 0);
      end if;

      -- Gate 3: Ternary Polarity Guard {-1: Guard, 0: RO, +1: RW}
      if not PTE.Valid or else PTE.Trit_Perm < 0 or else (Is_Write and then PTE.Trit_Perm <= 0) then
         return (Status => TLB_Fault_Ternary_Guard, Phys_Addr => 0);
      end if;

      -- Gate 4: Temporal Epoch Generation Match
      if Handle.Cap_Slot_ID /= PTE.Cap_Slot_ID or else Handle.Epoch_Gen /= PTE.Expected_Epoch then
         return (Status => TLB_Fault_Epoch_Stale, Phys_Addr => 0);
      end if;

      -- Gate 5: 3^3 = 27 Futhark Cube Tag Match
      if Handle.Futhark_Tag_R2 /= PTE.Futhark_Tag_R2 then
         return (Status => TLB_Fault_Futhark_Tag, Phys_Addr => 0);
      end if;

      -- Gate 6: 33-bit Carry-Safe Slice Bounds Check
      End_Blocks := Unsigned_64 (Handle.Block_Offset) + Unsigned_64 (Xfer_Beats);
      if Xfer_Beats = 0 or else End_Blocks > Unsigned_64 (PTE.Max_Blocks) then
         return (Status => TLB_Fault_Slice_Bounds, Phys_Addr => 0);
      end if;

      -- Gate 7: Capability Effect Mask Subset Check
      if (Req_Caps and PTE.Allowed_Caps) /= Req_Caps then
         return (Status => TLB_Fault_Cap_Rights, Phys_Addr => 0);
      end if;

      return (Status    => TLB_Status_OK,
              Phys_Addr => Shift_Left (PTE.Phys_Base_Block + Unsigned_64 (Handle.Block_Offset), 6));
   end Translate_Virtual_Capability;

end Agp_Iommu_Tlb_Spark;

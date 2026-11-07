pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;           use Interfaces;
with Embedded_Quant_Peter; use Embedded_Quant_Peter;
with Felix_Nand_TCB;       use Felix_Nand_TCB;
with Ljodatal_18_Spark;    use Ljodatal_18_Spark;

package Agp_Iommu_Tlb_Spark
  with Pure, SPARK_Mode => On
is
   subtype Epoch_Gen_Byte     is Unsigned_8;
   subtype Cap_Slot_Byte      is Unsigned_8;
   subtype Futhark_Cube_Index is Natural range 0 .. 26;
   subtype Byte_Align_6       is Unsigned_8 range 0 .. 63;
   subtype Phys_Block_36      is Unsigned_64 range 0 .. 16#000F_FFFF_FFFF#;

   type TLB_Translation_Status is
     (TLB_Status_OK,
      TLB_Fault_Unaligned,
      TLB_Fault_Charm18_Seal,
      TLB_Fault_Ternary_Guard,
      TLB_Fault_Epoch_Stale,
      TLB_Fault_Futhark_Tag,
      TLB_Fault_Slice_Bounds,
      TLB_Fault_Cap_Rights);

   type Virt_Cap_Handle_Record is record
      Epoch_Gen      : Epoch_Gen_Byte     := 0;
      Cap_Slot_ID    : Cap_Slot_Byte      := 0;
      Futhark_Tag_R2 : Futhark_Cube_Index := 0;
      Block_Offset   : Unsigned_32        := 0;
      Byte_Align     : Byte_Align_6       := 0;
   end record;

   type Cap_PTE_Record is record
      Phys_Base_Block : Phys_Block_36      := 0;
      Max_Blocks      : Unsigned_32        := 0;
      Allowed_Caps    : Capability_Mask    := Cap_None;
      Expected_Epoch  : Epoch_Gen_Byte     := 0;
      Cap_Slot_ID     : Cap_Slot_Byte      := 0;
      Futhark_Tag_R2  : Futhark_Cube_Index := 0;
      Trit_Perm       : Balanced_Trit      := -1;
      Valid           : Boolean            := False;
   end record;

   type Translation_Result is record
      Status    : TLB_Translation_Status := TLB_Fault_Ternary_Guard;
      Phys_Addr : Unsigned_64            := 0;
   end record;

   -- Evaluates the 7-Gate Zero-Trust 27-Way Futhark IOMMU Capability Check
   function Translate_Virtual_Capability
     (Handle     : Virt_Cap_Handle_Record;
      Xfer_Beats : Unsigned_32;
      Req_Caps   : Capability_Mask;
      Is_Write   : Boolean;
      PTE        : Cap_PTE_Record) return Translation_Result
     with Inline,
          Pre  => True,
          Post => (if Translate_Virtual_Capability'Result.Status = TLB_Status_OK then
                     PTE.Valid
                     and then Handle.Byte_Align = 0
                     and then (Req_Caps and Cap_Root_Seal) = 0
                     and then (PTE.Allowed_Caps and Cap_Root_Seal) = 0
                     and then Handle.Epoch_Gen = PTE.Expected_Epoch
                     and then Handle.Futhark_Tag_R2 = PTE.Futhark_Tag_R2
                     and then (Translate_Virtual_Capability'Result.Phys_Addr and 16#3F#) = 0
                   else
                     Translate_Virtual_Capability'Result.Phys_Addr = 0);

end Agp_Iommu_Tlb_Spark;

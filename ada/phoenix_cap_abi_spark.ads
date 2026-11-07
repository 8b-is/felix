pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;          use Interfaces;
with Felix_Nand_TCB;      use Felix_Nand_TCB;
with Ljodatal_18_Spark;   use Ljodatal_18_Spark;
with Agp_Iommu_Tlb_Spark; use Agp_Iommu_Tlb_Spark;

package Phoenix_Cap_Abi_Spark
  with Pure, SPARK_Mode => On
is
   type Target_Backend_Kind is
     (Backend_Linux_Mmap,
      Backend_Wasm32_Linear,
      Backend_BareMetal_DMA);

   type Cap_Slot_Table_256 is array (Cap_Slot_Byte) of Cap_PTE_Record;

   -- OS-Agnostic Capability Invocation Guard:
   -- Discharges all 7 IOMMU gates identically regardless of whether the underlying
   -- arena is backed by Linux `mmap`, Wasm32 linear memory, or Bare-Metal AXI4 DMA.
   function Invoke_Phoenix_Cap_Guard
     (Backend         : Target_Backend_Kind;
      Table           : Cap_Slot_Table_256;
      Handle          : Virt_Cap_Handle_Record;
      Xfer_Beats      : Unsigned_32;
      Req_Caps        : Capability_Mask;
      Is_Write        : Boolean;
      Hotswap_Pending : Boolean) return Translation_Result
     with Inline,
          Pre  => True,
          Post => (if Hotswap_Pending then
                     Invoke_Phoenix_Cap_Guard'Result.Phys_Addr = 0
                   elsif Invoke_Phoenix_Cap_Guard'Result.Status = TLB_Status_OK then
                     (Invoke_Phoenix_Cap_Guard'Result.Phys_Addr and 16#3F#) = 0
                     and then ((Req_Caps and Cap_Root_Seal) = 0));

   -- Ghost Theorem: Target Backend Agnosticism
   -- Proves that Linux mmap, WebAssembly linear memory, and Bare-Metal DMA produce
   -- bit-identical capability translation verdicts and physical offsets!
   function Prove_OS_Agnostic_Equivalence
     (Table           : Cap_Slot_Table_256;
      Handle          : Virt_Cap_Handle_Record;
      Xfer_Beats      : Unsigned_32;
      Req_Caps        : Capability_Mask;
      Is_Write        : Boolean;
      Hotswap_Pending : Boolean) return Boolean is
     (Invoke_Phoenix_Cap_Guard
        (Backend_Linux_Mmap, Table, Handle, Xfer_Beats, Req_Caps, Is_Write, Hotswap_Pending) =
      Invoke_Phoenix_Cap_Guard
        (Backend_Wasm32_Linear, Table, Handle, Xfer_Beats, Req_Caps, Is_Write, Hotswap_Pending)
      and then
      Invoke_Phoenix_Cap_Guard
        (Backend_Wasm32_Linear, Table, Handle, Xfer_Beats, Req_Caps, Is_Write, Hotswap_Pending) =
      Invoke_Phoenix_Cap_Guard
        (Backend_BareMetal_DMA, Table, Handle, Xfer_Beats, Req_Caps, Is_Write, Hotswap_Pending))
     with Ghost,
          Pre  => True,
          Post => Prove_OS_Agnostic_Equivalence'Result = True;

   -- Advances `Expected_Epoch` of a capability slot at an `@hotswappable` safe-point
   -- and proves that any handle holding the old epoch immediately faults with `TLB_Fault_Epoch_Stale`!
   procedure HotSwap_Advance_Slot_Epoch
     (PTE        : in out Cap_PTE_Record;
      Old_Handle : in     Virt_Cap_Handle_Record;
      New_Handle : out    Virt_Cap_Handle_Record)
     with Pre  => PTE.Valid
                  and then Old_Handle.Cap_Slot_ID = PTE.Cap_Slot_ID
                  and then Old_Handle.Epoch_Gen   = PTE.Expected_Epoch
                  and then Old_Handle.Byte_Align  = 0
                  and then (PTE.Allowed_Caps and Cap_Root_Seal) = 0
                  and then PTE.Trit_Perm >= 0,
          Post => New_Handle.Epoch_Gen = PTE.Expected_Epoch
                  and then New_Handle.Epoch_Gen /= Old_Handle.Epoch_Gen
                  and then Translate_Virtual_Capability
                             (Old_Handle, 1, Cap_None, False, PTE).Status = TLB_Fault_Epoch_Stale;

end Phoenix_Cap_Abi_Spark;

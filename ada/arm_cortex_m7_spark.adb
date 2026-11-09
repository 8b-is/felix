pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Arm_Cortex_M7_Spark
  with SPARK_Mode => On
is
   function Is_Valid_MMIO_Address (Addr : Unsigned_32) return Boolean is
   begin
      if (Addr and 16#0000_0003#) /= 0 then
         return False;
      end if;
      return (Addr >= MMIO_PERIPH_BASE and then Addr <= MMIO_PERIPH_END)
        or else (Addr >= MMIO_PPB_BASE and then Addr <= MMIO_PPB_END);
   end Is_Valid_MMIO_Address;

   function Clear_W1C_Status
     (Current_Status : Unsigned_32;
      Handled_Mask   : Unsigned_32) return Unsigned_32 is
   begin
      return Current_Status and (not Handled_Mask);
   end Clear_W1C_Status;

   function Is_Valid_DMA_Cache_Aligned_32
     (Addr       : Unsigned_32;
      Size_Bytes : Unsigned_32) return Boolean is
   begin
      return Size_Bytes > 0
        and then (Addr and CACHE_LINE_MASK_32) = 0
        and then (Size_Bytes and CACHE_LINE_MASK_32) = 0;
   end Is_Valid_DMA_Cache_Aligned_32;

   function Can_NVIC_Preempt
     (Candidate_Prio : NVIC_Priority_4Bit;
      Running_Prio   : NVIC_Priority_4Bit) return Boolean is
   begin
      return Candidate_Prio < Running_Prio;
   end Can_NVIC_Preempt;

   function Encode_BASEPRI_4Bit
     (Threshold : NVIC_Priority_4Bit) return Unsigned_32 is
   begin
      return Shift_Left (Threshold, 4);
   end Encode_BASEPRI_4Bit;

   function Validate_Teensy4_LPSPI_Config
     (Clock_Hz          : Unsigned_32;
      Peripheral_Active : Boolean) return LPSPI_Validation_Result
   is
      Div : Unsigned_32;
   begin
      if Peripheral_Active
        or else Clock_Hz = 0
        or else Clock_Hz > TEENSY4_LPSPI_MAX_HZ
      then
         return (Valid => False, Use_DMA => False, SCK_Div => 0);
      end if;

      Div := (TEENSY4_LPSPI_MAX_HZ + Clock_Hz - 1) / Clock_Hz;
      if Div < 1 then
         Div := 1;
      end if;

      return (Valid   => True,
              Use_DMA => Clock_Hz > SPI_DMA_THRESHOLD_HZ,
              SCK_Div => Div);
   end Validate_Teensy4_LPSPI_Config;

   function Verify_Stack_Guard_And_Canary
     (Current_SP      : Unsigned_32;
      MPU_Guard_Limit : Unsigned_32;
      Observed_Canary : Unsigned_32) return Boolean is
   begin
      return Current_SP >= MPU_Guard_Limit
        and then Observed_Canary = STACK_CANARY_MAGIC;
   end Verify_Stack_Guard_And_Canary;

   function Canonical_DMA_Speedup_PQ return Rational_PQ is
   begin
      return (P => 4, Q => 1);
   end Canonical_DMA_Speedup_PQ;

end Arm_Cortex_M7_Spark;

pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB;       use Felix_Nand_TCB;
with Embedded_Quant_Peter; use Embedded_Quant_Peter;

package Well_Signal_Spqt_Spark
  with Pure, SPARK_Mode => On
is
   Well_Register_Trits : constant := 12; -- 4 registers x 3 trits
   Well_Meta_Trits     : constant := 14; -- 12 register trits + 2 structural trits
   Well_Total_Trits    : constant := 15; -- 14 data trits + 1 Z_3 parity trit = 3 x 5

   type Well_14_Index is range 0 .. Well_Meta_Trits - 1;
   type Well_15_Index is range 0 .. Well_Total_Trits - 1;
   type SPQT_3_Index  is range 0 .. 2;

   type Well_Signal_14_Array is array (Well_14_Index) of Balanced_Trit;
   type Well_Signal_15_Array is array (Well_15_Index) of Balanced_Trit;
   type Well_SPQT_3_Bytes    is array (SPQT_3_Index)  of Packed_5Trit_Byte;

   type Decoded_Well_Result is record
      Trits_14     : Well_Signal_14_Array := (others => 0);
      Parity_Valid : Boolean              := False;
   end record;

   -- Computes the exact Z_3 parity trit in {-1, 0, +1} so that the sum of all 15
   -- unsigned trits (t_i + 1) is divisible by 3.
   function Compute_Z3_Parity_Trit (Data_14 : Well_Signal_14_Array) return Balanced_Trit
     with Inline,
          Pre  => True,
          Post => Compute_Z3_Parity_Trit'Result in -1 .. 1;

   -- Packs 14 signal/structural trits + 1 Z_3 parity trit into 3 SPQT Base-243 bytes (0 .. 242)
   function Pack_Well_Signal_SPQT3
     (Data_14 : Well_Signal_14_Array) return Well_SPQT_3_Bytes
     with Pre  => True,
          Post => (for all G in SPQT_3_Index => Pack_Well_Signal_SPQT3'Result (G) in 0 .. 242);

   -- Unpacks 3 SPQT Base-243 bytes and verifies the Z_3 parity trit, proven to recover Data_14!
   function Unpack_Well_Signal_SPQT3
     (Bytes_3 : Well_SPQT_3_Bytes) return Decoded_Well_Result
     with Pre  => True,
          Post => (if Unpack_Well_Signal_SPQT3'Result.Parity_Valid then
                     Pack_Well_Signal_SPQT3 (Unpack_Well_Signal_SPQT3'Result.Trits_14) = Bytes_3);

   -- Ghost Theorem: 100% Lossless Bijection for any 14-trit Well Signal Table
   function Prove_Well_SPQT3_Bijection
     (Data_14 : Well_Signal_14_Array) return Boolean is
     (Unpack_Well_Signal_SPQT3 (Pack_Well_Signal_SPQT3 (Data_14)).Parity_Valid
      and then Unpack_Well_Signal_SPQT3 (Pack_Well_Signal_SPQT3 (Data_14)).Trits_14 = Data_14)
     with Ghost,
          Pre  => True,
          Post => Prove_Well_SPQT3_Bijection'Result = True;

end Well_Signal_Spqt_Spark;

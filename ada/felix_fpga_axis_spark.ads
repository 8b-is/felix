pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

with Interfaces;           use Interfaces;
with Felix_Nand_TCB;       use Felix_Nand_TCB;
with Embedded_Quant_Peter; use Embedded_Quant_Peter;

package Felix_Fpga_Axis_Spark
  with Pure, SPARK_Mode => On
is
   Array_Dim_64 : constant := 64;
   type Col_Index_64 is range 0 .. Array_Dim_64 - 1;

   type Act_Vec_64_I16  is array (Col_Index_64) of Integer_16;
   type Trit_Row_64_Bal is array (Col_Index_64) of Balanced_Trit;

   -- Exact 17-bit PE product range: [-32768 .. +32768], immune to -(-32768) overflow!
   subtype PE_Product_S17 is Integer_32 range -32_768 .. +32_768;

   -- Max unscaled 64-element sum: 64 * 32_768 = 2_097_152
   subtype Row_Raw_Sum_I32 is Integer_32 range -(64 * 32_768) .. +(64 * 32_768);

   -- Max Q2.2 E2M1 scaled 64-element sum (|M| <= 12): 64 * 32_768 * 12 = 25_165_824
   subtype Row_PQ_Scaled_I32 is Integer_32 range -(64 * 32_768 * 12) .. +(64 * 32_768 * 12);

   subtype E2M1_Q2_Numerator is Integer_32 range -12 .. +12;

   function Eval_Ternary_PE_Cell
     (Act : Integer_16;
      W   : Balanced_Trit) return PE_Product_S17
     with Inline,
          Post => Eval_Ternary_PE_Cell'Result = Integer_32 (Act) * Integer_32 (W);

   function Eval_Row_64_Contraction
     (Acts   : Act_Vec_64_I16;
      Trits  : Trit_Row_64_Bal;
      M_Lo   : E2M1_Q2_Numerator;
      M_Hi   : E2M1_Q2_Numerator;
      Use_PQ : Boolean) return Row_PQ_Scaled_I32;

end Felix_Fpga_Axis_Spark;

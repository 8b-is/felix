pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;     use Interfaces;
with Felix_Nand_TCB; use Felix_Nand_TCB;

package Embedded_Quant_Peter
  with Pure, SPARK_Mode => On
is
   type Phase_Mod_3 is mod 3;

   -- Isentropic cyclic rotation over the balanced triad {-1, 0, +1}
   function Rotate_Cyclic
     (T     : Balanced_Trit;
      Phase : Phase_Mod_3) return Balanced_Trit
   is
     (case Phase is
         when 0 => T,
         when 1 => (if T = 1 then -1 else T + 1),
         when 2 => (if T = -1 then 1 else T - 1))
     with Inline,
          Pre  => T in -1 .. 1,
          Post => Rotate_Cyclic'Result in -1 .. 1;

   -- Widened 16-bit signed accumulator product preventing i8::MIN (-128) negation overflow
   function Safe_Trit_Act_Product
     (T   : Balanced_Trit;
      Act : Integer_8) return Integer_16
     with Inline,
          Pre  => T in -1 .. 1,
          Post => Safe_Trit_Act_Product'Result in -128 .. 128;

end Embedded_Quant_Peter;

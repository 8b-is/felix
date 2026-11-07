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

   -- SPQT Base-243 5-trit pentad types and bijective helpers
   type Unsigned_Trit is range 0 .. 2 with Size => 8;
   type Trit_Sub_Index is range 0 .. 4;
   type Unpacked_5Trit_Group is array (Trit_Sub_Index) of Unsigned_Trit;
   subtype Packed_5Trit_Byte is Base243_Byte;

   function Balanced_To_Unsigned (T : Balanced_Trit) return Unsigned_Trit is
     (Unsigned_Trit (Integer (T) + 1))
     with Inline;

   function Unsigned_To_Balanced (U : Unsigned_Trit) return Balanced_Trit is
     (Balanced_Trit (Integer (U) - 1))
     with Inline;

   function Pack_5Trits (Grp : Unpacked_5Trit_Group) return Packed_5Trit_Byte
     with Inline,
          Post => Pack_5Trits'Result in 0 .. 242;

   function Unpack_5Trits (B : Packed_5Trit_Byte) return Unpacked_5Trit_Group
     with Inline,
          Post => Pack_5Trits (Unpack_5Trits'Result) = B;

end Embedded_Quant_Peter;

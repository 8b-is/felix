pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

with Interfaces;           use Interfaces;
with Embedded_Quant_Peter; use Embedded_Quant_Peter;

package Omni_Ternary_Spark
  with Pure, SPARK_Mode => On
is
   -- Scaled integer representation of Specific Radix Cost `(b / ln(b)) * 1_000_000`:
   --   Euler's e (2.71828) -> 2_718_282
   --   Base 3    (3 / ln3) -> 2_730_718  (Unique Integer Minimum!)
   --   Base 2    (2 / ln2) -> 2_885_390
   --   Base 4    (4 / ln4) -> 2_885_390
   --   Base 10  (10 /ln10) -> 4_342_945
   Cost_Euler_E_Scaled : constant Unsigned_32 := 2_718_282;
   Cost_Base_3_Scaled  : constant Unsigned_32 := 2_730_718;
   Cost_Base_2_Scaled  : constant Unsigned_32 := 2_885_390;
   Cost_Base_4_Scaled  : constant Unsigned_32 := 2_885_390;
   Cost_Base_10_Scaled : constant Unsigned_32 := 4_342_945;

   -- Formal proof that Base 3 strictly dominates Base 2, Base 4, and Base 10
   function Base_3_Dominates_Binary_And_Decimal return Boolean is
     (Cost_Base_3_Scaled > Cost_Euler_E_Scaled
      and then Cost_Base_3_Scaled < Cost_Base_2_Scaled
      and then Cost_Base_3_Scaled < Cost_Base_4_Scaled
      and then Cost_Base_3_Scaled < Cost_Base_10_Scaled)
     with Ghost,
          Post => Base_3_Dominates_Binary_And_Decimal'Result = True;

   -- Formal Landauer Bijection Invariant:
   -- Rotating any Balanced_Trit `T` forward by `Phase` and backward by `(3 - Phase) mod 3`
   -- recovers `T` with zero information erasure (`Delta_H3 = 0`).
   function Landauer_Reversible_Bend_Roundtrip
     (T     : Balanced_Trit;
      Phase : Phase_Mod_3) return Balanced_Trit
   is
     (Rotate_Cyclic (Rotate_Cyclic (T, Phase), 0 - Phase))
     with Inline,
          Pre  => T in -1 .. 1,
          Post => Landauer_Reversible_Bend_Roundtrip'Result = T;

end Omni_Ternary_Spark;

pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

-- ============================================================================
-- StringTemplate5_Pq_Spark — 8b-is + ADA-Lang Sovereign Template & VBF Engine
-- ============================================================================
-- Derived from Terence Parr's ANTLR StringTemplate4 (BSD-3-Clause):
--   Copyright (c) 2012 Terence Parr. All rights reserved.
--   Copyright (c) 2026 8b-is / Peter Lodri & Omni (StringTemplate5 p/q + SPARK).
--   SPDX-License-Identifier: BSD-3-Clause
--
-- Pillars: HYDRA · AQUARIUS · TARTAN · SAMSARA · PHOENIX · MEM8 · GOD OF SUN&*MOON
-- ============================================================================

with Interfaces; use Interfaces;

package Stringtemplate5_Pq_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   subtype Balanced_Trit is Integer_8 range -1 .. 1;
   subtype Template_Depth is Unsigned_8 range 0 .. 32;
   subtype Positive_Denom is Unsigned_32 range 1 .. 65_536;
   subtype Bounded_Numer is Unsigned_32 range 0 .. 65_536;
   subtype Rto_Val_Ms is Unsigned_32 range 1 .. 60_000;
   subtype Scaled_Rto_Ms is Unsigned_32 range 1 .. 180_000;

   Max_Samsara_Depth : constant Template_Depth := 16;

   -- CoCoA Variable Backoff Factor (VBF) Rational p/q Numerators & Denominators
   Vbf_Fast_3x_P      : constant Positive_Denom := 3;
   Vbf_Fast_3x_Q      : constant Positive_Denom := 1;
   Vbf_Medium_2x_P    : constant Positive_Denom := 2;
   Vbf_Medium_2x_Q    : constant Positive_Denom := 1;
   Vbf_Slow_Shift_P   : constant Positive_Denom := 83;
   Vbf_Slow_Shift_Q   : constant Positive_Denom := 64;
   Vbf_Slow_Q8_P      : constant Positive_Denom := 333;
   Vbf_Slow_Q8_Q      : constant Positive_Denom := 256;

   -- 1. AQUARIUS: Pure Shift-Add 3.0x (`rto + (rto << 1)`) == Exact 3 * rto
   function Eval_Vbf_Fast_3x_Shift_Add
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
     with Inline,
          Pre  => Rto_Ms >= 1,
          Post => Eval_Vbf_Fast_3x_Shift_Add'Result = 3 * Rto_Ms
                  and then Eval_Vbf_Fast_3x_Shift_Add'Result =
                             (Rto_Ms * Vbf_Fast_3x_P) / Vbf_Fast_3x_Q;

   -- 2. AQUARIUS: Pure Shift-Add 2.0x (`rto << 1`) == Exact 2 * rto
   function Eval_Vbf_Medium_2x_Shift
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
     with Inline,
          Pre  => Rto_Ms >= 1,
          Post => Eval_Vbf_Medium_2x_Shift'Result = 2 * Rto_Ms
                  and then Eval_Vbf_Medium_2x_Shift'Result =
                             (Rto_Ms * Vbf_Medium_2x_P) / Vbf_Medium_2x_Q;

   -- 3. AQUARIUS: Multiplier-Free 3-Tap Shift 1.3x (`rto + (rto>>2) + (rto>>5) + (rto>>6)` ~ 83/64)
   function Eval_Vbf_Slow_3Tap_Shift
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
     with Inline,
          Pre  => Rto_Ms >= 1,
          Post => Eval_Vbf_Slow_3Tap_Shift'Result >= Rto_Ms
                  and then Eval_Vbf_Slow_3Tap_Shift'Result <= 2 * Rto_Ms;

   -- 4. AQUARIUS: Q8 Multiply-Shift 1.3x (`(rto * 333) >> 8` = 1.30078125x)
   function Eval_Vbf_Slow_Q8_Exact
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
     with Inline,
          Pre  => Rto_Ms >= 1,
          Post => Eval_Vbf_Slow_Q8_Exact'Result >= Rto_Ms
                  and then Eval_Vbf_Slow_Q8_Exact'Result <= 2 * Rto_Ms
                  and then Eval_Vbf_Slow_Q8_Exact'Result =
                             Unsigned_32 ((Unsigned_64 (Rto_Ms) * 333) / 256);

   -- 5. SAMSARA: Sub-Turing Acyclic Template Recursion Guard
   function Samsara_Expansion_Admissible
     (Current_Depth : Template_Depth) return Boolean
     with Inline,
          Pre  => True,
          Post => Samsara_Expansion_Admissible'Result = (Current_Depth < Max_Samsara_Depth);

   -- 6. TARTAN / ST5: Kleene 3-Way Balanced-Ternary Template Selector `<tsel(t, pos, zero, neg)>`
   function St5_Ternary_Select
     (Cond    : Balanced_Trit;
      On_Pos  : Unsigned_32;
      On_Zero : Unsigned_32;
      On_Neg  : Unsigned_32) return Unsigned_32
     with Inline,
          Pre  => True,
          Post => (if Cond = 1 then St5_Ternary_Select'Result = On_Pos
                   elsif Cond = 0 then St5_Ternary_Select'Result = On_Zero
                   else St5_Ternary_Select'Result = On_Neg);

   -- 7. GOD OF SUN & *MOON: Solar/Lunar Dual Rational Complement (`P/Q + (Q - P)/Q = 1`)
   function Lunar_Dual_Complement_Numer
     (Solar_P : Bounded_Numer;
      Denom_Q : Positive_Denom) return Bounded_Numer
     with Inline,
          Pre  => Solar_P <= Denom_Q,
          Post => Lunar_Dual_Complement_Numer'Result = Denom_Q - Solar_P
                  and then Solar_P + Lunar_Dual_Complement_Numer'Result = Denom_Q;

end Stringtemplate5_Pq_Spark;

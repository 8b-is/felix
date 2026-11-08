pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Stringtemplate5_Pq_Spark
  with SPARK_Mode => On
is
   function Eval_Vbf_Fast_3x_Shift_Add
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
   is
      R : constant Unsigned_32 := Unsigned_32 (Rto_Ms);
   begin
      return Scaled_Rto_Ms (R + Shift_Left (R, 1));
   end Eval_Vbf_Fast_3x_Shift_Add;

   function Eval_Vbf_Medium_2x_Shift
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
   is
      R : constant Unsigned_32 := Unsigned_32 (Rto_Ms);
   begin
      return Scaled_Rto_Ms (Shift_Left (R, 1));
   end Eval_Vbf_Medium_2x_Shift;

   function Eval_Vbf_Slow_3Tap_Shift
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
   is
      R : constant Unsigned_32 := Unsigned_32 (Rto_Ms);
   begin
      return Scaled_Rto_Ms
        (R + Shift_Right (R, 2) + Shift_Right (R, 5) + Shift_Right (R, 6));
   end Eval_Vbf_Slow_3Tap_Shift;

   function Eval_Vbf_Slow_Q8_Exact
     (Rto_Ms : Rto_Val_Ms) return Scaled_Rto_Ms
   is
      Prod : constant Unsigned_64 := Unsigned_64 (Rto_Ms) * 333;
   begin
      return Scaled_Rto_Ms (Unsigned_32 (Shift_Right (Prod, 8)));
   end Eval_Vbf_Slow_Q8_Exact;

   function Samsara_Expansion_Admissible
     (Current_Depth : Template_Depth) return Boolean
   is
   begin
      return Current_Depth < Max_Samsara_Depth;
   end Samsara_Expansion_Admissible;

   function St5_Ternary_Select
     (Cond    : Balanced_Trit;
      On_Pos  : Unsigned_32;
      On_Zero : Unsigned_32;
      On_Neg  : Unsigned_32) return Unsigned_32
   is
   begin
      if Cond > 0 then
         return On_Pos;
      elsif Cond = 0 then
         return On_Zero;
      else
         return On_Neg;
      end if;
   end St5_Ternary_Select;

   function Lunar_Dual_Complement_Numer
     (Solar_P : Bounded_Numer;
      Denom_Q : Positive_Denom) return Bounded_Numer
   is
   begin
      return Denom_Q - Solar_P;
   end Lunar_Dual_Complement_Numer;

end Stringtemplate5_Pq_Spark;

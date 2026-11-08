pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Ndh_Symplectic_Probe_Spark
  with SPARK_Mode => On
is

   function Forward_Shear_Q
     (Q : Bounded_Coord;
      P : Bounded_Coord;
      W : Balanced_Trit) return Sheared_Coord
   is
   begin
      return Q + Integer_64 (W) * P;
   end Forward_Shear_Q;

   function Inverse_Shear_Q
     (Q_Sheared : Sheared_Coord;
      P         : Bounded_Coord;
      W         : Balanced_Trit) return Integer_64
   is
   begin
      return Q_Sheared - Integer_64 (W) * P;
   end Inverse_Shear_Q;

   function Verify_Shear_Roundtrip_Exact
     (Q : Bounded_Coord;
      P : Bounded_Coord;
      W : Balanced_Trit) return Boolean
   is
      Q_Next : constant Sheared_Coord := Forward_Shear_Q (Q, P, W);
      Q_Back : constant Integer_64   := Inverse_Shear_Q (Q_Next, P, W);
   begin
      return Q_Back = Q;
   end Verify_Shear_Roundtrip_Exact;

   function Symplectic_Omega_Pair
     (Q1, P1, Q2, P2 : Sheared_Coord) return Integer_64
   is
   begin
      return Q1 * P2 - P1 * Q2;
   end Symplectic_Omega_Pair;

   function Verify_Symplectic_2Form_Conserved
     (Q1, P1, Q2, P2 : Bounded_Coord;
      W              : Balanced_Trit) return Boolean
   is
      Q1_Next : constant Sheared_Coord := Forward_Shear_Q (Q1, P1, W);
      Q2_Next : constant Sheared_Coord := Forward_Shear_Q (Q2, P2, W);
      Omega_0 : constant Integer_64    := Symplectic_Omega_Pair (Q1, P1, Q2, P2);
      Omega_1 : constant Integer_64    := Symplectic_Omega_Pair (Q1_Next, P1, Q2_Next, P2);
   begin
      return Omega_1 = Omega_0;
   end Verify_Symplectic_2Form_Conserved;

   function Sigma_Delta_Quantize_Step
     (X_In        : Quant_Sample;
      Residual_In : Quant_Resid;
      Scale_Q     : Quant_Scale) return Sigma_Delta_Result
   is
      U    : constant Integer_64 := X_In + Residual_In;
      Trit : Balanced_Trit;
      Res  : Quant_Resid;
   begin
      if U > Scale_Q / 2 then
         Trit := 1;
         Res  := U - Scale_Q;
      elsif U < -(Scale_Q / 2) then
         Trit := -1;
         Res  := U + Scale_Q;
      else
         Trit := 0;
         Res  := U;
      end if;
      return Sigma_Delta_Result'(Trit_Out => Trit, Residual_Out => Res);
   end Sigma_Delta_Quantize_Step;

   function Step_Tri_System_Dist_BC
     (State_A       : Bounded_Coord;
      State_B       : Bounded_Coord;
      State_C       : Bounded_Coord;
      Cap_Isolate_A : Boolean) return Integer_64
   is
      Leak_Mask : constant Integer_64 := (if Cap_Isolate_A then 0 else 1);
      Next_B    : constant Integer_64 := State_B + Leak_Mask * State_A;
      Next_C    : constant Integer_64 := State_C - Leak_Mask * State_A;
   begin
      return abs (Next_B - Next_C);
   end Step_Tri_System_Dist_BC;

end Ndh_Symplectic_Probe_Spark;

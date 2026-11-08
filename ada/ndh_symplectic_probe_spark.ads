pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

-- ============================================================================
-- Ndh_Symplectic_Probe_Spark — MEM|8 x Borealiscodes Representation-Level Proof
-- ============================================================================
-- Proves invariants directly on the 64-bit signed integer machine representation
-- (`Integer_64`) rather than idealized real numbers:
--   1. Exact integer canonical shear round-trip invertibility (`Forward` -> `Inverse` == ID).
--   2. Exact discrete symplectic 2-form conservation (`Omega(S(u), S(v)) = Omega(u, v)`).
--   3. Exact integer Sigma-Delta error-feedback conservation (`u - y * Q = r'`).
--   4. Non-vacuous capability-gated Tri-System partition isolation (`State_A` enters
--      the operator, yet `Cap_Isolate_A = True` zeroes cross-talk into `d(B', C')`).
-- ============================================================================

with Interfaces; use Interfaces;

package Ndh_Symplectic_Probe_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   subtype Balanced_Trit is Integer_8 range -1 .. 1;
   subtype Bounded_Coord is Integer_64 range -1_000_000_000 .. 1_000_000_000;
   subtype Sheared_Coord is Integer_64 range -2_000_000_000 .. 2_000_000_000;
   subtype Quant_Scale   is Integer_64 range 1 .. 1_000_000;
   subtype Quant_Sample  is Integer_64 range -500_000 .. 500_000;
   subtype Quant_Resid   is Integer_64 range -1_000_000 .. 1_000_000;

   type Sigma_Delta_Result is record
      Trit_Out     : Balanced_Trit;
      Residual_Out : Quant_Resid;
   end record;

   -- 1. Forward Canonical Integer Shear on Q (`Q' = Q + W * P`)
   function Forward_Shear_Q
     (Q : Bounded_Coord;
      P : Bounded_Coord;
      W : Balanced_Trit) return Sheared_Coord
     with Inline,
          Pre  => Q in Bounded_Coord and then P in Bounded_Coord,
          Post => Forward_Shear_Q'Result = Q + Integer_64 (W) * P;

   -- 2. Exact Inverse Integer Shear on Q (`Q = Q' - W * P`)
   function Inverse_Shear_Q
     (Q_Sheared : Sheared_Coord;
      P         : Bounded_Coord;
      W         : Balanced_Trit) return Integer_64
     with Inline,
          Pre  => Q_Sheared in Sheared_Coord and then P in Bounded_Coord,
          Post => Inverse_Shear_Q'Result = Q_Sheared - Integer_64 (W) * P;

   -- 3. Representation-Level Bijective Round-Trip Theorem (`Inverse(Forward(Q, P, W), P, W) = Q`)
   function Verify_Shear_Roundtrip_Exact
     (Q : Bounded_Coord;
      P : Bounded_Coord;
      W : Balanced_Trit) return Boolean
     with Inline,
          Pre  => Q in Bounded_Coord and then P in Bounded_Coord,
          Post => Verify_Shear_Roundtrip_Exact'Result = True
                  and then Inverse_Shear_Q (Forward_Shear_Q (Q, P, W), P, W) = Q;

   -- 4. Representation-Level Discrete Symplectic 2-Form Conservation Theorem
   --    `Omega((Q1', P1), (Q2', P2)) = Q1'*P2 - P1*Q2' = Q1*P2 - P1*Q2 = Omega((Q1, P1), (Q2, P2))`
   function Symplectic_Omega_Pair
     (Q1, P1, Q2, P2 : Sheared_Coord) return Integer_64
     with Inline,
          Pre  => Q1 in Sheared_Coord and then P1 in Sheared_Coord
                  and then Q2 in Sheared_Coord and then P2 in Sheared_Coord,
          Post => Symplectic_Omega_Pair'Result = Q1 * P2 - P1 * Q2;

   function Verify_Symplectic_2Form_Conserved
     (Q1, P1, Q2, P2 : Bounded_Coord;
      W              : Balanced_Trit) return Boolean
     with Inline,
          Pre  => Q1 in Bounded_Coord and then P1 in Bounded_Coord
                  and then Q2 in Bounded_Coord and then P2 in Bounded_Coord,
          Post => Verify_Symplectic_2Form_Conserved'Result = True
                  and then Symplectic_Omega_Pair
                             (Forward_Shear_Q (Q1, P1, W), P1,
                              Forward_Shear_Q (Q2, P2, W), P2)
                           = Symplectic_Omega_Pair (Q1, P1, Q2, P2);

   -- 5. First-Order Integer Sigma-Delta Error-Feedback Quantizer Step
   function Sigma_Delta_Quantize_Step
     (X_In        : Quant_Sample;
      Residual_In : Quant_Resid;
      Scale_Q     : Quant_Scale) return Sigma_Delta_Result
     with Inline,
          Pre  => abs (X_In) <= Scale_Q / 2
                  and then abs (Residual_In) <= Scale_Q,
          Post => X_In + Residual_In -
                    Integer_64 (Sigma_Delta_Quantize_Step'Result.Trit_Out) * Scale_Q =
                    Sigma_Delta_Quantize_Step'Result.Residual_Out
                  and then abs (Sigma_Delta_Quantize_Step'Result.Residual_Out) <= Scale_Q;

   -- 6. Non-Vacuous Capability-Gated Tri-System Isolation Step
   --    `State_A` is an explicit formal input to the transition operator.
   function Step_Tri_System_Dist_BC
     (State_A       : Bounded_Coord;
      State_B       : Bounded_Coord;
      State_C       : Bounded_Coord;
      Cap_Isolate_A : Boolean) return Integer_64
     with Inline,
          Pre  => State_A in Bounded_Coord
                  and then State_B in Bounded_Coord
                  and then State_C in Bounded_Coord,
          Post => (if Cap_Isolate_A then
                     Step_Tri_System_Dist_BC'Result = abs (State_B - State_C));

end Ndh_Symplectic_Probe_Spark;

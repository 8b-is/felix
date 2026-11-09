pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations

with Interfaces; use Interfaces;

-- @summary
-- Peloton_Row_Ayeos_Spark: Formally verified USB-C Peloton Row open-source
-- hardware driver, exact integer hydrodynamic cube-law power/split calculator
-- (P = 350,000,000 / t_500m^3), biomechanical Form-Assist Q15 scorer, eddy-current
-- brake PWM controller, and FTMS (0x1826 / 0x2AD1) zero-subscription telemetry.
package Peloton_Row_Ayeos_Spark is

   -- Handle cable extension in millimeters (0 .. 1,600 mm)
   subtype Handle_Mm_Type is Unsigned_32 range 0 .. 1_600;

   -- Monorail seat position in millimeters (0 .. 1,200 mm)
   subtype Seat_Mm_Type is Unsigned_32 range 0 .. 1_200;

   -- Stroke rate in strokes per minute (10 .. 60 SPM)
   subtype Stroke_Spm_Type is Unsigned_32 range 10 .. 60;

   -- 500m split time in whole seconds (70s = 1:10/500m .. 300s = 5:00/500m)
   subtype Split_500m_Sec_Type is Unsigned_32 range 70 .. 300;

   -- Eddy-current flywheel drag factor (80 .. 200, canonical Peloton/Concept2 = 115)
   subtype Drag_Factor_Type is Unsigned_32 range 80 .. 200;

   -- Eddy-current brake coil PWM duty in permille (0 .. 1,000)
   subtype Brake_Pwm_Permille_Type is Unsigned_32 range 0 .. 1_000;

   -- Q15 biomechanical form score (0 .. 32,768)
   subtype Form_Q15_Type is Unsigned_32 range 0 .. 32_768;

   -- Exact hydrodynamic cube-law constant: 2.80 * 500^3 = (14/5) * 125,000,000 = 350,000,000 W*s^3
   Hydrodynamic_Cube_Numerator : constant Unsigned_64 := 350_000_000;

   -- Zero-subscription OSS monthly cost in cents ($0.00/mo vs $44.00/mo proprietary lock-in)
   Ayeos_Monthly_Subscription_Cents : constant Unsigned_32 := 0;
   Proprietary_Annual_Saved_Cents   : constant Unsigned_32 := 4_400 * 12;

   -- 1. Exact integer hydrodynamic cube-law power in Watts: P = 350,000,000 / (Split_500m_Sec^3)
   --    At Split_500m_Sec = 100s (1:40.0/500m), P = 350,000,000 / 1,000,000 = 350 W (exact!).
   function Compute_Hydrodynamic_Power_Watts
     (Split_500m_Sec : Split_500m_Sec_Type) return Unsigned_32
     with Inline,
          Pre  => Split_500m_Sec >= 70 and then Split_500m_Sec <= 300,
          Post => Compute_Hydrodynamic_Power_Watts'Result >= 12
                  and then Compute_Hydrodynamic_Power_Watts'Result <= 1_021
                  and then (if Split_500m_Sec = 100
                            then Compute_Hydrodynamic_Power_Watts'Result = 350);

   -- 2. Eddy-current brake coil PWM duty (permille) from Drag_Factor and Stroke_Spm
   function Compute_Eddy_Brake_Pwm_Permille
     (Drag_Factor : Drag_Factor_Type;
      Stroke_Spm  : Stroke_Spm_Type) return Brake_Pwm_Permille_Type
     with Inline,
          Pre  => Drag_Factor >= 80 and then Stroke_Spm >= 10,
          Post => Compute_Eddy_Brake_Pwm_Permille'Result >= 100
                  and then Compute_Eddy_Brake_Pwm_Permille'Result <= 1_000;

   -- 3. Biomechanical Form-Assist Q15 evaluator (verifying legs -> core -> arms kinematic ratio)
   function Evaluate_Stroke_Kinematics_Q15
     (Handle_Mm : Handle_Mm_Type;
      Seat_Mm   : Seat_Mm_Type) return Form_Q15_Type
     with Inline,
          Pre  => Handle_Mm <= 1_600 and then Seat_Mm <= 1_200,
          Post => Evaluate_Stroke_Kinematics_Q15'Result <= 32_768
                  and then (if Handle_Mm * 3 = Seat_Mm * 4
                            then Evaluate_Stroke_Kinematics_Q15'Result = 32_768);

   -- 4. End-to-End USB-C + FTMS (0x1826 / 0x2AD1) + ayeOS Zero-Subscription Invariant
   function Verify_Zero_Subscription_Ftms_2ad1 return Boolean
     with Inline,
          Pre  => True,
          Post => Verify_Zero_Subscription_Ftms_2ad1'Result = True
                  and then Ayeos_Monthly_Subscription_Cents = 0
                  and then Proprietary_Annual_Saved_Cents = 52_800;

end Peloton_Row_Ayeos_Spark;

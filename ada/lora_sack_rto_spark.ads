pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Lora_Sack_Rto_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   -- Bounded millisecond domains for LoRa Time-on-Air (SF7..SF12) and RFC 6298 / CoCoA RTO
   subtype Rtt_Ms is Unsigned_32 range 0 .. 600_000;
   subtype Positive_Rtt_Ms is Unsigned_32 range 1 .. 60_000;
   subtype Dithered_Rtt_Ms is Unsigned_32 range 1 .. 90_000;
   subtype Scaled_Rtt_Ms is Unsigned_32 range 0 .. 4_800_000;
   subtype Backoff_Exponent is Unsigned_8 range 0 .. 6;
   subtype Vbf_Step_Count is Unsigned_8 range 1 .. 4;
   subtype Retry_Count is Unsigned_8 range 0 .. 255;

   -- CoCoA Variable Backoff Factor (VBF) Regimes (1000 ms / 3000 ms canonical thresholds)
   type Vbf_Regime is (Fast_3x, Medium_2x, Slow_1_3x);

   Clock_Granularity_Ms : constant Rtt_Ms := 10;
   Max_Weak_Retries     : constant Retry_Count := 2;

   -- 1. LoRa Time-on-Air Minimum RTO Floor: rto_min_ms = (toa_ms << 1) + guard_ms
   function Compute_Lora_Rto_Min_Ms
     (Toa_Ms   : Positive_Rtt_Ms;
      Guard_Ms : Rtt_Ms) return Rtt_Ms
     with Inline,
          Pre  => Guard_Ms <= 60_000,
          Post => Compute_Lora_Rto_Min_Ms'Result = 2 * Toa_Ms + Guard_Ms
                  and then Compute_Lora_Rto_Min_Ms'Result >= 2 * Toa_Ms;

   -- 2. RFC 6298 Section 2.2 First Sample Initialization (srtt_x8 = R << 3, rttvar_x4 = R << 1)
   procedure Seed_First_Rtt_Sample
     (R_Ms      : in  Positive_Rtt_Ms;
      Srtt_X8   : out Scaled_Rtt_Ms;
      Rttvar_X4 : out Scaled_Rtt_Ms)
     with Inline,
          Pre  => R_Ms >= 1,
          Post => Srtt_X8 = R_Ms * 8
                  and then Rttvar_X4 = R_Ms * 2
                  and then Srtt_X8 / 8 = R_Ms;

   -- 3. Karn's Rule Admissibility Gate across Cumulative ACKs and RFC 2018 / 6675 SACK Bitmaps
   function Karns_Rule_Admissible
     (Retries : Retry_Count;
      Sacked  : Boolean) return Boolean
     with Inline,
          Pre  => True,
          Post => Karns_Rule_Admissible'Result = (Retries = 0 and then not Sacked);

   -- 4. CoCoA Weak Estimator Admissibility Gate (1 <= Retries <= 2 and not Sacked)
   function Cocoa_Weak_Admissible
     (Retries : Retry_Count;
      Sacked  : Boolean) return Boolean
     with Inline,
          Pre  => True,
          Post => Cocoa_Weak_Admissible'Result =
                    (Retries >= 1 and then Retries <= Max_Weak_Retries and then not Sacked);

   -- 5. Binary Exponential Backoff Clamped to [Rto_Min_Ms, Rto_Max_Ms]
   function Compute_Backed_Off_Rto
     (Rto_Base_Ms : Positive_Rtt_Ms;
      Shift       : Backoff_Exponent;
      Rto_Min_Ms  : Positive_Rtt_Ms;
      Rto_Max_Ms  : Positive_Rtt_Ms) return Positive_Rtt_Ms
     with Inline,
          Pre  => Rto_Min_Ms <= Rto_Max_Ms,
          Post => Compute_Backed_Off_Rto'Result >= Rto_Min_Ms
                  and then Compute_Backed_Off_Rto'Result <= Rto_Max_Ms;

   -- 6. CoCoA Division-Free LFSR Dithering: rto_dither = rto_ms + ((rto_ms * u8) >> 9) in [1.0, 1.5)x
   function Dither_Rto_Division_Free
     (Rto_Ms    : Positive_Rtt_Ms;
      Random_U8 : Unsigned_8) return Dithered_Rtt_Ms
     with Inline,
          Pre  => Rto_Ms >= 1,
          Post => Dither_Rto_Division_Free'Result >= Rto_Ms
                  and then Dither_Rto_Division_Free'Result <= Rto_Ms + Rto_Ms / 2;

   -- 7. CoCoA Variable Backoff Factor (VBF) Regime Selector (< 1000ms -> 3x, 1000..3000ms -> 2x, > 3000ms -> 1.3x)
   function Select_Vbf_Regime
     (Rto_Base_Ms : Positive_Rtt_Ms) return Vbf_Regime
     with Inline,
          Pre  => Rto_Base_Ms >= 1,
          Post => (if Rto_Base_Ms < 1_000 then Select_Vbf_Regime'Result = Fast_3x
                   elsif Rto_Base_Ms <= 3_000 then Select_Vbf_Regime'Result = Medium_2x
                   else Select_Vbf_Regime'Result = Slow_1_3x);

   -- 8. Adaptive Maximum VBF Backoff Steps by Base RTO Regime (uIP / ContikiMAC bound)
   function Max_Backoffs_For_Base
     (Rto_Base_Ms : Positive_Rtt_Ms) return Vbf_Step_Count
     with Inline,
          Pre  => Rto_Base_Ms >= 1,
          Post => (if Rto_Base_Ms < 3_000 then Max_Backoffs_For_Base'Result = 4
                   elsif Rto_Base_Ms < 6_000 then Max_Backoffs_For_Base'Result = 3
                   elsif Rto_Base_Ms < 12_000 then Max_Backoffs_For_Base'Result = 2
                   else Max_Backoffs_For_Base'Result = 1);

   -- 9. Pure-Integer CoCoA VBF Timeout Step (3x = rto+(rto<<1), 2x = rto<<1, 1.3x = (rto*333)>>8 or 83/64 shift)
   function Step_Vbf_Rto_Ms
     (Rto_Current_Ms  : Positive_Rtt_Ms;
      Rto_Base_Ms     : Positive_Rtt_Ms;
      Rto_Min_Ms      : Positive_Rtt_Ms;
      Rto_Max_Ms      : Positive_Rtt_Ms;
      Use_Pure_Shifts : Boolean) return Positive_Rtt_Ms
     with Inline,
          Pre  => Rto_Min_Ms <= Rto_Max_Ms,
          Post => Step_Vbf_Rto_Ms'Result >= Rto_Min_Ms
                  and then Step_Vbf_Rto_Ms'Result <= Rto_Max_Ms;

end Lora_Sack_Rto_Spark;

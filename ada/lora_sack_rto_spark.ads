pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Lora_Sack_Rto_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   -- Bounded millisecond domains for LoRa Time-on-Air (SF7..SF12) and RFC 6298 RTO
   subtype Rtt_Ms is Unsigned_32 range 0 .. 600_000;
   subtype Positive_Rtt_Ms is Unsigned_32 range 1 .. 60_000;
   subtype Scaled_Rtt_Ms is Unsigned_32 range 0 .. 4_800_000;
   subtype Backoff_Exponent is Unsigned_8 range 0 .. 6;
   subtype Retry_Count is Unsigned_8 range 0 .. 255;

   Clock_Granularity_Ms : constant Rtt_Ms := 10;

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

   -- 4. Binary Exponential Backoff Clamped to [Rto_Min_Ms, Rto_Max_Ms]
   function Compute_Backed_Off_Rto
     (Rto_Base_Ms : Positive_Rtt_Ms;
      Shift       : Backoff_Exponent;
      Rto_Min_Ms  : Positive_Rtt_Ms;
      Rto_Max_Ms  : Positive_Rtt_Ms) return Positive_Rtt_Ms
     with Inline,
          Pre  => Rto_Min_Ms <= Rto_Max_Ms,
          Post => Compute_Backed_Off_Rto'Result >= Rto_Min_Ms
                  and then Compute_Backed_Off_Rto'Result <= Rto_Max_Ms;

end Lora_Sack_Rto_Spark;

pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Lora_Sack_Rto_Spark
  with SPARK_Mode => On
is
   function Compute_Lora_Rto_Min_Ms
     (Toa_Ms   : Positive_Rtt_Ms;
      Guard_Ms : Rtt_Ms) return Rtt_Ms
   is
   begin
      return 2 * Toa_Ms + Guard_Ms;
   end Compute_Lora_Rto_Min_Ms;

   procedure Seed_First_Rtt_Sample
     (R_Ms      : in  Positive_Rtt_Ms;
      Srtt_X8   : out Scaled_Rtt_Ms;
      Rttvar_X4 : out Scaled_Rtt_Ms)
   is
   begin
      Srtt_X8   := R_Ms * 8;
      Rttvar_X4 := R_Ms * 2;
   end Seed_First_Rtt_Sample;

   function Karns_Rule_Admissible
     (Retries : Retry_Count;
      Sacked  : Boolean) return Boolean
   is
   begin
      return Retries = 0 and then not Sacked;
   end Karns_Rule_Admissible;

   function Cocoa_Weak_Admissible
     (Retries : Retry_Count;
      Sacked  : Boolean) return Boolean
   is
   begin
      return Retries >= 1 and then Retries <= Max_Weak_Retries and then not Sacked;
   end Cocoa_Weak_Admissible;

   function Compute_Backed_Off_Rto
     (Rto_Base_Ms : Positive_Rtt_Ms;
      Shift       : Backoff_Exponent;
      Rto_Min_Ms  : Positive_Rtt_Ms;
      Rto_Max_Ms  : Positive_Rtt_Ms) return Positive_Rtt_Ms
   is
      Mult    : constant Unsigned_32 := Shift_Left (Unsigned_32 (1), Natural (Shift));
      Shifted : constant Unsigned_32 := Unsigned_32 (Rto_Base_Ms) * Mult;
   begin
      if Shifted > Unsigned_32 (Rto_Max_Ms) then
         return Rto_Max_Ms;
      elsif Shifted < Unsigned_32 (Rto_Min_Ms) then
         return Rto_Min_Ms;
      else
         return Positive_Rtt_Ms (Shifted);
      end if;
   end Compute_Backed_Off_Rto;

   function Dither_Rto_Division_Free
     (Rto_Ms    : Positive_Rtt_Ms;
      Random_U8 : Unsigned_8) return Dithered_Rtt_Ms
   is
      Prod   : constant Unsigned_64 := Unsigned_64 (Rto_Ms) * Unsigned_64 (Random_U8);
      Jitter : constant Unsigned_64 := Shift_Right (Prod, 9);
   begin
      return Dithered_Rtt_Ms (Unsigned_32 (Rto_Ms) + Unsigned_32 (Jitter));
   end Dither_Rto_Division_Free;

   function Select_Vbf_Regime
     (Rto_Base_Ms : Positive_Rtt_Ms) return Vbf_Regime
   is
   begin
      if Rto_Base_Ms < 1_000 then
         return Fast_3x;
      elsif Rto_Base_Ms <= 3_000 then
         return Medium_2x;
      else
         return Slow_1_3x;
      end if;
   end Select_Vbf_Regime;

   function Max_Backoffs_For_Base
     (Rto_Base_Ms : Positive_Rtt_Ms) return Vbf_Step_Count
   is
   begin
      if Rto_Base_Ms < 3_000 then
         return 4;
      elsif Rto_Base_Ms < 6_000 then
         return 3;
      elsif Rto_Base_Ms < 12_000 then
         return 2;
      else
         return 1;
      end if;
   end Max_Backoffs_For_Base;

   function Step_Vbf_Rto_Ms
     (Rto_Current_Ms  : Positive_Rtt_Ms;
      Rto_Base_Ms     : Positive_Rtt_Ms;
      Rto_Min_Ms      : Positive_Rtt_Ms;
      Rto_Max_Ms      : Positive_Rtt_Ms;
      Use_Pure_Shifts : Boolean) return Positive_Rtt_Ms
   is
      Rto    : constant Unsigned_32 := Unsigned_32 (Rto_Current_Ms);
      Regime : constant Vbf_Regime  := Select_Vbf_Regime (Rto_Base_Ms);
      Scaled : Unsigned_32;
   begin
      case Regime is
         when Fast_3x =>
            -- 3.0x = rto + (rto << 1)
            Scaled := Rto + Shift_Left (Rto, 1);
         when Medium_2x =>
            -- 2.0x = rto << 1
            Scaled := Shift_Left (Rto, 1);
         when Slow_1_3x =>
            if Use_Pure_Shifts then
               -- 83 / 64 = 1.296875x = rto + (rto >> 2) + (rto >> 5) + (rto >> 6)
               Scaled := Rto + Shift_Right (Rto, 2) + Shift_Right (Rto, 5) + Shift_Right (Rto, 6);
            else
               -- 333 / 256 = 1.30078125x = (rto * 333) >> 8
               Scaled := Unsigned_32 (Shift_Right (Unsigned_64 (Rto) * 333, 8));
            end if;
      end case;

      if Scaled > Unsigned_32 (Rto_Max_Ms) then
         return Rto_Max_Ms;
      elsif Scaled < Unsigned_32 (Rto_Min_Ms) then
         return Rto_Min_Ms;
      else
         return Positive_Rtt_Ms (Scaled);
      end if;
   end Step_Vbf_Rto_Ms;

end Lora_Sack_Rto_Spark;

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

end Lora_Sack_Rto_Spark;

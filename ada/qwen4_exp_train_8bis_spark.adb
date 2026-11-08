pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Qwen4_Exp_Train_8bis_Spark
  with SPARK_Mode => On
is

   function Peetpedro_11_Datasets_Permille_Sum return Unsigned_32 is
      W_Cogito    : constant Unsigned_32 := 250;
      W_8bis      : constant Unsigned_32 := 150;
      W_Dogfood   : constant Unsigned_32 := 120;
      W_Komp_V4   : constant Unsigned_32 := 100;
      W_Headroomz : constant Unsigned_32 := 60;
      W_Bitnet    : constant Unsigned_32 := 60;
      W_Quantal2  : constant Unsigned_32 := 100;
      W_Music     : constant Unsigned_32 := 40;
      W_Axiom     : constant Unsigned_32 := 40;
      W_Voice     : constant Unsigned_32 := 50;
      W_Overflow  : constant Unsigned_32 := 30;
   begin
      return W_Cogito + W_8bis + W_Dogfood + W_Komp_V4 + W_Headroomz
             + W_Bitnet + W_Quantal2 + W_Music + W_Axiom + W_Voice + W_Overflow;
   end Peetpedro_11_Datasets_Permille_Sum;

   function Qsa_Trailing_Uncompressed
     (Seq_Len : Seq_Length_Type;
      Ratio   : Compress_Ratio) return Unsigned_32
   is
   begin
      return Seq_Len mod Ratio;
   end Qsa_Trailing_Uncompressed;

   function Householder_Involution_Check (X : Coord_Range) return Integer_64 is
      Reflected_Once  : constant Integer_64 := -X;
      Reflected_Twice : constant Integer_64 := -Reflected_Once;
   begin
      return Reflected_Twice;
   end Householder_Involution_Check;

   function Verify_A6000_Vram_Budget return Boolean is
      Base_Weights : constant Unsigned_32 := 4_096;
      Adapters     : constant Unsigned_32 := 1_536;
      Optimizer    : constant Unsigned_32 := 4_608;
      Activations  : constant Unsigned_32 := 24_576;
      Peak         : constant Unsigned_32 := Base_Weights + Adapters + Optimizer + Activations;
   begin
      return Peak = A6000_Peak_Vram_Mib
             and then Peak + A6000_Free_Vram_Mib = A6000_Total_Vram_Mib
             and then Peak * 24 = A6000_Total_Vram_Mib * 17;
   end Verify_A6000_Vram_Budget;

end Qwen4_Exp_Train_8bis_Spark;

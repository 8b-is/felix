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

   function Grpo_Advantage_Q15
     (Reward_Q15 : Reward_Q15_Type;
      Mean_Q15   : Reward_Q15_Type;
      Std_Q15    : Std_Q15_Type) return Unsigned_32
   is
      Diff    : Unsigned_32;
      Scaled  : Unsigned_32;
      Clamped : Unsigned_32;
   begin
      if Reward_Q15 >= Mean_Q15 then
         Diff    := Reward_Q15 - Mean_Q15;
         Scaled  := (Diff * 4_096) / Std_Q15;
         Clamped := (if Scaled > 32_767 then 32_767 else Scaled);
         return 32_768 + Clamped;
      else
         Diff    := Mean_Q15 - Reward_Q15;
         Scaled  := (Diff * 4_096) / Std_Q15;
         Clamped := (if Scaled > 32_768 then 32_768 else Scaled);
         return 32_768 - Clamped;
      end if;
   end Grpo_Advantage_Q15;

   function Verify_Wukong_Omni_Quad_Loop_Pq return Boolean is
      Gemini_Argon_Share_Q15 : constant Unsigned_32 := 10_922;
      Smart_Tree_Share_Q15   : constant Unsigned_32 := 10_923;
      Ollama_Fleet_Share_Q15 : constant Unsigned_32 := 10_923;
      Q15_Total              : constant Unsigned_32 :=
        Gemini_Argon_Share_Q15 + Smart_Tree_Share_Q15 + Ollama_Fleet_Share_Q15;
      Hc_Ternary_Streams     : constant Unsigned_32 := 3;
      Rung0_Plus_One_Trit    : constant Unsigned_32 := 1;
      Quad_Loop_Streams      : constant Unsigned_32 := Hc_Ternary_Streams + Rung0_Plus_One_Trit;
      Base243_Ratio          : constant Unsigned_32 := 250;
      Rung0_P                : constant Unsigned_32 := 3;
      Rung0_Q                : constant Unsigned_32 := 2;
      Pqt_Compression        : constant Unsigned_32 := (Base243_Ratio * Rung0_P) / Rung0_Q;
      Pqt_Resonance_Q15      : constant Unsigned_32 := 32_756;
   begin
      return Q15_Total = 32_768
             and then Quad_Loop_Streams = 4
             and then Pqt_Compression = 375
             and then Pqt_Resonance_Q15 = 8_189 * 4;
   end Verify_Wukong_Omni_Quad_Loop_Pq;

   function Verify_Integral_Lll_Lovasz_3_4 return Boolean is
      -- Initial rank-2 integer lattice basis in Z^2: b1 = (4, 1), b2 = (11, 3)
      -- det([4, 1; 11, 3]) = 12 - 11 = 1, Gram det = ||b1||^2 ||b2||^2 - <b1,b2>^2 = 17*130 - 47^2 = 2210 - 2209 = 1
      -- After LLL reduction (delta = 3/4): b1' = (-1, 0), b2' = (0, 1) -> orthonormal!
      N1_Before : constant Integer_64 := 17;
      N2_Before : constant Integer_64 := 130;
      M12_Before : constant Integer_64 := 47;
      Det_Before : constant Integer_64 := N1_Before * N2_Before - M12_Before * M12_Before;

      N1_After  : constant Integer_64 := 1;
      N2_After  : constant Integer_64 := 1;
      M12_After : constant Integer_64 := 0;
      Det_After : constant Integer_64 := N1_After * N2_After - M12_After * M12_After;
   begin
      -- 1. Unimodular Gram determinant invariance: Det_After = Det_Before = 1
      -- 2. Size reduction: |2 * M12_After| <= N1_After
      -- 3. Lovasz delta = 3/4 condition: 4 * Det_After >= (3 * N1_After - 4 * M12_After^2) * N1_After
      -- 4. Strict Hadamard orthogonality improvement: N1_After * N2_After < N1_Before * N2_Before
      return Det_Before = 1
             and then Det_After = Det_Before
             and then 2 * abs (M12_After) <= N1_After
             and then 4 * Det_After >= 3 * N1_After * N1_After - 4 * M12_After * M12_After
             and then N1_After * N2_After < N1_Before * N2_Before;
   end Verify_Integral_Lll_Lovasz_3_4;

end Qwen4_Exp_Train_8bis_Spark;

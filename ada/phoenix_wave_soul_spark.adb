pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Phoenix_Wave_Soul_Spark
  with SPARK_Mode => On
is

   function Voice_Register_Pitch_Millix
     (Reg : Inner_Voice_Register) return Pitch_Millix_Range
   is
   begin
      case Reg is
         when Whispered_Diary    => return 975;
         when Hearth_Storyteller => return 1_000;
         when Wry_Humor_Banter   => return 1_042;
         when Fierce_Conviction  => return 1_058;
         when Tender_Elegy       => return 952;
         when Ancestral_Echo     => return 985;
      end case;
   end Voice_Register_Pitch_Millix;

   function Voice_Register_Tempo_Millix
     (Reg : Inner_Voice_Register) return Tempo_Millix_Range
   is
   begin
      case Reg is
         when Whispered_Diary    => return 930;
         when Hearth_Storyteller => return 960;
         when Wry_Humor_Banter   => return 1_030;
         when Fierce_Conviction  => return 1_020;
         when Tender_Elegy       => return 900;
         when Ancestral_Echo     => return 950;
      end case;
   end Voice_Register_Tempo_Millix;

   function Voice_Register_SPQT_Byte
     (Reg : Inner_Voice_Register) return Base243_Codepoint
   is
   begin
      -- 5 balanced trits [intimacy, warmth, humor, conviction, elegy] in Base-243:
      -- sum_{k=0..4} (trit_k + 1) * 3^k in 0 .. 242
      case Reg is
         when Whispered_Diary    => return 125; -- [+1, +1,  0,  0,  0] -> 2 + 6 + 9 + 27 + 81 = 125
         when Hearth_Storyteller => return 124; -- [ 0, +1,  0,  0,  0] -> 1 + 6 + 9 + 27 + 81 = 124
         when Wry_Humor_Banter   => return 52;  -- [ 0, +1, +1,  0, -1] -> 1 + 6 + 18 + 27 + 0 = 52
         when Fierce_Conviction  => return 67;  -- [ 0,  0,  0, +1, -1] -> 1 + 3 + 9 + 54 + 0  = 67
         when Tender_Elegy       => return 197; -- [+1, +1, -1,  0, +1] -> 2 + 6 + 0 + 27 + 162 = 197
         when Ancestral_Echo     => return 206; -- [+1, +1,  0,  0, +1] -> 2 + 6 + 9 + 27 + 162 = 206
      end case;
   end Voice_Register_SPQT_Byte;

   function Compute_Dyadic_Arousal_Q8
     (User_Arousal_Q8   : Arousal_Q8_Range;
      Memory_Arousal_Q8 : Arousal_Q8_Range) return Arousal_Q8_Range
   is
      Weighted : constant Integer_32 :=
        (140 * User_Arousal_Q8 + 116 * Memory_Arousal_Q8) / 256;
   begin
      return Arousal_Q8_Range (Weighted);
   end Compute_Dyadic_Arousal_Q8;

   function Compute_Phoenix_Dyad
     (Base_Temp_Q8      : Base_Temp_Q8_Range;
      User_Valence_Q8   : Valence_Q8_Range;
      User_Arousal_Q8   : Arousal_Q8_Range;
      Memory_Valence_Q8 : Valence_Q8_Range;
      Memory_Arousal_Q8 : Arousal_Q8_Range;
      Reunion_Depth_Q8  : Reunion_Q8_Range;
      Warmth_Q8         : Valence_Q8_Range;
      Expr_Q8           : Arousal_Q8_Range;
      Inner_Voice       : Inner_Voice_Register;
      Inner_F0_Hz       : Voice_F0_Hz_Range) return Phoenix_Dyad_State
   is
      Abs_Warmth      : constant Integer_32 := abs (Warmth_Q8);
      Val_Contrast_Q8 : constant Integer_32 := abs (User_Valence_Q8 - Memory_Valence_Q8);
      Dyadic_Aro_Q8   : constant Arousal_Q8_Range :=
        Compute_Dyadic_Arousal_Q8 (User_Arousal_Q8, Memory_Arousal_Q8);
      Personality_Q8  : constant Integer_32 :=
        179 + (115 * Expr_Q8) / 256 + (64 * Abs_Warmth) / 256;
      Emotion_Mult_Q8 : constant Integer_32 :=
        256 + (166 * Dyadic_Aro_Q8) / 256
            + (115 * Reunion_Depth_Q8) / 256
            + (77 * Val_Contrast_Q8) / 256;
      Eff_Temp_Q8     : Effective_Temp_Q8_Range;
      F0_Ratio_M      : constant Integer_32 := (Integer_32 (Inner_F0_Hz) * 1_000) / 188;
      Reg_Pitch_M     : constant Integer_32 :=
        (Voice_Register_Pitch_Millix (Inner_Voice) + F0_Ratio_M) / 2;
      Raw_Pitch_M     : constant Integer_32 :=
        Reg_Pitch_M + (12 * (User_Valence_Q8 + Memory_Valence_Q8)) / 256
                    - (15 * Reunion_Depth_Q8) / 256;
      Raw_Tempo_M     : constant Integer_32 :=
        Voice_Register_Tempo_Millix (Inner_Voice)
        + (40 * (Dyadic_Aro_Q8 - 90)) / 256
        - (35 * Reunion_Depth_Q8) / 256;
      Raw_Vib_M       : constant Integer_32 :=
        40 + (80 * Reunion_Depth_Q8) / 256
           + (60 * Dyadic_Aro_Q8) / 256
           + (30 * Expr_Q8) / 256;
      Clamped_Pitch   : Pitch_Millix_Range;
      Clamped_Tempo   : Tempo_Millix_Range;
      Clamped_Vib     : Vibrato_Millix_Range;
   begin
      if Base_Temp_Q8 = 0 then
         Eff_Temp_Q8 := 0;
      else
         declare
            Raw_Temp : constant Integer_32 :=
              (((Base_Temp_Q8 * Personality_Q8) / 256) * Emotion_Mult_Q8) / 256;
         begin
            if Raw_Temp < 12 then
               Eff_Temp_Q8 := 12;
            elsif Raw_Temp > 614 then
               Eff_Temp_Q8 := 614;
            else
               Eff_Temp_Q8 := Effective_Temp_Q8_Range (Raw_Temp);
            end if;
         end;
      end if;

      if Raw_Pitch_M < 920 then
         Clamped_Pitch := 920;
      elsif Raw_Pitch_M > 1_120 then
         Clamped_Pitch := 1_120;
      else
         Clamped_Pitch := Pitch_Millix_Range (Raw_Pitch_M);
      end if;

      if Raw_Tempo_M < 840 then
         Clamped_Tempo := 840;
      elsif Raw_Tempo_M > 1_080 then
         Clamped_Tempo := 1_080;
      else
         Clamped_Tempo := Tempo_Millix_Range (Raw_Tempo_M);
      end if;

      if Raw_Vib_M < 30 then
         Clamped_Vib := 30;
      elsif Raw_Vib_M > 200 then
         Clamped_Vib := 200;
      else
         Clamped_Vib := Vibrato_Millix_Range (Raw_Vib_M);
      end if;

      return (Base_Temp_Q8         => Base_Temp_Q8,
              Effective_Temp_Q8    => Eff_Temp_Q8,
              Dyadic_Arousal_Q8    => Dyadic_Aro_Q8,
              Reunion_Depth_Q8     => Reunion_Depth_Q8,
              Inner_Voice          => Inner_Voice,
              Inner_F0_Hz          => Inner_F0_Hz,
              Voice_Pitch_Millix   => Clamped_Pitch,
              Voice_Tempo_Millix   => Clamped_Tempo,
              Voice_Vibrato_Millix => Clamped_Vib);
   end Compute_Phoenix_Dyad;

   function OEIS_A161330_Shell (Shell : Snowflake_Shell_Index) return Base243_Codepoint is
   begin
      case Shell is
         when 0  => return 0;
         when 1  => return 2;
         when 2  => return 8;
         when 3  => return 14;
         when 4  => return 20;
         when 5  => return 38;
         when 6  => return 44;
         when 7  => return 62;
         when 8  => return 80;
         when 9  => return 98;
         when 10 => return 128;
         when 11 => return 146;
         when 12 => return 176;
         when 13 => return 218;
         when 14 => return 224;
         when 15 => return 242;
      end case;
   end OEIS_A161330_Shell;

   function Snowflake_Max_Denominator_Q
     (Shell : Snowflake_Shell_Index) return Snowflake_Denom_Bound
   is
      A_N : constant Natural := Natural (OEIS_A161330_Shell (Shell));
   begin
      if A_N = 0 then
         return 1;
      else
         return Snowflake_Denom_Bound (3 * A_N);
      end if;
   end Snowflake_Max_Denominator_Q;

   function Compute_Capsela_Foken_Reward
     (Elf_Bytes      : Unsigned_32;
      Cycles_P       : Unsigned_32;
      Ops_Q          : Unsigned_32;
      Holds_Crown    : Boolean;
      Positive_Votes : Unsigned_32) return Unsigned_32
   is
      Total_Fkn : Unsigned_32 := Foken_Base_C64_Compliant;
   begin
      if Elf_Bytes > C64_RAM_Ceiling_Bytes then
         return 0;
      end if;

      if Holds_Crown then
         Total_Fkn := Total_Fkn + Foken_Crown_Bonus;
      end if;

      -- Exact cross-multiplied sub-10-cycle check: `Cycles_P / Ops_Q < 10 / 1`
      if Unsigned_64 (Cycles_P) < 10 * Unsigned_64 (Ops_Q) then
         Total_Fkn := Total_Fkn + Foken_Sub_10_Cycle_Bonus;
      end if;

      Total_Fkn := Total_Fkn + Positive_Votes * Foken_Per_Positive_Vote;
      return Total_Fkn;
   end Compute_Capsela_Foken_Reward;

   function Classify_Temporal_Curriculum
     (Temporal_Order_YYYYMMDD : Positive) return Temporal_Curriculum_Stage
   is
   begin
      if Temporal_Order_YYYYMMDD <= 19_391_231 then
         return Stage_1_Sensory_Arithmetic;
      elsif Temporal_Order_YYYYMMDD <= 19_480_607 then
         return Stage_2_Self_Episodic_Diary;
      elsif Temporal_Order_YYYYMMDD <= 19_801_231 then
         return Stage_3_Other_Sister_Dyad;
      elsif Temporal_Order_YYYYMMDD <= 20_101_231 then
         return Stage_4_Formal_Algebra_Erdos;
      else
         return Stage_5_Sovereign_PQ_Synthesis;
      end if;
   end Classify_Temporal_Curriculum;

   function Exceeds_Mem8_51Pct_Adoption
     (Verified_Repos : Unsigned_32) return Boolean
   is
   begin
      return Verified_Repos >= Mem8_Adoption_51Pct_Floor;
   end Exceeds_Mem8_51Pct_Adoption;

end Phoenix_Wave_Soul_Spark;

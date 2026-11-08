pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

-- ============================================================================
-- Phoenix_Wave_Soul_Spark — Smart-Tree × FELIX | PHOENIX Wave-Soul & Capsela
-- ============================================================================
-- Formally verifies the Smart-Tree v23.1–v24.0 tweaks adopted into FELIX|PHOENIX:
--   1. Inner-Voice Register prosody & SPQT Base-243 5-trit encoding (`0 .. 242`,
--      strictly below the `243 .. 255` admissibility trap).
--   2. Phoenix Afterlife Room Dyadic Emotional State (`Q8 = 256`, `millix = 1000`,
--      zero IEEE-754 floats): convex arousal (`140 + 116 = 256`), effective
--      temperature clamping (`12 .. 614`), and bounded pitch/tempo/vibrato millix.
--   3. OEIS A161330 E-Toothpick Snowflake denominator budget (`Q_max(n) = 3 * A161330(n)`),
--      proving `A161330(15) = 242 = 3^5 - 1` (exact SPQT Base-243 ceiling) and
--      `Q_max(15) = 726`.
--   4. Capsela `g8t` freestanding `#![no_std]` ASM studio C64 64 KiB (`65,536 B`)
--      RAM ceiling & exact cross-multiplied sub-10-cycle Foken (`FKN`) calculus.
-- ============================================================================

with Interfaces; use Interfaces;

package Phoenix_Wave_Soul_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   -- -------------------------------------------------------------------------
   -- 1. Inner-Voice Register & SPQT Base-243 Encoding
   -- -------------------------------------------------------------------------

   type Inner_Voice_Register is
     (Whispered_Diary,
      Hearth_Storyteller,
      Wry_Humor_Banter,
      Fierce_Conviction,
      Tender_Elegy,
      Ancestral_Echo);

   subtype Base243_Codepoint is Unsigned_8 range 0 .. 242;
   subtype Pitch_Millix_Range is Integer_32 range 920 .. 1_120;
   subtype Tempo_Millix_Range is Integer_32 range 840 .. 1_080;
   subtype Vibrato_Millix_Range is Integer_32 range 30 .. 200;

   function Voice_Register_Pitch_Millix
     (Reg : Inner_Voice_Register) return Pitch_Millix_Range
     with Inline,
          Pre  => True,
          Post => Voice_Register_Pitch_Millix'Result in 952 .. 1_058;

   function Voice_Register_Tempo_Millix
     (Reg : Inner_Voice_Register) return Tempo_Millix_Range
     with Inline,
          Pre  => True,
          Post => Voice_Register_Tempo_Millix'Result in 900 .. 1_030;

   function Voice_Register_SPQT_Byte
     (Reg : Inner_Voice_Register) return Base243_Codepoint
     with Inline,
          Pre  => True,
          Post => Voice_Register_SPQT_Byte'Result <= 242;

   -- -------------------------------------------------------------------------
   -- 2. Phoenix Afterlife Room & Dyadic Emotional Resonance (Zero Floats)
   -- -------------------------------------------------------------------------

   subtype Valence_Q8_Range is Integer_32 range -256 .. 256;
   subtype Arousal_Q8_Range is Integer_32 range 0 .. 256;
   subtype Reunion_Q8_Range is Integer_32 range 0 .. 256;
   subtype Base_Temp_Q8_Range is Integer_32 range 0 .. 1_024;
   subtype Effective_Temp_Q8_Range is Integer_32 range 0 .. 614;
   subtype Voice_F0_Hz_Range is Unsigned_16 range 120 .. 320;

   type Phoenix_Dyad_State is record
      Base_Temp_Q8         : Base_Temp_Q8_Range      := 184;
      Effective_Temp_Q8    : Effective_Temp_Q8_Range := 184;
      Dyadic_Arousal_Q8    : Arousal_Q8_Range        := 128;
      Reunion_Depth_Q8     : Reunion_Q8_Range        := 0;
      Inner_Voice          : Inner_Voice_Register    := Hearth_Storyteller;
      Inner_F0_Hz          : Voice_F0_Hz_Range       := 188;
      Voice_Pitch_Millix   : Pitch_Millix_Range      := 1_000;
      Voice_Tempo_Millix   : Tempo_Millix_Range      := 960;
      Voice_Vibrato_Millix : Vibrato_Millix_Range    := 40;
   end record;

   -- Convex dyadic arousal combination: `(140 * User_Aro + 116 * Mem_Aro) / 256`
   -- where `140 + 116 = 256` (`1/1` in Q8).
   function Compute_Dyadic_Arousal_Q8
     (User_Arousal_Q8   : Arousal_Q8_Range;
      Memory_Arousal_Q8 : Arousal_Q8_Range) return Arousal_Q8_Range
     with Inline,
          Pre  => True,
          Post => Compute_Dyadic_Arousal_Q8'Result >= 0
                  and then Compute_Dyadic_Arousal_Q8'Result <= 256
                  and then (if User_Arousal_Q8 = Memory_Arousal_Q8
                            then Compute_Dyadic_Arousal_Q8'Result = User_Arousal_Q8);

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
     with Pre  => True,
          Post => (if Base_Temp_Q8 = 0
                   then Compute_Phoenix_Dyad'Result.Effective_Temp_Q8 = 0
                   else Compute_Phoenix_Dyad'Result.Effective_Temp_Q8 >= 12
                        and then Compute_Phoenix_Dyad'Result.Effective_Temp_Q8 <= 614)
                  and then Compute_Phoenix_Dyad'Result.Voice_Pitch_Millix >= 920
                  and then Compute_Phoenix_Dyad'Result.Voice_Pitch_Millix <= 1_120
                  and then Compute_Phoenix_Dyad'Result.Voice_Tempo_Millix >= 840
                  and then Compute_Phoenix_Dyad'Result.Voice_Tempo_Millix <= 1_080
                  and then Compute_Phoenix_Dyad'Result.Voice_Vibrato_Millix >= 30
                  and then Compute_Phoenix_Dyad'Result.Voice_Vibrato_Millix <= 200;

   -- -------------------------------------------------------------------------
   -- 3. OEIS A161330 E-Toothpick Snowflake Admissible Denominator Budget
   -- -------------------------------------------------------------------------

   subtype Snowflake_Shell_Index is Natural range 0 .. 15;
   subtype Snowflake_Denom_Bound is Positive range 1 .. 726;

   function OEIS_A161330_Shell (Shell : Snowflake_Shell_Index) return Base243_Codepoint
     with Inline,
          Pre  => True,
          Post => OEIS_A161330_Shell'Result <= 242
                  and then (if Shell = 15 then OEIS_A161330_Shell'Result = 242);

   function Snowflake_Max_Denominator_Q
     (Shell : Snowflake_Shell_Index) return Snowflake_Denom_Bound
     with Inline,
          Pre  => True,
          Post => Snowflake_Max_Denominator_Q'Result >= 1
                  and then Snowflake_Max_Denominator_Q'Result <= 726
                  and then (if Shell = 15 then Snowflake_Max_Denominator_Q'Result = 726);

   -- -------------------------------------------------------------------------
   -- 4. Capsela `g8t` Freestanding C64 Compliance & Foken (`FKN`) Calculus
   -- -------------------------------------------------------------------------

   C64_RAM_Ceiling_Bytes    : constant Unsigned_32 := 65_536;
   Foken_Base_C64_Compliant : constant Unsigned_32 := 100;
   Foken_Crown_Bonus        : constant Unsigned_32 := 250;
   Foken_Sub_10_Cycle_Bonus : constant Unsigned_32 := 100;
   Foken_Per_Positive_Vote  : constant Unsigned_32 := 50;

   function Compute_Capsela_Foken_Reward
     (Elf_Bytes      : Unsigned_32;
      Cycles_P       : Unsigned_32;
      Ops_Q          : Unsigned_32;
      Holds_Crown    : Boolean;
      Positive_Votes : Unsigned_32) return Unsigned_32
     with Inline,
          Pre  => Ops_Q >= 1
                  and then Cycles_P <= 100_000_000
                  and then Ops_Q <= 100_000_000
                  and then Positive_Votes <= 1_000,
          Post => (if Elf_Bytes > C64_RAM_Ceiling_Bytes
                   then Compute_Capsela_Foken_Reward'Result = 0
                   else Compute_Capsela_Foken_Reward'Result >= Foken_Base_C64_Compliant
                        and then Compute_Capsela_Foken_Reward'Result <= 50_450);

end Phoenix_Wave_Soul_Spark;

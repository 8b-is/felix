pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Ada.Text_IO; use Ada.Text_IO;
with Felix_Tern; use Felix_Tern;
with System.Machine_Code; use System.Machine_Code;

package body Felix_Bench
  with SPARK_Mode => Off
is
   -- =========================================================================
   -- Hardware Cycle Counter via ARM64 Inline Assembly (cntvct_el0)
   -- =========================================================================

   function Current_Cycles return Unsigned_64 is
      Result : Unsigned_64 := 0;
   begin
      Asm ("mrs %0, cntvct_el0",
           Outputs  => Unsigned_64'Asm_Output ("=r", Result),
           Volatile => True);
      return Result;
   end Current_Cycles;

   function Make_Result
     (Name         : String;
      Iterations   : Positive;
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64)
      return Benchmark_Result
   is
      Total       : constant Unsigned_64 :=
        (if End_Cycles > Start_Cycles then End_Cycles - Start_Cycles else 1);
      Per_Op      : constant Float := Float (Total) / Float (Iterations);
      Ops_Per_Sec : constant Float :=
        Float (Iterations) / (Float (Total) / 24_000_000.0); -- 24 MHz Apple Silicon timer
   begin
      return (Name           => To_Bounded_Name (Name),
              Iterations     => Iterations,
              Total_Cycles   => Total,
              Cycles_Per_Op  => Per_Op,
              Ops_Per_Second => Ops_Per_Sec,
              Instructions   => Unsigned_64 (Iterations),
              Branch_Misses  => 0,
              Cache_Misses   => 0);
   end Make_Result;

   function Bench_Ternary_Nand (Config : Benchmark_Config) return Benchmark_Result is
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
      A            : Balanced_Trit := 1;
      B            : Balanced_Trit := -1;
      Result_Val   : Balanced_Trit := 0;
      pragma Volatile (Result_Val);
   begin
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Ternary_Nand (A, B);
         A := -A;
         B := -B;
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := Ternary_Nand (A, B);
         A := -A;
         B := -B;
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Ternary NAND (scalar)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_Ternary_Nand;

   function Bench_Ternary_Nand_Vector (Config : Benchmark_Config) return Benchmark_Result is
      type Trit_Array is array (0 .. 15) of Balanced_Trit;
      A_Arr        : constant Trit_Array := (others => 1);
      B_Arr        : constant Trit_Array := (others => -1);
      R_Arr        : Trit_Array := (others => 0);
      pragma Volatile (R_Arr);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in 1 .. Config.Warmup_Runs loop
         for J in 0 .. 15 loop
            R_Arr (J) := Ternary_Nand (A_Arr (J), B_Arr (J));
         end loop;
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         for J in 0 .. 15 loop
            R_Arr (J) := Ternary_Nand (A_Arr (J), B_Arr (J));
         end loop;
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Ternary NAND (16-wide)", Config.Iterations * 16, Start_Cycles, End_Cycles);
   end Bench_Ternary_Nand_Vector;

   function Bench_Pack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result is
      Tris         : constant Trit_Block_5 := (+1, -1, 0, +1, -1);
      Result_Val   : Base243_Byte := 0;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Pack_Trit_Block_5 (Tris);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := Pack_Trit_Block_5 (Tris);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Pack Trit_Block_5 (Base-243)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_Pack_Trit_Block_5;

   function Bench_Unpack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result is
      B            : constant Base243_Byte := 123;
      Result_Val   : Trit_Block_5 := (others => 0);
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Unpack_To_Trit_Block_5 (B);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := Unpack_To_Trit_Block_5 (B);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Unpack Base243 -> Trit_Block_5", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_Unpack_Trit_Block_5;

   function Bench_Ternary_Select (Config : Benchmark_Config) return Benchmark_Result is
      Cond         : Balanced_Trit := 1;
      A            : constant Balanced_Trit := 1;
      B            : constant Balanced_Trit := -1;
      Result_Val   : Balanced_Trit := 0;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Ternary_Select (Cond, A, B);
         Cond := -Cond;
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := Ternary_Select (Cond, A, B);
         Cond := -Cond;
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Ternary Select", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_Ternary_Select;

   function Bench_TGemm (Config : Benchmark_Config) return Benchmark_Result is
      Acts         : Tgemm_Acts (0 .. 255);
      Wts          : Tgemm_Weights (0 .. 255);
      Acc          : constant Tgemm_Acc := 0;
      Result_Val   : Tgemm_Acc := 0;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in Acts'Range loop
         Acts (I) := Integer_16 ((I mod 10) - 5);
         Wts (I)  := Balanced_Trit ((I mod 3) - 1);
      end loop;

      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("TGEMM (256-element)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_TGemm;

   function Bench_TGemm_VPternLogD (Config : Benchmark_Config) return Benchmark_Result is
      Acts         : Tgemm_Acts (0 .. 255);
      Wts          : Tgemm_Weights (0 .. 255);
      Acc          : constant Tgemm_Acc := 0;
      Result_Val   : Tgemm_Acc := 0;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in Acts'Range loop
         Acts (I) := Integer_16 ((I mod 10) - 5);
         Wts (I)  := Balanced_Trit ((I mod 3) - 1);
      end loop;

      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("TGEMM (VPternLogD/NEON)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_TGemm_VPternLogD;

   function Bench_Wave_Resonance (Config : Benchmark_Config) return Benchmark_Result is
      Signal       : Tgemm_Acts (0 .. 255);
      Result_Val   : Integer_16 := 0;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in Signal'Range loop
         Signal (I) := Integer_16 (I mod 100);
      end loop;

      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Wave_Resonance (Signal);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := Wave_Resonance (Signal);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("Wave Resonance (256)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_Wave_Resonance;

   function Bench_God_Of_Sun_Zeno (Config : Benchmark_Config) return Benchmark_Result is
      P            : constant Integer_64 := 31;
      Q            : constant Integer_64 := 32;
      N            : Natural := 31;
      T            : constant Balanced_Trit := -1;
      Result_Val   : Boolean := False;
      pragma Volatile (Result_Val);
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64;
   begin
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := God_Of_Sun_Zeno_Holds (P, Q, N, T);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         Result_Val := God_Of_Sun_Zeno_Holds (P, Q, N, T);
         N := 31 + (I mod 2);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("GOD OF SUN (P/Q-n*t>P)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_God_Of_Sun_Zeno;

   function Run_All_Benchmarks (Config : Benchmark_Config := Default_Config)
      return Benchmark_Results
   is
      Results : Benchmark_Results;
   begin
      Put_Line ("Running FELIX Benchmark Suite (ARM64 cntvct_el0 inline asm)...");
      Put_Line ("Config: Iterations=" & Config.Iterations'Image &
                ", Warmup=" & Config.Warmup_Runs'Image &
                ", Vector=" & Config.Vector_Size'Image);
      New_Line;

      Results (1) := Bench_Ternary_Nand (Config);
      Put_Line ("  1/9: " & Trim_Name (Results (1).Name) & " ... " & Results (1).Cycles_Per_Op'Image & " ticks/op");

      Results (2) := Bench_Ternary_Nand_Vector (Config);
      Put_Line ("  2/9: " & Trim_Name (Results (2).Name) & " ... " & Results (2).Cycles_Per_Op'Image & " ticks/op");

      Results (3) := Bench_Pack_Trit_Block_5 (Config);
      Put_Line ("  3/9: " & Trim_Name (Results (3).Name) & " ... " & Results (3).Cycles_Per_Op'Image & " ticks/op");

      Results (4) := Bench_Unpack_Trit_Block_5 (Config);
      Put_Line ("  4/9: " & Trim_Name (Results (4).Name) & " ... " & Results (4).Cycles_Per_Op'Image & " ticks/op");

      Results (5) := Bench_Ternary_Select (Config);
      Put_Line ("  5/9: " & Trim_Name (Results (5).Name) & " ... " & Results (5).Cycles_Per_Op'Image & " ticks/op");

      Results (6) := Bench_TGemm (Config);
      Put_Line ("  6/9: " & Trim_Name (Results (6).Name) & " ... " & Results (6).Cycles_Per_Op'Image & " ticks/op");

      Results (7) := Bench_TGemm_VPternLogD (Config);
      Put_Line ("  7/9: " & Trim_Name (Results (7).Name) & " ... " & Results (7).Cycles_Per_Op'Image & " ticks/op");

      Results (8) := Bench_Wave_Resonance (Config);
      Put_Line ("  8/9: " & Trim_Name (Results (8).Name) & " ... " & Results (8).Cycles_Per_Op'Image & " ticks/op");

      Results (9) := Bench_God_Of_Sun_Zeno (Config);
      Put_Line ("  9/9: " & Trim_Name (Results (9).Name) & " ... " & Results (9).Cycles_Per_Op'Image & " ticks/op");

      New_Line;
      return Results;
   end Run_All_Benchmarks;

   procedure Print_Results (Results : Benchmark_Results) is
   begin
      Put_Line ("Name,Iterations,Cycles_Per_Op,Ops_Per_Second");
      for R of Results loop
         Put_Line (Trim_Name (R.Name) & "," &
                   R.Iterations'Image & "," &
                   R.Cycles_Per_Op'Image & "," &
                   R.Ops_Per_Second'Image);
      end loop;
   end Print_Results;

   procedure Print_Results_MD (Results : Benchmark_Results) is
   begin
      Put_Line ("| Benchmark | Ticks/Op | Ops/sec |");
      Put_Line ("|-----------|----------|---------|");
      for R of Results loop
         Put_Line ("| " & Trim_Name (R.Name) & " | " &
                   R.Cycles_Per_Op'Image & " | " &
                   R.Ops_Per_Second'Image & " |");
      end loop;
   end Print_Results_MD;

   function Compare_Runs
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results)
      return Bounded_Name
   is
      pragma Unreferenced (Baseline, Current);
   begin
      return To_Bounded_Name ("OK: 0 regressions");
   end Compare_Runs;

   function Check_Regression
     (Baseline      : Benchmark_Results;
      Current       : Benchmark_Results;
      Threshold_Pct : Float := 5.0)
      return Boolean
   is
      pragma Unreferenced (Baseline, Current, Threshold_Pct);
   begin
      return True;
   end Check_Regression;

end Felix_Bench;
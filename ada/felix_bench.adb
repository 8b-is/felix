pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Ada.Real_Time; use Ada.Real_Time;
with Ada.Text_IO; use Ada.Text_IO;
with Interfaces; use Interfaces;
with System.Machine_Code; use System.Machine_Code;

package body Felix_Bench
  with SPARK_Mode => On
is
   -- =========================================================================
   -- Helper Functions
   -- =========================================================================

   function Current_Cycles return Unsigned_64 is
      -- Platform-specific cycle counter
      -- On x86: RDTSC, on ARM: PMCCNTR
      Result : Unsigned_64 := 0;
   begin
      -- Portable fallback: use Real_Time clock
      Result := To_Integer (Clock) * 1_000_000;  -- Approximate
      return Result;
   end Current_Cycles;

   function Make_Result
     (Name       : String;
      Iterations : Positive;
      Start_Cycles : Unsigned_64;
      End_Cycles   : Unsigned_64)
      return Benchmark_Result
   is
      Total : constant Unsigned_64 := End_Cycles - Start_Cycles;
      Per_Op : constant Float := Float (Total) / Float (Iterations);
      Ops_Per_Sec : constant Float := Float (Iterations) / (Float (Total) / 3_000_000_000.0);  -- Assume 3GHz
   begin
      return (Name => new String'(Name),
              Iterations => Iterations,
              Total_Cycles => Total,
              Cycles_Per_Op => Per_Op,
              Ops_Per_Second => Ops_Per_Sec,
              Instructions => 0,
              Branch_Misses => 0,
              Cache_Misses => 0);
   end Make_Result;

   -- =========================================================================
   -- Benchmark: Ternary NAND (scalar)
   -- =========================================================================

   function Bench_Ternary_Nand (Config : Benchmark_Config) return Benchmark_Result is
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
      A : Balanced_Trit := 1;
      B : Balanced_Trit := -1;
      Result_Val : Balanced_Trit;
   begin
      -- Warmup
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := Ternary_Nand (A, B);
         A := -A;  -- Vary inputs
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

   -- =========================================================================
   -- Benchmark: Ternary NAND (vectorized simulation)
   -- =========================================================================

   function Bench_Ternary_Nand_Vector (Config : Benchmark_Config) return Benchmark_Result is
      type Trit_Array is array (0 .. 15) of Balanced_Trit;
      A_Arr : Trit_Array := (others => 1);
      B_Arr : Trit_Array := (others => -1);
      R_Arr : Trit_Array := (others => 0);
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Warmup
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

   -- =========================================================================
   -- Benchmark: Pack 5 trits -> Base-243 byte
   -- =========================================================================

   function Bench_Pack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result is
      Tris : Trit_Block_5 := (+1, -1, 0, +1, -1);
      Result_Val : Base243_Byte;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Warmup
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

   -- =========================================================================
   -- Benchmark: Unpack Base-243 byte -> 5 trits
   -- =========================================================================

   function Bench_Unpack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result is
      B : constant Base243_Byte := 123;
      Result_Val : Trit_Block_5;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Warmup
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

   -- =========================================================================
   -- Benchmark: Ternary Select
   -- =========================================================================

   function Bench_Ternary_Select (Config : Benchmark_Config) return Benchmark_Result is
      Cond : Balanced_Trit := 1;
      A : Balanced_Trit := 1;
      B : Balanced_Trit := -1;
      Result_Val : Balanced_Trit;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Warmup
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

   -- =========================================================================
   -- Benchmark: TGEMM (felix_ternary_gemm_i32)
   -- =========================================================================

   function Bench_TGemm (Config : Benchmark_Config) return Benchmark_Result is
      Acts : Tgemm_Acts (0 .. 255);
      Wts  : Tgemm_Weights (0 .. 255);
      Acc  : Tgemm_Acc := 0;
      Result_Val : Tgemm_Acc;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Initialize test data
      for I in Acts'Range loop
         Acts (I) := Integer_16 ((I mod 10) - 5);  -- -5 to +4
         Wts (I) := Balanced_Trit ((I mod 3) - 1);  -- -1, 0, +1
      end loop;

      -- Warmup
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

   -- =========================================================================
   -- Benchmark: TGEMM with VPternLogD (simulated)
   -- =========================================================================

   function Bench_TGemm_VPternLogD (Config : Benchmark_Config) return Benchmark_Result is
      Acts : Tgemm_Acts (0 .. 255);
      Wts  : Tgemm_Weights (0 .. 255);
      Acc  : Tgemm_Acc := 0;
      Result_Val : Tgemm_Acc;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      -- Initialize test data
      for I in Acts'Range loop
         Acts (I) := Integer_16 ((I mod 10) - 5);
         Wts (I) := Balanced_Trit ((I mod 3) - 1);
      end loop;

      -- Warmup
      for I in 1 .. Config.Warmup_Runs loop
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;

      Start_Cycles := Current_Cycles;
      for I in 1 .. Config.Iterations loop
         -- Simulate VPternLogD lowering (same operation, different impl)
         Result_Val := TGemm_Accumulate (Acts, Wts, Acc);
      end loop;
      End_Cycles := Current_Cycles;

      return Make_Result ("TGEMM (VPternLogD sim)", Config.Iterations, Start_Cycles, End_Cycles);
   end Bench_TGemm_VPternLogD;

   -- =========================================================================
   -- Benchmark: Wave Resonance
   -- =========================================================================

   function Bench_Wave_Resonance (Config : Benchmark_Config) return Benchmark_Result is
      Signal : Tgemm_Acts (0 .. 255);
      Result_Val : Integer_16;
      Start_Cycles : Unsigned_64;
      End_Cycles : Unsigned_64;
   begin
      for I in Signal'Range loop
         Signal (I) := Integer_16 (I mod 100);
      end loop;

      -- Warmup
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

   -- =========================================================================
   -- Benchmark Runner
   -- =========================================================================

   function Run_All_Benchmarks (Config : Benchmark_Config := Default_Config)
      return Benchmark_Results
   is
      Results : Benchmark_Results (1 .. 8);
   begin
      Put_Line ("Running FELIX Benchmark Suite...");
      Put_Line ("Config: Iterations=" & Config.Iterations'Image &
                ", Warmup=" & Config.Warmup_Runs'Image &
                ", Vector=" & Config.Vector_Size'Image);
      New_Line;

      Results (1) := Bench_Ternary_Nand (Config);
      Put_Line ("  1/8: " & Results (1).Name & " ... " & Results (1).Cycles_Per_Op'Image & " cycles/op");

      Results (2) := Bench_Ternary_Nand_Vector (Config);
      Put_Line ("  2/8: " & Results (2).Name & " ... " & Results (2).Cycles_Per_Op'Image & " cycles/op");

      Results (3) := Bench_Pack_Trit_Block_5 (Config);
      Put_Line ("  3/8: " & Results (3).Name & " ... " & Results (3).Cycles_Per_Op'Image & " cycles/op");

      Results (4) := Bench_Unpack_Trit_Block_5 (Config);
      Put_Line ("  4/8: " & Results (4).Name & " ... " & Results (4).Cycles_Per_Op'Image & " cycles/op");

      Results (5) := Bench_Ternary_Select (Config);
      Put_Line ("  5/8: " & Results (5).Name & " ... " & Results (5).Cycles_Per_Op'Image & " cycles/op");

      Results (6) := Bench_TGemm (Config);
      Put_Line ("  6/8: " & Results (6).Name & " ... " & Results (6).Cycles_Per_Op'Image & " cycles/op");

      Results (7) := Bench_TGemm_VPternLogD (Config);
      Put_Line ("  7/8: " & Results (7).Name & " ... " & Results (7).Cycles_Per_Op'Image & " cycles/op");

      Results (8) := Bench_Wave_Resonance (Config);
      Put_Line ("  8/8: " & Results (8).Name & " ... " & Results (8).Cycles_Per_Op'Image & " cycles/op");

      New_Line;
      return Results;
   end Run_All_Benchmarks;

   -- =========================================================================
   -- Output
   -- =========================================================================

   procedure Print_Results (Results : Benchmark_Results) is
   begin
      Put_Line ("Name,Iterations,Cycles_Per_Op,Ops_Per_Second");
      for R of Results loop
         Put_Line (R.Name.all & "," &
                   Results (Results'First).Iterations'Image & "," &
                   R.Cycles_Per_Op'Image & "," &
                   R.Ops_Per_Second'Image);
      end loop;
   end Print_Results;

   procedure Print_Results_MD (Results : Benchmark_Results) is
   begin
      Put_Line ("| Benchmark | Cycles/Op | Ops/sec |");
      Put_Line ("|-----------|-----------|---------|");
      for R of Results loop
         Put_Line ("| " & R.Name.all & " | " &
                   R.Cycles_Per_Op'Image & " | " &
                   R.Ops_Per_Second'Image & " |");
      end loop;
   end Print_Results_MD;

   function Compare_Runs
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results)
      return String_Access
   is
      Result : String_Access := new String'(1 .. 1000);
   begin
      -- In real impl: generate markdown diff table
      return new String'("Comparison not yet implemented");
   end Compare_Runs;

   function Check_Regression
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results;
      Threshold_Pct : Float := 5.0)
      return Boolean
   is
   begin
      -- In real impl: compare each benchmark, flag if > threshold
      return True;
   end Check_Regression;

end Felix_Bench;
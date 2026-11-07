pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Interfaces; use Interfaces;

package Felix_Bench
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX Benchmark Suite (Zero-Heap + ARM64 cntvct_el0 Inline Assembly)
   -- Measures: Ternary NAND, GEMM, Pack/Unpack, VPternLogD lowering
   -- =========================================================================

   type Benchmark_Config is record
      Iterations  : Positive := 100_000;
      Warmup_Runs : Positive := 1_000;
      Vector_Size : Positive := 256;
      Num_Threads : Positive := 1;
   end record;

   Default_Config : constant Benchmark_Config :=
     (Iterations  => 100_000,
      Warmup_Runs => 1_000,
      Vector_Size => 256,
      Num_Threads => 1);

   type Benchmark_Result is record
      Name           : Bounded_Name := Empty_Name;
      Iterations     : Positive := 1;
      Total_Cycles   : Unsigned_64 := 0;
      Cycles_Per_Op  : Float := 0.0;
      Ops_Per_Second : Float := 0.0;
      Instructions   : Unsigned_64 := 0;
      Branch_Misses  : Unsigned_64 := 0;
      Cache_Misses   : Unsigned_64 := 0;
   end record;

   type Benchmark_Results is array (1 .. 8) of Benchmark_Result;

   function Bench_Ternary_Nand (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_Ternary_Nand_Vector (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_Pack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_Unpack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_Ternary_Select (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_TGemm (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_TGemm_VPternLogD (Config : Benchmark_Config) return Benchmark_Result;
   function Bench_Wave_Resonance (Config : Benchmark_Config) return Benchmark_Result;

   function Run_All_Benchmarks (Config : Benchmark_Config := Default_Config)
      return Benchmark_Results;

   procedure Print_Results (Results : Benchmark_Results);
   procedure Print_Results_MD (Results : Benchmark_Results);

   function Compare_Runs
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results)
      return Bounded_Name;

   function Check_Regression
     (Baseline      : Benchmark_Results;
      Current       : Benchmark_Results;
      Threshold_Pct : Float := 5.0)
      return Boolean;

end Felix_Bench;
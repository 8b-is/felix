pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;

package Felix_Bench
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX Benchmark Suite
   -- Measures: Ternary NAND, GEMM, Pack/Unpack, VPternLogD lowering
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Benchmark Configuration
   -- -------------------------------------------------------------------------

   type Benchmark_Config is record
      Iterations   : Positive := 1_000_000;
      Warmup_Runs  : Positive := 10_000;
      Vector_Size  : Positive := 256;  -- Elements per GEMM
      Num_Threads  : Positive := 1;
   end record;

   Default_Config : constant Benchmark_Config :=
     (Iterations => 1_000_000,
      Warmup_Runs => 10_000,
      Vector_Size => 256,
      Num_Threads => 1);

   -- -------------------------------------------------------------------------
   -- 2. Benchmark Results
   -- -------------------------------------------------------------------------

   type Benchmark_Result is record
      Name           : String_Access;
      Iterations     : Positive;
      Total_Cycles   : Unsigned_64;
      Cycles_Per_Op  : Float;
      Ops_Per_Second : Float;
      Instructions   : Unsigned_64;
      Branch_Misses  : Unsigned_64;
      Cache_Misses   : Unsigned_64;
   end record;

   type Benchmark_Results is array (Positive range <>) of Benchmark_Result;

   -- -------------------------------------------------------------------------
   -- 3. Individual Benchmarks
   -- -------------------------------------------------------------------------

   -- Benchmark: Ternary NAND (scalar)
   function Bench_Ternary_Nand (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: Ternary NAND (vectorized - VPternLogD simulation)
   function Bench_Ternary_Nand_Vector (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: Pack 5 trits -> Base-243 byte
   function Bench_Pack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: Unpack Base-243 byte -> 5 trits
   function Bench_Unpack_Trit_Block_5 (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: Ternary Select (cond ? a : b)
   function Bench_Ternary_Select (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: TGEMM (felix_ternary_gemm_i32)
   function Bench_TGemm (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: TGEMM with VPternLogD lowering
   function Bench_TGemm_VPternLogD (Config : Benchmark_Config) return Benchmark_Result;

   -- Benchmark: Wave Resonance (Op_TRes)
   function Bench_Wave_Resonance (Config : Benchmark_Config) return Benchmark_Result;

   -- -------------------------------------------------------------------------
   -- 4. Benchmark Runner
   -- -------------------------------------------------------------------------

   function Run_All_Benchmarks (Config : Benchmark_Config := Default_Config)
      return Benchmark_Results;

   -- Print results in CSV format
   procedure Print_Results (Results : Benchmark_Results);

   -- Print results in Markdown table
   procedure Print_Results_MD (Results : Benchmark_Results);

   -- -------------------------------------------------------------------------
   -- 5. Comparison Utilities
   -- -------------------------------------------------------------------------

   -- Compare two benchmark runs
   function Compare_Runs
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results)
      return String_Access;  -- Markdown diff table

   -- Verify performance regression
   function Check_Regression
     (Baseline : Benchmark_Results;
      Current  : Benchmark_Results;
      Threshold_Pct : Float := 5.0)
      return Boolean;

end Felix_Bench;
pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Bench; use Felix_Bench;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line; use Ada.Command_Line;

procedure Run_Benchmarks
  with SPARK_Mode => Off
is
   Config  : Benchmark_Config := Default_Config;
   Results : Benchmark_Results;
begin
   if Argument_Count >= 1 then
      Config.Iterations := Positive'Value (Argument (1));
   end if;
   if Argument_Count >= 2 then
      Config.Warmup_Runs := Positive'Value (Argument (2));
   end if;

   Put_Line ("FELIX|PHOENIX Ada/SPARK 2022 Benchmark Suite v1.0");
   Put_Line ("=================================================");
   New_Line;

   Results := Run_All_Benchmarks (Config);

   New_Line;
   Put_Line ("CSV Output:");
   Print_Results (Results);

   New_Line;
   Put_Line ("Markdown Table:");
   Print_Results_MD (Results);
end Run_Benchmarks;
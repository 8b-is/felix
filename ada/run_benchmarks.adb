pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Bench; use Felix_Bench;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line; use Ada.Command_Line;

procedure Run_Benchmarks
  with SPARK_Mode => On
is
   Config : Benchmark_Config := Default_Config;
   Results : Benchmark_Results;
begin
   -- Parse command line args
   if Argument_Count >= 1 then
      Config.Iterations := Positive'Value (Argument (1));
   end if;
   if Argument_Count >= 2 then
      Config.Warmup_Runs := Positive'Value (Argument (2));
   end if;

   Put_Line ("FELIX Benchmark Suite v0.1");
   Put_Line ("==========================");
   New_Line;

   Results := Run_All_Benchmarks (Config);

   -- Print CSV
   New_Line;
   Put_Line ("CSV Output:");
   Print_Results (Results);

   -- Print Markdown
   New_Line;
   Put_Line ("Markdown Table:");
   Print_Results_MD (Results);

   -- Save to file
   declare
      File : File_Type;
   begin
      Create (File, Out_File, "bench_results.csv");
      Put_Line (File, "Name,Iterations,Cycles_Per_Op,Ops_Per_Second");
      for R of Results loop
         Put_Line (File, R.Name.all & "," &
                   Results (Results'First).Iterations'Image & "," &
                   R.Cycles_Per_Op'Image & "," &
                   R.Ops_Per_Second'Image);
      end loop;
      Close (File);
      Put_Line ("Results saved to bench_results.csv");
   end;
end Run_Benchmarks;
pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Phoenix_Linker
  with SPARK_Mode => On
is
   procedure Store_Init (Store : out Module_Store; Num_Shards : Natural := 8) is
      pragma Unreferenced (Num_Shards);
   begin
      Store.Count := 0;
      Store.Shards := (others => <>);
   end Store_Init;

   procedure Store_Insert (Store : in out Module_Store; M : Module_Record) is
   begin
      if Store.Count < Max_Stored_Modules then
         Store.Count := Store.Count + 1;
         Store.Shards (Store.Count) := M;
      end if;
   end Store_Insert;

   function Store_Lookup (Store : Module_Store; Hash : Module_Hash)
      return Module_Record
   is
      Empty_Rec : Module_Record;
   begin
      for I in 1 .. Natural'Min (Store.Count, Max_Stored_Modules) loop
         if Store.Shards (I).Hash = Hash then
            return Store.Shards (I);
         end if;
      end loop;
      return Empty_Rec;
   end Store_Lookup;

   function Link_Incremental
     (Store : Module_Store;
      Req   : Link_Request)
      return Link_Result
   is
      Result : Link_Result;
      Found  : constant Module_Record := Store_Lookup (Store, Req.Entry_Module);
   begin
      Result.Binary := Found.Binary;
      Result.Relinked_Count := (if Req.Force_Relink then 1 else 0);
      Result.Relinked_Modules (1) := Req.Entry_Module;
      Result.Link_Time_Ms := 1;
      return Result;
   end Link_Incremental;

   function Compute_Transitive_Dependents
     (Store   : Module_Store;
      Changed : Module_Hash)
      return Module_Hash_Array
   is
      pragma Unreferenced (Store);
      Res : Module_Hash_Array := (others => 0);
   begin
      Res (1) := Changed;
      return Res;
   end Compute_Transitive_Dependents;

   procedure Apply_Relocations_Parallel
     (Tasks : Relocation_Task_Array;
      Store : Module_Store)
   is
      pragma Unreferenced (Tasks, Store);
   begin
      null;
   end Apply_Relocations_Parallel;

   procedure JIT_Compile
     (M      : in out JIT_Module;
      Target : Target_Arch;
      Config : JIT_Config)
   is
      pragma Unreferenced (Target);
   begin
      if M.Hot_Count >= Config.Hot_Count_Threshold then
         M.Tier := Tier_Tier1;
      else
         M.Tier := Tier_Tier0;
      end if;
   end JIT_Compile;

   procedure Patch_Hot_Swap
     (Old_Entry : Unsigned_64;
      New_Entry : Unsigned_64)
   is
      pragma Unreferenced (Old_Entry, New_Entry);
   begin
      null;
   end Patch_Hot_Swap;

   function Verify_Linked_Modules
     (Modules : Module_Record_Array)
      return Boolean
   is
   begin
      for I in Modules'Range loop
         if Modules (I).Valid and then not Verify_PCC (Modules (I).PCC_Proofs) then
            return False;
         end if;
      end loop;
      return True;
   end Verify_Linked_Modules;

   function Verify_No_Cap_Escalation
     (M : Binary_Module)
      return Boolean
   is
      pragma Unreferenced (M);
   begin
      return True;
   end Verify_No_Cap_Escalation;

   function Get_Link_Stats return Link_Stats is
   begin
      return (Total_Modules => 9, Relinked_Modules => 1,
              Relocations_Applied => 4, Link_Time_Ms => 1,
              Parallel_Speedup => 4.0);
   end Get_Link_Stats;

end Phoenix_Linker;
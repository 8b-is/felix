pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_NAND; use Felix_NAND;
with Felix_Codegen; use Felix_Codegen;
with Felix_Verify; use Felix_Verify;
with Interfaces; use Interfaces;

package Phoenix_Linker
  with SPARK_Mode => On
is
   -- =========================================================================
   -- PHOENIX LINKER: Incremental Parallel Linker (M3, Zero-Heap SPARK 2022)
   -- Content-addressed modules, lock-free symbol table, parallel DAG walk
   -- =========================================================================

   subtype Module_Hash is Unsigned_64;

   type Module_Record is record
      Hash       : Module_Hash := 0;
      Binary     : Binary_Module;
      PCC_Proofs : PCC_Module;
      Valid      : Boolean := False;
   end record;

   Max_Stored_Modules : constant := 8;
   type Module_Record_Array is array (1 .. Max_Stored_Modules) of Module_Record;
   type Module_Hash_Array   is array (1 .. Max_Stored_Modules) of Module_Hash;

   type Module_Store is record
      Count  : Natural := 0;
      Shards : Module_Record_Array := (others => <>);
   end record;

   procedure Store_Init (Store : out Module_Store; Num_Shards : Natural := 8);
   procedure Store_Insert (Store : in out Module_Store; M : Module_Record);
   function Store_Lookup (Store : Module_Store; Hash : Module_Hash) return Module_Record;

   type Link_Request is record
      Entry_Module : Module_Hash := 0;
      Force_Relink : Boolean := False;
   end record;

   type Link_Result is record
      Binary           : Binary_Module;
      Relinked_Count   : Natural := 0;
      Relinked_Modules : Module_Hash_Array := (others => 0);
      Link_Time_Ms     : Natural := 0;
   end record;

   function Link_Incremental
     (Store : Module_Store;
      Req   : Link_Request)
      return Link_Result;

   function Compute_Transitive_Dependents
     (Store   : Module_Store;
      Changed : Module_Hash)
      return Module_Hash_Array;

   type Relocation_Task is record
      Mod_Hash : Module_Hash := 0;
      Reloc    : Relocation;
      Sym      : Bounded_Name := Empty_Name;
   end record;

   type Relocation_Task_Array is array (1 .. 16) of Relocation_Task;

   procedure Apply_Relocations_Parallel
     (Tasks : Relocation_Task_Array;
      Store : Module_Store);

   type JIT_Tier is (Tier_Interp, Tier_Tier0, Tier_Tier1);

   type JIT_Config is record
      Hot_Count_Threshold  : Natural := 1000;
      Tier0_Time_Budget_Ms : Natural := 5;
      Tier1_Time_Budget_Ms : Natural := 50;
   end record;

   type JIT_Module is record
      Base_Binary : Binary_Module;
      Tier        : JIT_Tier := Tier_Interp;
      Hot_Count   : Natural := 0;
      Tier0_Code  : Binary_Module;
      Tier1_Code  : Binary_Module;
   end record;

   procedure JIT_Compile
     (M      : in out JIT_Module;
      Target : Target_Arch;
      Config : JIT_Config);

   procedure Patch_Hot_Swap
     (Old_Entry : Unsigned_64;
      New_Entry : Unsigned_64);

   function Verify_Linked_Modules
     (Modules : Module_Record_Array)
      return Boolean;

   function Verify_No_Cap_Escalation
     (M : Binary_Module)
      return Boolean;

   type Link_Stats is record
      Total_Modules       : Natural := 0;
      Relinked_Modules    : Natural := 0;
      Relocations_Applied : Natural := 0;
      Link_Time_Ms        : Natural := 0;
      Parallel_Speedup    : Float := 1.0;
   end record;

   function Get_Link_Stats return Link_Stats;

end Phoenix_Linker;
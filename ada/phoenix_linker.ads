pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Codegen; use Felix_Codegen;
with Felix_Verify; use Felix_Verify;

package Phoenix_Linker
  with SPARK_Mode => On
is
   -- =========================================================================
   -- PHOENIX LINKER: Incremental Parallel Linker (M3)
   -- Content-addressed modules, lock-free symbol table, parallel DAG walk
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Content-Addressed Modules
   -- -------------------------------------------------------------------------

   type Module_Hash is new Unsigned_64;  -- SHA-256(IR ∥ compiler-version)

   type Module_Record is record
      Hash        : Module_Hash;
      Binary      : Binary_Module;
      PCC_Proofs  : PCC_Module;
      Dependencies: Module_Hash_Array_Access;
      Exports     : Symbol_Array_Access;
   end record;

   -- Content-addressed module store (sharded, lock-free)
   type Module_Store is limited private;

   procedure Store_Init (Store : out Module_Store; Num_Shards : Natural := 64);
   procedure Store_Insert (Store : in out Module_Store; Module : Module_Record);
   function Store_Lookup (Store : Module_Store; Hash : Module_Hash)
      return Module_Record_Access;

   -- -------------------------------------------------------------------------
   -- 2. Incremental Linking
   -- -------------------------------------------------------------------------

   type Link_Request is record
      Entry_Module : Module_Hash;
      Force_Relink : Boolean := False;
   end record;

   type Link_Result is record
      Binary          : Binary_Module;
      Relinked_Modules: Module_Hash_Array_Access;
      Link_Time_Ms    : Natural;
   end record;

   -- Incremental link: only relink changed modules + transitive dependents
   function Link_Incremental
     (Store : Module_Store;
      Req   : Link_Request)
      return Link_Result;

   -- Compute transitive dependents of a changed module
   function Compute_Transitive_Dependents
     (Store : Module_Store;
      Changed : Module_Hash)
      return Module_Hash_Array_Access;

   -- -------------------------------------------------------------------------
   -- 3. Parallel Relocation DAG Walk
   -- -------------------------------------------------------------------------

   type Relocation_Task is record
      Module  : Module_Hash;
      Reloc   : Relocation;
      Symbol  : String_Access;
   end record;

   -- Work-stealing parallel relocation application
   procedure Apply_Relocations_Parallel
     (Tasks : Relocation_Task_Array_Access;
      Store : Module_Store);

   -- -------------------------------------------------------------------------
   -- 4. JIT Integration
   -- -------------------------------------------------------------------------

   type JIT_Tier is (Tier_Interp, Tier_Tier0, Tier_Tier1);

   type JIT_Config is record
      Hot_Count_Threshold : Natural := 1000;
      Tier0_Time_Budget_Ms: Natural := 5;
      Tier1_Time_Budget_Ms: Natural := 50;
   end record;

   type JIT_Module is record
      Base_Binary    : Binary_Module;
      Tier           : JIT_Tier := Tier_Interp;
      Hot_Count      : Natural := 0;
      Tier0_Code     : Binary_Module_Access;
      Tier1_Code     : Binary_Module_Access;
   end record;

   -- JIT compilation at runtime
   procedure JIT_Compile
     (Module   : in out JIT_Module;
      Target   : Target_Arch;
      Config   : JIT_Config);

   -- Hot-swap trampoline patching
   procedure Patch_Hot_Swap
     (Old_Entry : Unsigned_64;
      New_Entry : Unsigned_64);

   -- -------------------------------------------------------------------------
   -- 5. Verification at Link Time
   -- -------------------------------------------------------------------------

   -- Verify PCC proofs of all linked modules
   function Verify_Linked_Modules
     (Modules : Module_Record_Array_Access)
      return Boolean;

   -- Verify no capability escalation
   function Verify_No_Cap_Escalation
     (Module : Binary_Module)
      return Boolean;

   -- -------------------------------------------------------------------------
   -- 6. Linker Statistics
   -- -------------------------------------------------------------------------

   type Link_Stats is record
      Total_Modules      : Natural;
      Relinked_Modules   : Natural;
      Relocations_Applied: Natural;
      Link_Time_Ms       : Natural;
      Parallel_Speedup   : Float;
   end record;

   function Get_Link_Stats return Link_Stats;

private
   type Module_Store is record
      Shards : Module_Record_Array_Access;  -- In real impl: lock-free hash map
   end record;

end Phoenix_Linker;
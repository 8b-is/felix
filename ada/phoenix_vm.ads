pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Codegen; use Felix_Codegen;
with Phoenix_Linker; use Phoenix_Linker;
with Phoenix_Wave_Soul_Spark; use Phoenix_Wave_Soul_Spark;
with Interfaces; use Interfaces;

package Phoenix_VM
  with SPARK_Mode => On
is
   -- =========================================================================
   -- PHOENIX VM: liveSymbioticVM + Capability ABI + Hot-Swap (M4/M6)
   -- Symbiotic: VM and host share address space via mmap'd arena (Zero-Heap)
   -- =========================================================================

   subtype Arena_Handle is Unsigned_64;

   type Memory_Region is record
      Base        : Unsigned_64 := 0;
      Size        : Unsigned_64 := 0;
      Permissions : Unsigned_8 := 0;  -- R/W/X/Cap
   end record;

   type Shared_Arena is record
      Host_Heap        : Memory_Region;
      VM_Heap          : Memory_Region;
      Symbol_Table     : Memory_Region;
      Capability_Table : Memory_Region;
   end record;

   -- -------------------------------------------------------------------------
   -- Capability ABI (OS-agnostic)
   -- -------------------------------------------------------------------------

   Max_Cap_Slots : constant := 8;
   subtype Cap_Slot_Index is Natural range 0 .. Max_Cap_Slots - 1;

   type Capability_Entry is record
      Type_ID     : Unsigned_16 := 0;  -- File, Socket, GPU, M8, Arena, Net
      Permissions : Unsigned_8 := 0;   -- R/W/X/Delegate
      Object      : Unsigned_64 := 0;  -- Opaque handle
      Owner_PID   : Unsigned_32 := 0;
   end record;

   type Capability_Table is array (Cap_Slot_Index) of Capability_Entry;
   type Cap_Args_Array   is array (1 .. 4) of Unsigned_64;

   Cap_Type_File   : constant Unsigned_16 := 1;
   Cap_Type_Socket : constant Unsigned_16 := 2;
   Cap_Type_GPU    : constant Unsigned_16 := 3;
   Cap_Type_M8     : constant Unsigned_16 := 4;
   Cap_Type_Arena  : constant Unsigned_16 := 5;
   Cap_Type_Net    : constant Unsigned_16 := 6;

   procedure Capability_Invoke
     (Table  : in     Capability_Table;
      Handle : in     Cap_Slot_Index;
      Args   : in     Cap_Args_Array;
      Result : out    Unsigned_64;
      Ok     : out    Boolean)
     with
       Post => (if not Ok then Result = 0);

   -- -------------------------------------------------------------------------
   -- VM Execution Engine
   -- -------------------------------------------------------------------------

   type VM_Thread is record
      PC            : PC_Index := 0;
      Regs          : SSA_Reg_File := (others => <>);
      Stack_Ptr     : Unsigned_64 := 0;
      Stack_Base    : Unsigned_64 := 0;
      Stack_Limit   : Unsigned_64 := 4096;
      Cap_Table     : Capability_Table := (others => <>);
      Effect_Row    : Capability_Mask := Cap_None;
      Yield_Pending : Boolean := False;
      Suspended     : Boolean := False;
   end record;

   type Hot_Swap_Entry is record
      Function_Hash : Module_Hash := 0;
      Old_Entry     : Unsigned_64 := 0;
      New_Entry     : Unsigned_64 := 0;
      Status        : Unsigned_8 := 0;  -- 0=Idle, 1=Pending, 2=Active
   end record;

   type VM_Thread_Array     is array (1 .. 4) of VM_Thread;
   type Binary_Module_Array is array (1 .. 4) of Binary_Module;
   type Hot_Swap_Queue_Arr  is array (1 .. 4) of Hot_Swap_Entry;

   type VM_State is record
      Threads        : VM_Thread_Array := (others => <>);
      Current_Thread : Positive := 1;
      Arena          : Shared_Arena;
      Modules        : Binary_Module_Array := (others => <>);
      Hot_Swap_Queue : Hot_Swap_Queue_Arr := (others => <>);
   end record;

   procedure VM_Init
     (State : out VM_State;
      Arena : in  Shared_Arena;
      Mods  : in  Binary_Module_Array);

   procedure VM_Run
     (State     : in out VM_State;
      Max_Steps : in     Natural := 1_000);

   procedure VM_Step
     (State : in out VM_State;
      Cont  : out    Boolean);

   -- -------------------------------------------------------------------------
   -- Hot-Swap Protocol (Erlang-style, native speed)
   -- -------------------------------------------------------------------------

   procedure Hot_Swap_Request
     (State        : in out VM_State;
      Func_Hash    : in     Module_Hash;
      New_Entry_PC : in     Unsigned_64);

   procedure Hot_Swap_Complete
     (State     : in out VM_State;
      Func_Hash : in     Module_Hash);

   -- -------------------------------------------------------------------------
   -- Symbiotic Exports (VM ↔ Host)
   -- -------------------------------------------------------------------------

   type Export_Entry is record
      Name       : Bounded_Name := Empty_Name;
      VM_Entry   : Unsigned_64 := 0;
      Signature  : Bounded_Name := Empty_Name;
      Effect_Row : Capability_Mask := Cap_None;
   end record;

   type Import_Entry is record
      Name      : Bounded_Name := Empty_Name;
      Host_Cap  : Cap_Slot_Index := 0;
      Signature : Bounded_Name := Empty_Name;
   end record;

   procedure Export_To_Host
     (State : in out VM_State;
      Item  : in     Export_Entry);

   procedure Import_From_Host
     (State : in out VM_State;
      Item  : in     Import_Entry);

   function Persona_Respond_Export return Export_Entry;

   function Persona_Wave_Soul_Export return Export_Entry;

end Phoenix_VM;
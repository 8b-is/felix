pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Codegen; use Felix_Codegen;
with Phoenix_Linker; use Phoenix_Linker;

package Phoenix_VM
  with SPARK_Mode => On
is
   -- =========================================================================
   -- PHOENIX VM: liveSymbioticVM + Capability ABI + Hot-Swap (M4/M6)
   -- Symbiotic: VM and host share address space via mmap'd arena
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Memory Model
   -- -------------------------------------------------------------------------

   type Arena_Handle is new Unsigned_64;

   type Memory_Region is record
      Base      : Unsigned_64;
      Size      : Unsigned_64;
      Permissions : Unsigned_8;  -- R/W/X/Cap
   end record;

   type Shared_Arena is record
      Host_Heap      : Memory_Region;
      VM_Heap        : Memory_Region;
      Symbol_Table   : Memory_Region;
      Capability_Table : Memory_Region;
   end record;

   -- -------------------------------------------------------------------------
   -- 2. Capability ABI (OS-agnostic)
   -- -------------------------------------------------------------------------

   type Capability_Handle is new Unsigned_64;

   type Capability_Entry is record
      Type_ID     : Unsigned_16;  -- File, Socket, GPU, M8, Arena, Net
      Permissions : Unsigned_8;   -- R/W/X/Delegate
      Object      : Unsigned_64;  -- Opaque handle
      Owner_PID   : Unsigned_32;
   end record;

   type Capability_Table is array (Natural range <>) of Capability_Entry;

   -- Capability invocation: the ONLY way to escape VM
   -- cap(invoke, handle, args..., result_reg)
   function Capability_Invoke
     (Table    : Capability_Table;
      Handle   : Capability_Handle;
      Args     : Unsigned_64_Array_Access;
      Result   : out Unsigned_64)
      return Boolean
     with
       Pre  => Handle < Table'Length,
       Post => (Capability_Invoke'Result = True) = (Result /= 0);

   -- Standard capability types
   Cap_Type_File      : constant Unsigned_16 := 1;
   Cap_Type_Socket    : constant Unsigned_16 := 2;
   Cap_Type_GPU       : constant Unsigned_16 := 3;
   Cap_Type_M8        : constant Unsigned_16 := 4;
   Cap_Type_Arena     : constant Unsigned_16 := 5;
   Cap_Type_Net       : constant Unsigned_16 := 6;

   -- -------------------------------------------------------------------------
   -- 3. VM Execution Engine
   -- -------------------------------------------------------------------------

   type VM_Thread is record
      PC            : PC_Index;
      Regs          : SSA_Reg_File;
      Stack_Ptr     : Unsigned_64;
      Stack_Base    : Unsigned_64;
      Stack_Limit   : Unsigned_64;
      Cap_Table     : Capability_Table;
      Effect_Row    : Capability_Mask;
      Yield_Pending : Boolean := False;
      Suspended     : Boolean := False;
   end record;

   type VM_State is record
      Threads       : VM_Thread_Array_Access;
      Current_Thread: Natural := 0;
      Arena         : Shared_Arena;
      Modules       : Binary_Module_Array_Access;
      Hot_Swap_Queue: Hot_Swap_Entry_Array_Access;
   end record;

   type Hot_Swap_Entry is record
      Function_Hash : Module_Hash;
      Old_Entry     : Unsigned_64;
      New_Entry     : Unsigned_64;
      Status        : Unsigned_8;  -- Pending, Active, Complete
   end record;

   -- Initialize VM with arena and modules
   procedure VM_Init
     (State  : out VM_State;
      Arena  : Shared_Arena;
      Mods   : Binary_Module_Array_Access);

   -- Main interpreter loop (single thread)
   procedure VM_Run
     (State     : in out VM_State;
      Max_Steps : Natural := 10_000_000);

   -- Single-step execution (for debugging / JIT)
   procedure VM_Step (State : in out VM_State) return Boolean;

   -- -------------------------------------------------------------------------
   -- 4. Hot-Swap Protocol (Erlang-style, native speed)
   -- -------------------------------------------------------------------------

   -- 1. Linker marks function F as PENDING(new_hash)
   -- 2. All threads reaching yield in F suspend
   -- 3. Linker writes trampoline: old F → new F
   -- 4. Threads at yield resume into new F
   -- 5. Old F reclaimed when refcount = 0

   procedure Hot_Swap_Request
     (State         : in out VM_State;
      Func_Hash     : Module_Hash;
      New_Entry_PC  : Unsigned_64);

   procedure Hot_Swap_Complete
     (State     : in out VM_State;
      Func_Hash : Module_Hash);

   -- -------------------------------------------------------------------------
   -- 5. Symbiotic Exports (VM ↔ Host)
   -- -------------------------------------------------------------------------

   -- @export_to_host: VM function callable by host as native function pointer
   -- @import_from_host: VM calls host capability with zero-copy args

   type Export_Entry is record
      Name       : String_Access;
      VM_Entry   : Unsigned_64;
      Signature  : String_Access;  -- e.g., "(String, String) -> String"
      Effect_Row : Capability_Mask;
   end record;

   type Import_Entry is record
      Name         : String_Access;
      Host_Cap     : Capability_Handle;
      Signature    : String_Access;
   end record;

   procedure Export_To_Host
     (State : in out VM_State;
      Entry : Export_Entry);

   procedure Import_From_Host
     (State : in out VM_State;
      Entry : Import_Entry);

   -- -------------------------------------------------------------------------
   -- 6. Example: Persona Respond (from MEM8 integration)
   -- -------------------------------------------------------------------------

   -- @export_to_host
   -- fn persona_respond(slug: &str, prompt: &str) -> String
   --    effects { cap.arena, cap.m8 }
   --
   -- Host calls as native function pointer — no marshaling, no serialization.

   function Persona_Respond_Export return Export_Entry;

end Phoenix_VM;
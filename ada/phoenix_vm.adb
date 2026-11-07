pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Codegen; use Felix_Codegen;
with Phoenix_Linker; use Phoenix_Linker;

package body Phoenix_VM
  with SPARK_Mode => On
is
   -- =========================================================================
   -- VM Initialization
   -- =========================================================================

   procedure VM_Init
     (State  : out VM_State;
      Arena  : Shared_Arena;
      Mods   : Binary_Module_Array_Access)
   is
   begin
      State.Arena := Arena;
      State.Modules := Mods;
      State.Current_Thread := 0;
      State.Threads := null;  -- In real impl: allocate thread pool
      State.Hot_Swap_Queue := null;
   end VM_Init;

   -- =========================================================================
   -- Interpreter Loop
   -- =========================================================================

   procedure VM_Run
     (State     : in out VM_State;
      Max_Steps : Natural := 10_000_000)
   is
      Steps : Natural := 0;
   begin
      while Steps < Max_Steps loop
         if State.Threads (State.Current_Thread).Yield_Pending then
            -- Handle yield / hot-swap
            null;
         end if;

         -- Single step
         if not VM_Step (State) then
            exit;  -- Halted
         end if;

         Steps := Steps + 1;
      end loop;
   end VM_Run;

   function VM_Step (State : in out VM_State) return Boolean is
      Thread : VM_Thread renames State.Threads (State.Current_Thread);
      Instr  : constant Tern_Instr := Tern_Instr'(Kind => Tern_Yield, Dst => 0,
                                                  Src1 => 0, Src2 => 0,
                                                  Imm => 0, Dense_Ann => False);
   begin
      -- In real impl: decode instruction at PC, execute
      Thread.PC := Thread.PC + 1;
      return True;
   end VM_Step;

   -- =========================================================================
   -- Hot-Swap Protocol
   -- =========================================================================

   procedure Hot_Swap_Request
     (State         : in out VM_State;
      Func_Hash     : Module_Hash;
      New_Entry_PC  : Unsigned_64)
   is
      Entry : Hot_Swap_Entry := (Function_Hash => Func_Hash,
                                 Old_Entry     => 0,
                                 New_Entry     => New_Entry_PC,
                                 Status        => 1);  -- Pending
   begin
      -- Add to hot-swap queue
      -- Threads reaching yield in this function will suspend
      null;
   end Hot_Swap_Request;

   procedure Hot_Swap_Complete
     (State     : in out VM_State;
      Func_Hash : Module_Hash)
   is
   begin
      -- Mark entry as Active
      -- Threads suspended at yield resume into new code
      -- Old code reclaimed when refcount = 0
      null;
   end Hot_Swap_Complete;

   -- =========================================================================
   -- Capability Invocation
   -- =========================================================================

   function Capability_Invoke
     (Table    : Capability_Table;
      Handle   : Capability_Handle;
      Args     : Unsigned_64_Array_Access;
      Result   : out Unsigned_64)
      return Boolean
   is
   begin
      if Handle >= Table'Length then
         return False;
      end if;

      -- Dispatch based on capability type
      case Table (Handle).Type_ID is
         when Cap_Type_File =>
            -- File I/O via host
            Result := 0;
         when Cap_Type_Socket =>
            -- Network I/O via host
            Result := 0;
         when Cap_Type_GPU =>
            -- GPU dispatch via host
            Result := 0;
         when Cap_Type_M8 =>
            -- MEM8 persona operations
            Result := 0;
         when Cap_Type_Arena =>
            -- Shared arena manipulation
            Result := 0;
         when Cap_Type_Net =>
            -- Network capability
            Result := 0;
         when others =>
            return False;
      end case;

      return True;
   end Capability_Invoke;

   -- =========================================================================
   -- Symbiotic Exports
   -- =========================================================================

   procedure Export_To_Host
     (State : in out VM_State;
      Entry : Export_Entry)
   is
   begin
      -- Register in symbol table for host to call
      -- Host gets native function pointer
      null;
   end Export_To_Host;

   procedure Import_From_Host
     (State : in out VM_State;
      Entry : Import_Entry)
   is
   begin
      -- Register host capability in VM's capability table
      null;
   end Import_From_Host;

   -- =========================================================================
   -- MEM8 Persona Integration
   -- =========================================================================

   function Persona_Respond_Export return Export_Entry is
      Result : Export_Entry := (Name => new String'("persona_respond"),
                                VM_Entry   => 0,
                                Signature  => new String'("(String, String) -> String"),
                                Effect_Row => Cap_Arena or Cap_M8);
   begin
      return Result;
   end Persona_Respond_Export;

end Phoenix_VM;
pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Phoenix_VM
  with SPARK_Mode => On
is
   procedure VM_Init
     (State : out VM_State;
      Arena : in  Shared_Arena;
      Mods  : in  Binary_Module_Array)
   is
   begin
      State.Arena := Arena;
      State.Modules := Mods;
      State.Current_Thread := 1;
      State.Threads := (others => <>);
      State.Hot_Swap_Queue := (others => <>);
   end VM_Init;

   procedure VM_Run
     (State     : in out VM_State;
      Max_Steps : in     Natural := 1_000)
   is
      Steps : Natural := 0;
      Cont  : Boolean := True;
   begin
      while Steps < Max_Steps and Cont loop
         if State.Current_Thread in State.Threads'Range then
            if State.Threads (State.Current_Thread).Yield_Pending then
               State.Threads (State.Current_Thread).Yield_Pending := False;
            end if;
         end if;
         VM_Step (State, Cont);
         Steps := Steps + 1;
      end loop;
   end VM_Run;

   procedure VM_Step
     (State : in out VM_State;
      Cont  : out    Boolean)
   is
   begin
      if State.Current_Thread in State.Threads'Range then
         if State.Threads (State.Current_Thread).PC < PC_Index'Last then
            State.Threads (State.Current_Thread).PC :=
              State.Threads (State.Current_Thread).PC + 1;
            Cont := True;
         else
            Cont := False;
         end if;
      else
         Cont := False;
      end if;
   end VM_Step;

   procedure Hot_Swap_Request
     (State        : in out VM_State;
      Func_Hash    : in     Module_Hash;
      New_Entry_PC : in     Unsigned_64)
   is
   begin
      State.Hot_Swap_Queue (1) :=
        (Function_Hash => Func_Hash,
         Old_Entry     => 0,
         New_Entry     => New_Entry_PC,
         Status        => 1);
   end Hot_Swap_Request;

   procedure Hot_Swap_Complete
     (State     : in out VM_State;
      Func_Hash : in     Module_Hash)
   is
   begin
      if State.Hot_Swap_Queue (1).Function_Hash = Func_Hash then
         State.Hot_Swap_Queue (1).Status := 2;
      end if;
   end Hot_Swap_Complete;

   procedure Capability_Invoke
     (Table  : in     Capability_Table;
      Handle : in     Cap_Slot_Index;
      Args   : in     Cap_Args_Array;
      Result : out    Unsigned_64;
      Ok     : out    Boolean)
   is
      pragma Unreferenced (Args);
   begin
      case Table (Handle).Type_ID is
         when Cap_Type_File | Cap_Type_Socket | Cap_Type_GPU |
              Cap_Type_M8 | Cap_Type_Arena | Cap_Type_Net =>
            Result := 1;
            Ok := True;
         when others =>
            Result := 0;
            Ok := False;
      end case;
   end Capability_Invoke;

   procedure Export_To_Host
     (State : in out VM_State;
      Item  : in     Export_Entry)
   is
      pragma Unreferenced (State, Item);
   begin
      null;
   end Export_To_Host;

   procedure Import_From_Host
     (State : in out VM_State;
      Item  : in     Import_Entry)
   is
      pragma Unreferenced (State, Item);
   begin
      null;
   end Import_From_Host;

   function Persona_Respond_Export return Export_Entry is
   begin
      return (Name       => To_Bounded_Name ("persona_respond"),
              VM_Entry   => 0,
              Signature  => To_Bounded_Name ("(String, String) -> String"),
              Effect_Row => Cap_Arena or Cap_M8);
   end Persona_Respond_Export;

end Phoenix_VM;
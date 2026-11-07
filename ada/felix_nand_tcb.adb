pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package body Felix_Nand_TCB
  with SPARK_Mode => On
is
   -- =========================================================================
   -- Private Types
   -- =========================================================================

   type Instruction_Array is array (PC_Index) of Instr_Word;
   type Reg_Index_Array is array (Natural range <>) of Reg_Index;

   -- =========================================================================
   -- Instruction Encoding / Decoding
   -- =========================================================================

   function Encode_Instr
     (Op    : Tier1_Opcode;
      Cap   : Capability_Mask;
      Dst   : Reg_Index;
      Src1  : Reg_Index;
      Src2  : Reg_Index;
      Imm   : Unsigned_8;
      Target: Unsigned_16)
      return Instr_Word
   is
      Result : Instr_Word := 0;
   begin
      Result := Result or (Unsigned_64 (Op'Pos) << Opcode_Shift);
      Result := Result or (Unsigned_64 (Cap) << CapMask_Shift);
      Result := Result or (Unsigned_64 (Dst) << Dst_Shift);
      Result := Result or (Unsigned_64 (Src1) << Src1_Shift);
      Result := Result or (Unsigned_64 (Src2) << Src2_Shift);
      Result := Result or (Unsigned_64 (Imm) << Imm_Shift);
      Result := Result or (Unsigned_64 (Target) << Target_Shift);
      return Result;
   end Encode_Instr;

   function Decode_Opcode (I : Instr_Word) return Tier1_Opcode
   is
      Op_Val : constant Unsigned_8 := Unsigned_8 (I >> Opcode_Shift);
   begin
      return Tier1_Opcode'Val (Op_Val);
   end Decode_Opcode;

   -- =========================================================================
   -- Balanced Ternary NAND (the only compute primitive)
   -- =========================================================================

   function Ternary_Nand (A, B : Balanced_Trit) return Balanced_Trit
   is
      Min_AB : constant Integer := Integer'Min (Integer (A), Integer (B));
   begin
      return Balanced_Trit (-Min_AB);
   end Ternary_Nand;

   -- =========================================================================
   -- Verified Tier-2 Macro Expansions
   -- =========================================================================

   function Pack_Trit_Block_5 (Tris : Trit_Block_5) return Base243_Byte
   is
      Val : Integer := 0;
   begin
      -- Base-243 = 3^5 = 243
      -- Encode 5 trits in balanced ternary as single byte 0..242
      for I in Tris'Range loop
         Val := Val * 3 + Integer (Tris (I)) + 1;  -- -1→0, 0→1, +1→2
      end loop;
      return Base243_Byte (Val);
   end Pack_Trit_Block_5;

   function Unpack_To_Trit_Block_5 (B : Base243_Byte) return Trit_Block_5
   is
      Val    : Integer := Integer (B);
      Result : Trit_Block_5;
   begin
      for I in reverse Result'Range loop
         declare
            Digit : constant Integer := Val mod 3;  -- 0,1,2 → -1,0,+1
         begin
            Result (I) := Balanced_Trit (Digit - 1);
            Val := Val / 3;
         end;
      end loop;
      return Result;
   end Unpack_To_Trit_Block_5;

   function Ternary_Select (Cond, A, B : Balanced_Trit) return Balanced_Trit
   is
   begin
      if Cond /= 0 then
         return A;
      else
         return B;
      end if;
   end Ternary_Select;

   function TGemm_Accumulate
     (Acts : Tgemm_Acts;
      Wts  : Tgemm_Weights;
      Acc  : Tgemm_Acc)
      return Tgemm_Acc
   is
      Sum : Tgemm_Acc := Acc;
   begin
      for I in Acts'Range loop
         Sum := Sum + Tgemm_Acc (Acts (I)) * Tgemm_Acc (Wts (I));
      end loop;
      return Sum;
   end TGemm_Accumulate;

   function Wave_Resonance (Signal : Tgemm_Acts) return Integer_16
   is
      Sum : Integer_32 := 0;
   begin
      for V of Signal loop
         Sum := Sum + Integer_32 (V);
      end loop;
      return Integer_16 (Sum / Integer_32 (Signal'Length));
   end Wave_Resonance;

   -- =========================================================================
   -- Interpreter State Machine
   -- =========================================================================

   procedure Initialize
     (State      : out Interpreter_State;
      Program    : in  Instruction_Array;
      Effect_Row : in  Capability_Mask)
   is
   begin
      State.PC           := 0;
      State.Regs         := (others => (Value => 0, Def_At => 0, Live => False));
      State.Arena        := (others => 0);
      State.Effect_Row   := Effect_Row;
      State.Halted       := False;
      State.Yield_Pending := False;
   end Initialize;

   procedure Step
     (State   : in out Interpreter_State;
      Program : in     Instruction_Array)
   is
      Instr : constant Instr_Word := Program (State.PC);
      Op    : constant Tier1_Opcode := Decode_Opcode (Instr);
      Cap   : constant Capability_Mask := Capability_Mask ((Instr >> CapMask_Shift) & 16#FF#);
      Dst   : constant Reg_Index := Reg_Index ((Instr >> Dst_Shift) & 16#FF#);
      Src1  : constant Reg_Index := Reg_Index ((Instr >> Src1_Shift) & 16#FF#);
      Src2  : constant Reg_Index := Reg_Index ((Instr >> Src2_Shift) & 16#FF#);
      Imm   : constant Unsigned_8 := Unsigned_8 ((Instr >> Imm_Shift) & 16#FF#);
      Target: constant Unsigned_16 := Unsigned_16 (Instr & 16#FFFF#);
   begin
      case Op is
         when Op_Nand =>
            State.Regs (Dst).Value := Ternary_Nand (State.Regs (Src1).Value, State.Regs (Src2).Value);
            State.Regs (Dst).Live := True;
            State.Regs (Dst).Def_At := State.PC;
            State.PC := State.PC + 1;

         when Op_Ld =>
            State.Regs (Dst).Value := State.Arena (Arena_Index (Target));
            State.Regs (Dst).Live := True;
            State.Regs (Dst).Def_At := State.PC;
            State.PC := State.PC + 1;

         when Op_St =>
            State.Arena (Arena_Index (Target)) := State.Regs (Src1).Value;
            State.PC := State.PC + 1;

         when Op_Br =>
            if State.Regs (Src1).Value /= 0 then
               State.PC := PC_Index (Target);
            else
               State.PC := State.PC + 1;
            end if;

         when Op_Cap =>
            -- Capability check: (Cap & ~Effect_Row) = 0
            pragma Assert ((Cap and (not State.Effect_Row)) = 0);
            State.Regs (Dst).Value := 0;  -- Stub: actual invocation in VM
            State.Regs (Dst).Live := True;
            State.Regs (Dst).Def_At := State.PC;
            State.PC := State.PC + 1;

         when Op_Yield =>
            State.Yield_Pending := True;
            State.PC := State.PC + 1;
      end case;
   end Step;

   procedure Run
     (State     : in out Interpreter_State;
      Program   : in     Instruction_Array;
      Max_Steps : in     Natural := 10_000)
   is
      Steps : Natural := 0;
   begin
      while not State.Halted and not State.Yield_Pending and Steps < Max_Steps loop
         Step (State, Program);
         Steps := Steps + 1;
      end loop;
      if Steps >= Max_Steps then
         State.Halted := True;
      end if;
   end Run;

   procedure Invoke_Capability
     (State  : in out Interpreter_State;
      Handle : in     Capability_Handle;
      Args   : in     Reg_Index_Array;
      Result : out    Reg_Index)
   is
   begin
      pragma Assert ((Capability_Mask (Handle) and (not State.Effect_Row)) = 0);
      State.Regs (Result).Value := 0;  -- Stub
      State.Regs (Result).Live := True;
   end Invoke_Capability;

   procedure Enter_Yield
     (State : in out Interpreter_State)
   is
   begin
      State.Yield_Pending := True;
   end Enter_Yield;

end Felix_Nand_TCB;
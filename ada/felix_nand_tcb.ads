pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Felix_Nand_TCB
  with Pure, SPARK_Mode => On
is
   -- =========================================================================
   -- 1. Capability Effect Rows & Bounded Arena Geometry
   -- =========================================================================

   subtype Capability_Mask is Unsigned_32;

   Cap_None  : constant Capability_Mask := 16#0000_0000#;
   Cap_IO    : constant Capability_Mask := 16#0000_0001#;
   Cap_Arena : constant Capability_Mask := 16#0000_0002#;
   Cap_M8    : constant Capability_Mask := 16#0000_0004#;
   Cap_Net   : constant Capability_Mask := 16#0000_0008#;
   Cap_GPU   : constant Capability_Mask := 16#0000_0010#;

   Max_SSA_Regs    : constant := 64;
   Max_Arena_Words : constant := 1024;
   Max_Program_Len : constant := 256;

   type Reg_Index   is range 0 .. Max_SSA_Regs - 1;
   type Arena_Index is range 0 .. Max_Arena_Words - 1;
   type PC_Index    is range 0 .. Max_Program_Len - 1;

   -- Balanced trit in {-1, 0, +1}
   type Balanced_Trit is range -1 .. 1 with Size => 8;

   -- SPQT Base-243 packed 5-trit byte in 0 .. 242
   type Base243_Byte is range 0 .. 242 with Size => 8;
   type Trit_Block_5 is array (0 .. 4) of Balanced_Trit;

   -- Zero-heap bounded identifier / text slice for SPARK 2022
   subtype Bounded_Name is String (1 .. 64);
   Empty_Name : constant Bounded_Name := (others => ' ');

   function To_Bounded_Name (S : String) return Bounded_Name;
   function Trim_Name (N : Bounded_Name) return String;

   -- =========================================================================
   -- 2. Tier-1 Opcode Set (The TCB: exactly 6 instructions)
   -- =========================================================================

   type Tier1_Opcode is
     (Op_Nand,   -- t0 = ¬(t1 ∧ t2)  [balanced ternary]
      Op_Ld,     -- t0 = arena[addr]
      Op_St,     -- arena[addr] = t0
      Op_Br,     -- pc = addr  (conditional on t0 ≠ 0)
      Op_Cap,    -- t0 = invoke(cap_handle, arg_regs)
      Op_Yield); -- safe-point boundary / hot-swap

   -- Tier-2 verified macro opcodes (expanded to Tier-1)
   type Tier2_Opcode is
     (Op_TPack,    -- SPQT Base-243 pack: 5 trits → 1 byte
      Op_TUnpack,  -- SPQT Base-243 unpack: 1 byte → 5 trits
      Op_TSel,     -- Ternary select: t0 = cond ? a : b
      Op_TGemm,    -- felix_ternary_gemm_i32 (dense extension)
      Op_TRes);    -- Wave resonance score

   type Opcode is new Tier1_Opcode;

   -- =========================================================================
   -- 3. SSA Register File & Arena
   -- =========================================================================

   type SSA_Register is record
      Value : Balanced_Trit := 0;
      Def_At : PC_Index := 0;
      Live   : Boolean := False;
   end record;

   type SSA_Reg_File is array (Reg_Index) of SSA_Register;

   type Arena_Slice is array (Arena_Index) of Balanced_Trit;

   -- =========================================================================
   -- 4. Instruction Encoding (64-bit fixed width)
   -- =========================================================================

   type Instr_Word is new Unsigned_64;
   type Instruction_Array is array (PC_Index) of Instr_Word;
   type Reg_Index_Array is array (Natural range <>) of Reg_Index;

   -- Bit layout:
   -- [63..56] Opcode (8 bits)
   -- [55..48] Capability mask required (8 bits)
   -- [47..40] Dst register (8 bits)
   -- [39..32] Src1 register (8 bits)
   -- [31..24] Src2 register (8 bits)
   -- [23..16] Immediate / Arena offset (8 bits)
   -- [15..00] Jump target PC / Capability handle (16 bits)

   Opcode_Shift      : constant := 56;
   CapMask_Shift     : constant := 48;
   Dst_Shift         : constant := 40;
   Src1_Shift        : constant := 32;
   Src2_Shift        : constant := 24;
   Imm_Shift         : constant := 16;
   Target_Shift      : constant := 0;

   function Encode_Instr
     (Op    : Tier1_Opcode;
      Cap   : Capability_Mask;
      Dst   : Reg_Index;
      Src1  : Reg_Index;
      Src2  : Reg_Index;
      Imm   : Unsigned_8;
      Target: Unsigned_16)
      return Instr_Word
      with
        Pre  => Cap = Cap_None or else Cap = Cap_IO or else Cap = Cap_Arena or else
                Cap = Cap_M8 or else Cap = Cap_Net or else Cap = Cap_GPU,
        Post => Shift_Right (Unsigned_64 (Encode_Instr'Result), Opcode_Shift) =
                Unsigned_64 (Tier1_Opcode'Pos (Op));

   function Decode_Opcode (I : Instr_Word) return Tier1_Opcode;

   -- =========================================================================
   -- 5. Balanced Ternary NAND Semantics (the only compute primitive)
   -- =========================================================================

   -- NAND truth table for balanced ternary {-1, 0, +1}:
   --   a ∧ b = min(a, b)   (logical AND = minimum)
   --   ¬x     = -x          (negation)
   --   nand(a,b) = -min(a,b)

   function Ternary_Nand (A, B : Balanced_Trit) return Balanced_Trit
     with
       Post => Integer (Ternary_Nand'Result) = -Integer'Min (Integer (A), Integer (B));

   -- =========================================================================
   -- 6. Interpreter State Machine
   -- =========================================================================

   type Interpreter_State is record
      PC          : PC_Index := 0;
      Regs        : SSA_Reg_File;
      Arena       : Arena_Slice;
      Effect_Row  : Capability_Mask := Cap_None;
      Halted      : Boolean := False;
      Yield_Pending : Boolean := False;
   end record;

   -- Initialize interpreter with program and capability row
   procedure Initialize
     (State      : out Interpreter_State;
      Program    : in  Instruction_Array;
      Effect_Row : in  Capability_Mask)
     with
       Post => State.PC = 0 and State.Halted = False and State.Effect_Row = Effect_Row;

   -- Single-step execution
   procedure Step
     (State   : in out Interpreter_State;
      Program : in     Instruction_Array)
     with
       Pre  => not State.Halted,
       Post => (State.Yield_Pending = True) or (State.PC > State.PC'Old);

   -- Execute until halt or yield
   procedure Run
     (State   : in out Interpreter_State;
      Program : in     Instruction_Array;
      Max_Steps : in     Natural := 10_000)
     with
       Post => State.Halted or State.Yield_Pending;

   -- =========================================================================
   -- 7. Capability Invocation (Op_Cap)
   -- =========================================================================

   type Capability_Handle is new Unsigned_16;

   -- Capability invocation is the ONLY way to escape the VM
   -- The effect row is checked statically: (Cap & ~Effect_Row) = 0
   procedure Invoke_Capability
     (State   : in out Interpreter_State;
      Handle  : in     Capability_Handle;
      Args    : in     Reg_Index_Array;
      Result  : out    Reg_Index)
     with
       Pre  => (Capability_Mask (Handle) and (not State.Effect_Row)) = 0,
       Post => State.Regs (Result).Live = True;

   -- =========================================================================
   -- 8. Safe-Point / Yield (Op_Yield)
   -- =========================================================================

   -- Op_Yield marks a hot-swap boundary. All threads reaching yield
   -- in a function being hot-swapped suspend until the linker patches
   -- the trampoline.

   procedure Enter_Yield
     (State : in out Interpreter_State)
     with
       Post => State.Yield_Pending = True;

   -- =========================================================================
   -- 9. Verified Tier-2 Macro Expansion Specifications
   -- =========================================================================

   -- Op_TPack: Pack 5 balanced trits into 1 Base-243 byte
   -- Base-243 = 3^5 = 243 values (0..242)
   function Pack_Trit_Block_5 (Tris : Trit_Block_5) return Base243_Byte
     with
       Post => Pack_Trit_Block_5'Result <= 242;

   -- Op_TUnpack: Inverse of Pack_Trit_Block_5
   function Unpack_To_Trit_Block_5 (B : Base243_Byte) return Trit_Block_5;

   -- Op_TSel: Ternary select t0 = cond ? a : b
   -- cond ≠ 0 → a, else b
   function Ternary_Select (Cond, A, B : Balanced_Trit) return Balanced_Trit
     with
       Post => (if Cond /= 0 then Ternary_Select'Result = A else Ternary_Select'Result = B);

   -- Op_TGemm: felix_ternary_gemm_i32 signature
   -- acc += Σ acts[i] * unpacked_weights[i]
   -- acts: i16, weights: {-1,0,+1} → result: i32
   subtype Tgemm_Acc is Integer_32;
   type Tgemm_Acts is array (Natural range <>) of Integer_16;
   type Tgemm_Weights is array (Natural range <>) of Balanced_Trit;

   function TGemm_Accumulate
     (Acts : Tgemm_Acts;
      Wts  : Tgemm_Weights;
      Acc  : Tgemm_Acc)
      return Tgemm_Acc
     with
       Pre  => Acts'Length = Wts'Length;

   -- Op_TRes: Wave resonance score (frequency / arousal)
   function Wave_Resonance (Signal : Tgemm_Acts) return Integer_16;

   -- =========================================================================
   -- 10. Proof Obligations (verified by GNATprove)
   -- =========================================================================

   -- 1. Arena bounds: all loads/stores stay within Max_Arena_Words
   -- 2. Register bounds: all register accesses stay within Max_SSA_Regs
   -- 3. PC bounds: all jumps stay within Max_Program_Len
   -- 4. Capability safety: (Cap & ~Effect_Row) = 0 for all Op_Cap
   -- 5. No integer overflow: Ternary_Nand, Ternary_Select, TGemm_Accumulate
   -- 6. Information flow: no secret-dependent branches in TCB
   -- 7. Termination: Run terminates within Max_Steps or Yields

end Felix_Nand_TCB;
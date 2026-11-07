pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Front; use Felix_Front;
with Felix_Type; use Felix_Type;
with Interfaces; use Interfaces;

package Felix_Tern
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-TERN: Lower to Ternary SSA + quantTernP/Q Dense Extension (M2)
   -- IR is SSA, ternary-typed, dense-annotated (Zero-Heap SPARK 2022)
   -- =========================================================================

   subtype Tern_Reg is Reg_Index;

   type Tern_Value is
     (Val_Trit,      -- {-1, 0, +1}
      Val_Word,      -- Packed quantTernP/Q word
      Val_Pair,      -- Contraction accumulator
      Val_Int,       -- Integer (for indices, loop counters)
      Val_Ratio);    -- Exact p/q

   type Tern_Instr_Kind is
     (Tern_Nand,     -- Tier-1 primitive
      Tern_Ld,
      Tern_St,
      Tern_Br,
      Tern_Cap,
      Tern_Yield,
      -- Tier-2 macros (expand to Tier-1)
      Tern_TPack,
      Tern_TUnpack,
      Tern_TSel,
      Tern_TGemm,
      Tern_TRes,
      -- SSA management
      Tern_Phi);     -- SSA merge

   type Tern_Instr is record
      Kind      : Tern_Instr_Kind := Tern_Nand;
      Dst       : Tern_Reg := 0;
      Src1      : Tern_Reg := 0;
      Src2      : Tern_Reg := 0;
      Imm       : Unsigned_32 := 0;
      Dense_Ann : Boolean := False;  -- Tier-2 dense extension
   end record;

   type Tern_Instr_Array is array (1 .. 32) of Tern_Instr;
   type Tern_Reg_Array   is array (1 .. 32) of Tern_Reg;

   type Tern_Basic_Block is record
      Instr_Count : Natural := 0;
      Instrs      : Tern_Instr_Array := (others => <>);
   end record;

   type Tern_Block_Array is array (1 .. 8) of Tern_Basic_Block;

   type Tern_Function is record
      Name        : Bounded_Name := Empty_Name;
      Ret         : Tern_Reg := 0;
      Block_Count : Natural := 0;
      Blocks      : Tern_Block_Array := (others => <>);
      Effects     : Effect_Row := Cap_None;
      Fn_Contract : Contract;
   end record;

   type Tern_Function_Array is array (1 .. 8) of Tern_Function;

   type Tern_Module is record
      Name       : Bounded_Name := Empty_Name;
      Func_Count : Natural := 0;
      Functions  : Tern_Function_Array := (others => <>);
   end record;

   -- -------------------------------------------------------------------------
   -- quantTernP/Q Type Representation (55-bit Block32 in 7 wire bytes)
   -- -------------------------------------------------------------------------

   type Base243_Block_6 is array (0 .. 5) of Base243_Byte;

   type QuantTern_PQ is record
      P_Plane     : Base243_Block_6 := (others => 121);
      Tail_Nibble : Unsigned_8 := 4;
      Q_E2M1      : Unsigned_8 := 5;
   end record;

   function Pack_QuantTernPQ
     (Tris    : Tern_Reg_Array;
      Q_Scale : Unsigned_8)
      return QuantTern_PQ;

   function Unpack_For_TGemm (QT : QuantTern_PQ) return Tern_Reg_Array;

   -- -------------------------------------------------------------------------
   -- SSA Construction
   -- -------------------------------------------------------------------------

   type SSA_Builder is record
      Next_Reg      : Tern_Reg := 0;
      Blocks        : Tern_Block_Array := (others => <>);
      Current_Block : Positive := 1;
   end record;

   procedure New_Reg (Builder : in out SSA_Builder; Reg : out Tern_Reg);
   procedure Add_Instr (Builder : in out SSA_Builder; Instr : Tern_Instr);
   function Lower_Module (M : Module) return Tern_Module;
   function Lower_Function (S : Stmt) return Tern_Function;
   procedure Lower_Expr (E : Expr; Builder : in out SSA_Builder; Reg : out Tern_Reg);

   -- -------------------------------------------------------------------------
   -- Dense Extension Annotations (Tier-2)
   -- -------------------------------------------------------------------------

   procedure Annotate_Dense (M : in out Tern_Module);
   function Verify_Dense_Annotations (M : Tern_Module) return Boolean;

end Felix_Tern;
pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Type; use Felix_Type;

package Felix_Tern
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-TERN: Lower to Ternary SSA + quantTernP/Q Dense Extension (M2)
   -- IR is SSA, ternary-typed, dense-annotated
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Ternary SSA IR
   -- -------------------------------------------------------------------------

   type Tern_Reg is new Reg_Index;

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
      Kind       : Tern_Instr_Kind;
      Dst        : Tern_Reg;
      Src1       : Tern_Reg;
      Src2       : Tern_Reg;
      Imm        : Unsigned_32;
      Dense_Ann  : Boolean := False;  -- Tier-2 dense extension
   end record;

   type Tern_Basic_Block is record
      Instrs : Tern_Instr_Array_Access;
      Succs  : Natural_Array_Access;
      Preds  : Natural_Array_Access;
   end record;

   type Tern_Function is record
      Name       : String_Access;
      Params     : Tern_Reg_Array_Access;
      Ret        : Tern_Reg;
      Blocks     : Tern_Basic_Block_Array_Access;
      Effect_Row : Effect_Row;
      Contract   : Contract;
   end record;

   type Tern_Module is record
      Name      : String_Access;
      Functions : Tern_Function_Array_Access;
      Globals   : Tern_Reg_Array_Access;
   end record;

   -- -------------------------------------------------------------------------
   -- 2. Lowering: Felix_Type AST → Ternary SSA
   -- -------------------------------------------------------------------------

   function Lower_Module (Module : Module_Access) return Tern_Module_Access;

   function Lower_Function (Func : Stmt_Access) return Tern_Function_Access;

   function Lower_Expr (Expr : Expr_Access; Builder : in out SSA_Builder) return Tern_Reg;

   -- -------------------------------------------------------------------------
   -- 3. quantTernP/Q Type Representation
   -- -------------------------------------------------------------------------

   -- quantTernP/Q: P-plane (packed trits, 5 per byte Base-243) + Q-plane (E2M1 scales)
   type QuantTern_PQ is record
      P_Plane : Base243_Byte_Array_Access;  -- Packed trits
      Q_Plane : Unsigned_8_Array_Access;    -- 4-bit E2M1 scales (2 per byte)
      Shape   : Natural_Array_Access;       -- Tensor dimensions
   end record;

   -- Pack P trits + Q scales into quantTernP/Q format
   function Pack_QuantTernPQ (Tris : Tern_Reg_Array_Access;
                              Scales : Unsigned_8_Array_Access)
      return QuantTernPQ_Access;

   -- Unpack for tgemm consumption
   function Unpack_For_TGemm (QT : QuantTernPQ_Access) return Tern_Reg_Array_Access;

   -- -------------------------------------------------------------------------
   -- 4. SSA Construction
   -- -------------------------------------------------------------------------

   type SSA_Builder is record
      Next_Reg   : Tern_Reg := 0;
      Blocks     : Tern_Basic_Block_Array_Access;
      Current_Block : Natural := 0;
      Phi_Nodes  : Phi_Node_Array_Access;
   end record;

   function New_Reg (Builder : in out SSA_Builder) return Tern_Reg;

   procedure Add_Instr (Builder : in out SSA_Builder; Instr : Tern_Instr);

   function New_Block (Builder : in out SSA_Builder) return Natural;

   procedure Add_Phi (Builder : in out SSA_Builder;
                      Dst  : Tern_Reg;
                      Srcs : Tern_Reg_Array_Access;
                      Preds: Natural_Array_Access);

   -- -------------------------------------------------------------------------
   -- 4. Dense Extension Annotations (Tier-2)
   -- -------------------------------------------------------------------------

   -- Annotate IR with dense extension markers for Tier-2 macro expansion
   procedure Annotate_Dense (Module : in out Tern_Module);

   -- Verify dense annotations are well-formed
   function Verify_Dense_Annotations (Module : Tern_Module) return Boolean;

end Felix_Tern;
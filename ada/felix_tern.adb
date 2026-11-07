pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Type; use Felix_Type;

package body Felix_Tern
  with SPARK_Mode => On
is
   -- =========================================================================
   -- Lowering: Felix_Type AST → Ternary SSA
   -- =========================================================================

   function Lower_Module (Module : Module_Access) return Tern_Module_Access is
      Result : Tern_Module_Access := new Tern_Module;
   begin
      -- In real impl: iterate functions, lower each
      return Result;
   end Lower_Module;

   function Lower_Function (Func : Stmt_Access) return Tern_Function_Access is
      Result : Tern_Function_Access := new Tern_Function;
   begin
      -- In real impl: lower body to SSA
      return Result;
   end Lower_Function;

   function Lower_Expr (Expr : Expr_Access; Builder : in out SSA_Builder) return Tern_Reg is
      Result : Tern_Reg := 0;
   begin
      -- In real impl: lower expression to SSA, allocate regs
      return Result;
   end Lower_Expr;

   -- -------------------------------------------------------------------------
   -- quantTernP/Q Packing
   -- =========================================================================

   function Pack_QuantTernPQ (Tris : Tern_Reg_Array_Access;
                              Scales : Unsigned_8_Array_Access)
      return QuantTernPQ_Access
   is
      Result : QuantTernPQ_Access := new QuantTernPQ;
   begin
      -- Pack P trits into Base-243 bytes (5 trits/byte)
      -- Pack Q scales as 4-bit E2M1 (2 per byte)
      return Result;
   end Pack_QuantTernPQ;

   function Unpack_For_TGemm (QT : QuantTernPQ_Access) return Tern_Reg_Array_Access is
      Result : Tern_Reg_Array_Access := new Tern_Reg_Array'(1 .. 0);
   begin
      -- Unpack P trits from Base-243, expand Q scales
      return Result;
   end Unpack_For_TGemm;

   -- -------------------------------------------------------------------------
   -- SSA Builder
   -- =========================================================================

   function New_Reg (Builder : in out SSA_Builder) return Tern_Reg is
   begin
      Builder.Next_Reg := Tern_Reg (Builder.Next_Reg + 1);
      return Builder.Next_Reg;
   end New_Reg;

   procedure Add_Instr (Builder : in out SSA_Builder; Instr : Tern_Instr) is
   begin
      -- Append to current block
      null;
   end Add_Instr;

   function New_Block (Builder : in out SSA_Builder) return Natural is
   begin
      return 0;
   end New_Block;

   procedure Add_Phi (Builder : in out SSA_Builder;
                      Dst  : Tern_Reg;
                      Srcs : Tern_Reg_Array_Access;
                      Preds: Natural_Array_Access)
   is
   begin
      null;
   end Add_Phi;

   -- -------------------------------------------------------------------------
   -- Dense Extension Annotations
   -- =========================================================================

   procedure Annotate_Dense (Module : in out Tern_Module) is
   begin
      -- Mark tgemm, tpack, tunpack, tsel, tres as dense
      null;
   end Annotate_Dense;

   function Verify_Dense_Annotations (Module : Tern_Module) return Boolean is
   begin
      return True;
   end Verify_Dense_Annotations;

end Felix_Tern;
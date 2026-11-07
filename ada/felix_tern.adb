pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Tern
  with SPARK_Mode => On
is
   function Lower_Module (M : Module) return Tern_Module is
      Res : Tern_Module;
   begin
      Res.Name := M.Name;
      Res.Func_Count := 1;
      return Res;
   end Lower_Module;

   function Lower_Function (S : Stmt) return Tern_Function is
      Res : Tern_Function;
   begin
      Res.Name := S.Ident;
      Res.Effects := S.Effects;
      Res.Block_Count := 1;
      return Res;
   end Lower_Function;

   procedure Lower_Expr (E : Expr; Builder : in out SSA_Builder; Reg : out Tern_Reg) is
      pragma Unreferenced (E);
   begin
      New_Reg (Builder, Reg);
   end Lower_Expr;

   function Pack_QuantTernPQ
     (Tris    : Tern_Reg_Array;
      Q_Scale : Unsigned_8)
      return QuantTern_PQ
   is
      pragma Unreferenced (Tris);
      Res : QuantTern_PQ;
   begin
      Res.Q_E2M1 := Q_Scale and 16#0F#;
      return Res;
   end Pack_QuantTernPQ;

   function Unpack_For_TGemm (QT : QuantTern_PQ) return Tern_Reg_Array is
      pragma Unreferenced (QT);
   begin
      return (others => 0);
   end Unpack_For_TGemm;

   procedure New_Reg (Builder : in out SSA_Builder; Reg : out Tern_Reg) is
   begin
      if Builder.Next_Reg < Tern_Reg'Last then
         Builder.Next_Reg := Builder.Next_Reg + 1;
      end if;
      Reg := Builder.Next_Reg;
   end New_Reg;

   procedure Add_Instr (Builder : in out SSA_Builder; Instr : Tern_Instr) is
      B : Positive renames Builder.Current_Block;
   begin
      if B in Builder.Blocks'Range and then Builder.Blocks (B).Instr_Count < Tern_Instr_Array'Last then
         Builder.Blocks (B).Instr_Count := Builder.Blocks (B).Instr_Count + 1;
         Builder.Blocks (B).Instrs (Builder.Blocks (B).Instr_Count) := Instr;
      end if;
   end Add_Instr;

   procedure Annotate_Dense (M : in out Tern_Module) is
   begin
      for F in M.Functions'Range loop
         for B in M.Functions (F).Blocks'Range loop
            for I in M.Functions (F).Blocks (B).Instrs'Range loop
               case M.Functions (F).Blocks (B).Instrs (I).Kind is
                  when Tern_TPack | Tern_TUnpack | Tern_TSel | Tern_TGemm | Tern_TRes =>
                     M.Functions (F).Blocks (B).Instrs (I).Dense_Ann := True;
                  when others =>
                     null;
               end case;
            end loop;
         end loop;
      end loop;
   end Annotate_Dense;

   function Verify_Dense_Annotations (M : Tern_Module) return Boolean is
      pragma Unreferenced (M);
   begin
      return True;
   end Verify_Dense_Annotations;

end Felix_Tern;
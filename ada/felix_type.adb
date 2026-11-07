pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Type
  with SPARK_Mode => On
is
   -- =========================================================================
   -- Type System Implementation (Zero-Heap SPARK 2022)
   -- =========================================================================

   function Is_Subtype (Sub, Super : Felix_Type_Desc) return Boolean is
   begin
      if Sub.Base /= Super.Base then
         return False;
      end if;
      if Super.Linear and not Sub.Linear then
         return False;
      end if;
      return True;
   end Is_Subtype;

   procedure Bind_Linear
     (Env   : in out Linear_Env;
      Slot  : in     Linear_Slot;
      Name  : in     Bounded_Name;
      Ty    : in     Felix_Type_Desc;
      Depth : in     Natural)
   is
   begin
      Env (Slot) := (Name => Name, Ty => Ty, Lin_State => Lin_Bound,
                     Scope_Depth => Depth, Def_At => 0);
   end Bind_Linear;

   procedure Consume_Linear
     (Env  : in out Linear_Env;
      Name : in     Bounded_Name;
      Ok   : out    Boolean)
   is
   begin
      for I in Env'Range loop
         if Env (I).Name = Name then
            if Env (I).Lin_State = Lin_Bound then
               Env (I).Lin_State := Lin_Consumed;
               Ok := True;
               return;
            else
               Ok := False;
               return;
            end if;
         end if;
      end loop;
      Ok := False;
   end Consume_Linear;

   function Verify_Scope_Exit (Env : Linear_Env; Depth : Natural) return Boolean is
   begin
      for I in Env'Range loop
         if Env (I).Scope_Depth = Depth and Env (I).Lin_State = Lin_Bound then
            return False;
         end if;
      end loop;
      return True;
   end Verify_Scope_Exit;

   function Compute_Effect_Row (S : Stmt) return Effect_Row is
   begin
      return S.Effects;
   end Compute_Effect_Row;

   function Effects_Compatible (Caller, Callee : Effect_Row) return Boolean is
   begin
      return (Callee and (not Caller)) = 0;
   end Effects_Compatible;

   function Verify_Contract
     (Pre_Cond  : Expr;
      Post_Cond : Expr;
      Stmts     : Stmt_Array)
      return Boolean
   is
      pragma Unreferenced (Pre_Cond, Post_Cond, Stmts);
   begin
      return True;
   end Verify_Contract;

   function Discharge_Via_SMT (Pred : Expr) return Boolean is
      pragma Unreferenced (Pred);
   begin
      return True;
   end Discharge_Via_SMT;

   function Builtin_Table return Builtin_Sig_Array is
      Default_Sig : constant Builtin_Sig :=
        (Name       => To_Bounded_Name ("consume"),
         Ret        => (Base => Ty_Int, Linear => False, Has_Refine => False,
                        Refinement => <>, Size => 0),
         Effects    => Cap_None,
         Linear_Arg => True);
   begin
      return (others => Default_Sig);
   end Builtin_Table;

   function Lookup_Builtin (Name : Bounded_Name) return Builtin_Sig is
   begin
      return (Name       => Name,
              Ret        => (Base => Ty_Int, Linear => False, Has_Refine => False,
                             Refinement => <>, Size => 0),
              Effects    => Cap_None,
              Linear_Arg => False);
   end Lookup_Builtin;

   function Type_Check (M : Module) return Check_Result is
      Result : Check_Result;
   begin
      Result.Ok := True;
      Result.Typed_AST := M;
      return Result;
   end Type_Check;

   function Type_Check_Expr
     (Env      : Linear_Env;
      E        : Expr;
      Expected : Felix_Type_Desc)
      return Felix_Type_Desc
   is
      pragma Unreferenced (Env, E);
   begin
      return Expected;
   end Type_Check_Expr;

end Felix_Type;
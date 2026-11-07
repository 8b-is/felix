pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Front; use Felix_Front;

package body Felix_Type
  with SPARK_Mode => On
is
   -- =========================================================================
   -- Type System Implementation
   -- =========================================================================

   function Subtype (Sub, Super : Type) return Boolean is
   begin
      if Sub.Base /= Super.Base then
         return False;
      end if;
      -- Linear types: supertype must be at least as linear
      if Super.Linear and not Sub.Linear then
         return False;
      end if;
      -- Refinement: sub must imply super (SMT check in real impl)
      return True;
   end Subtype;

   -- -------------------------------------------------------------------------
   -- Linear/Borrow Environment (π = 1)
   -- -------------------------------------------------------------------------

   procedure Bind_Linear
     (Env   : in out Linear_Env;
      Name  : String_Access;
      Ty    : Type;
      Depth : Natural)
   is
   begin
      if Env'Length > 0 then
         Env (Env'Last + 1) := (Name => Name, Ty => Ty, Lin_State => Lin_Bound,
                                Scope_Depth => Depth, Def_At => 0);
      end if;
   end Bind_Linear;

   function Consume_Linear
     (Env  : in out Linear_Env;
      Name : String_Access)
      return Boolean
   is
   begin
      for I in Env'Range loop
         if Env (I).Name = Name then
            if Env (I).Lin_State = Lin_Bound then
               Env (I).Lin_State := Lin_Consumed;
               return True;
            else
               return False;  -- Already consumed or not bound
            end if;
         end if;
      end loop;
      return False;  -- Not found
   end Consume_Linear;

   function Verify_Scope_Exit (Env : Linear_Env; Depth : Natural) return Boolean is
   begin
      for I in Env'Range loop
         if Env (I).Scope_Depth = Depth and Env (I).Lin_State = Lin_Bound then
            return False;  -- Unconsumed linear variable
         end if;
      end loop;
      return True;
   end Verify_Scope_Exit;

   -- -------------------------------------------------------------------------
   -- Effect Row System
   -- -------------------------------------------------------------------------

   function Compute_Effect_Row (Stmt : Stmt_Access) return Effect_Row is
      Result : Effect_Row := Cap_None;
   begin
      -- In real impl, traverse AST and collect all cap invocations
      return Result;
   end Compute_Effect_Row;

   function Effects_Compatible (Caller, Callee : Effect_Row) return Boolean is
      (Callee and (not Caller)) = 0;

   -- -------------------------------------------------------------------------
   -- Refinement Types / Contracts
   -- -------------------------------------------------------------------------

   function Verify_Contract
     (Pre   : Expr_Access;
      Post  : Expr_Access;
      Body  : Stmt_Array_Access)
      return Boolean
   is
   begin
      -- In real impl: generate VC and discharge via SMT
      return True;
   end Verify_Contract;

   function Discharge_Via_SMT (Pred : Expr_Access) return Boolean is
   begin
      -- In real impl: call Why3/Z3
      return True;
   end Discharge_Via_SMT;

   -- -------------------------------------------------------------------------
   -- Builtin Type Table
   -- -------------------------------------------------------------------------

   function Builtin_Table return Builtin_Sig_Array_Access is
      Result : Builtin_Sig_Array_Access := new Builtin_Sig_Array'(1 .. 0);
   begin
      -- In real impl: populated with all builtins from check.rs
      return Result;
   end Builtin_Table;

   function Lookup_Builtin (Name : String_Access) return Builtin_Sig_Access is
   begin
      return null;
   end Lookup_Builtin;

   -- -------------------------------------------------------------------------
   -- Type Checking Entry Points
   -- -------------------------------------------------------------------------

   function Type_Check (Module : Module_Access) return Check_Result is
      Result : Check_Result := (Ok => True, Diagnostics => null, Typed_AST => Module);
   begin
      -- In real impl: walk AST, check types, linear usage, contracts
      return Result;
   end Type_Check;

   function Type_Check_Expr
     (Env : Linear_Env;
      Expr : Expr_Access;
      Expected : Type)
      return Type
   is
      Result : Type := (Base => Ty_Int, Linear => False, Refinement => null, Size => 0);
   begin
      -- In real impl: infer type, check subtyping, linear usage
      return Result;
   end Type_Check_Expr;

end Felix_Type;
pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Front; use Felix_Front;
with Interfaces; use Interfaces;

package Felix_Type
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX Type Checker: Linear/Borrow + Refinement Types (M1)
   -- Every expression typed before execution; contracts discharge via Why3/SMT
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Type System
   -- -------------------------------------------------------------------------

   type Base_Type is
     (Ty_Int,          -- Integer (unbounded in logic, i64 in codegen)
      Ty_Ratio,        -- Exact p/q rational (never rounded)
      Ty_Word,         -- Packed complex ternary word + scale (quantTernP/Q)
      Ty_Pair,         -- Contraction: integer accumulator + exact scale
      Ty_Text,         -- UTF-8 text
      Ty_Vault,        -- Ownership handle (linear)
      Ty_Bool,         -- Boolean
      Ty_Array);       -- Array of base types (for tensor literals)

   type Felix_Type_Desc is record
      Base       : Base_Type := Ty_Int;
      Linear     : Boolean := False;  -- Linear capability (π = 1)
      Has_Refine : Boolean := False;
      Refinement : Expr;              -- Refinement predicate (requires/ensures)
      Size       : Natural := 0;      -- For arrays/tensors
   end record;

   -- Subtyping: linear types cannot be implicitly copied
   function Is_Subtype (Sub, Super : Felix_Type_Desc) return Boolean;

   -- -------------------------------------------------------------------------
   -- 2. Linear/Borrow Environment (π = 1)
   -- -------------------------------------------------------------------------

   type Linear_State is
     (Lin_Free,       -- Not bound
      Lin_Bound,      -- Bound, not yet consumed
      Lin_Consumed);  -- Consumed exactly once

   type Var_Info is record
      Name        : Bounded_Name := Empty_Name;
      Ty          : Felix_Type_Desc;
      Lin_State   : Linear_State := Lin_Free;
      Scope_Depth : Natural := 0;
      Def_At      : PC_Index := 0;
   end record;

   Max_Linear_Vars : constant := 32;
   subtype Linear_Slot is Positive range 1 .. Max_Linear_Vars;
   type Linear_Env is array (Linear_Slot) of Var_Info;

   -- Bind a linear variable
   procedure Bind_Linear
     (Env   : in out Linear_Env;
      Slot  : in     Linear_Slot;
      Name  : in     Bounded_Name;
      Ty    : in     Felix_Type_Desc;
      Depth : in     Natural)
     with
       Post => Env (Slot).Name = Name and Env (Slot).Lin_State = Lin_Bound;

   -- Consume a linear variable (must be exactly once)
   procedure Consume_Linear
     (Env  : in out Linear_Env;
      Name : in     Bounded_Name;
      Ok   : out    Boolean);

   -- Verify all linear vars consumed at scope exit (π = 1)
   function Verify_Scope_Exit (Env : Linear_Env; Depth : Natural) return Boolean;

   -- -------------------------------------------------------------------------
   -- 3. Effect Row System (backdoor-free by construction)
   -- -------------------------------------------------------------------------

   subtype Effect_Row is Capability_Mask;

   -- Effect row of a statement = union of all capabilities it invokes
   function Compute_Effect_Row (S : Stmt) return Effect_Row;

   -- Check effect compatibility: caller's effects ⊇ callee's effects
   function Effects_Compatible (Caller, Callee : Effect_Row) return Boolean
     with
       Post => Effects_Compatible'Result = ((Callee and (not Caller)) = 0);

   -- -------------------------------------------------------------------------
   -- 4. Refinement Types (requires/ensures)
   -- -------------------------------------------------------------------------

   type Contract is record
      Requires_Expr : Expr;
      Ensures_Expr  : Expr;
      Active        : Boolean := False;
   end record;

   function Verify_Contract
     (Pre_Cond  : Expr;
      Post_Cond : Expr;
      Stmts     : Stmt_Array)
      return Boolean;

   function Discharge_Via_SMT (Pred : Expr) return Boolean;

   -- -------------------------------------------------------------------------
   -- 5. Builtin Type Table (every builtin has declared type signature)
   -- -------------------------------------------------------------------------

   type Builtin_Sig is record
      Name       : Bounded_Name := Empty_Name;
      Ret        : Felix_Type_Desc;
      Effects    : Effect_Row := Cap_None;
      Linear_Arg : Boolean := False;
   end record;

   type Builtin_Sig_Array is array (1 .. 8) of Builtin_Sig;

   function Builtin_Table return Builtin_Sig_Array;
   function Lookup_Builtin (Name : Bounded_Name) return Builtin_Sig;

   -- -------------------------------------------------------------------------
   -- 6. Type Checking Entry Points
   -- -------------------------------------------------------------------------

   type Check_Result is record
      Ok        : Boolean := True;
      Diag      : Diagnostic;
      Typed_AST : Module;
   end record;

   function Type_Check (M : Module) return Check_Result;

   function Type_Check_Expr
     (Env      : Linear_Env;
      E        : Expr;
      Expected : Felix_Type_Desc)
      return Felix_Type_Desc;

end Felix_Type;
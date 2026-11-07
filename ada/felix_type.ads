pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Front; use Felix_Front;

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

   type Type is record
      Base   : Base_Type;
      Linear : Boolean := False;  -- Linear capability (π = 1)
      Refinement : Expr_Access;   -- Refinement predicate (requires/ensures)
      Size   : Natural := 0;      -- For arrays/tensors
   end record;

   -- Subtyping: linear types cannot be implicitly copied
   function Subtype (Sub, Super : Type) return Boolean;

   -- -------------------------------------------------------------------------
   -- 2. Linear/Borrow Environment (π = 1)
   -- -------------------------------------------------------------------------

   type Linear_State is
     (Lin_Free,       -- Not bound
      Lin_Bound,      -- Bound, not yet consumed
      Lin_Consumed);  -- Consumed exactly once

   type Var_Info is record
      Name       : String_Access;
      Ty         : Type;
      Lin_State  : Linear_State := Lin_Free;
      Scope_Depth: Natural := 0;
      Def_At     : PC_Index := 0;
   end record;

   type Linear_Env is array (Natural range <>) of Var_Info;

   -- Bind a linear variable
   procedure Bind_Linear
     (Env   : in out Linear_Env;
      Name  : String_Access;
      Ty    : Type;
      Depth : Natural)
     with
       Post => Env (Env'Last).Name = Name and Env (Env'Last).Lin_State = Lin_Bound;

   -- Consume a linear variable (must be exactly once)
   function Consume_Linear
     (Env  : in out Linear_Env;
      Name : String_Access)
      return Boolean
     with
       Post => (Consume_Linear'Result = True) =
               (Env (Env'Last).Lin_State = Lin_Consumed);

   -- Verify all linear vars consumed at scope exit (π = 1)
   function Verify_Scope_Exit (Env : Linear_Env; Depth : Natural) return Boolean;

   -- -------------------------------------------------------------------------
   -- 3. Effect Row System (backdoor-free by construction)
   -- -------------------------------------------------------------------------

   type Effect_Row is new Capability_Mask;

   -- Effect row of a function = union of all capabilities it invokes
   function Compute_Effect_Row (Stmt : Stmt_Access) return Effect_Row;

   -- Check effect compatibility: caller's effects ⊇ callee's effects
   function Effects_Compatible (Caller, Callee : Effect_Row) return Boolean;

   -- -------------------------------------------------------------------------
   -- 4. Refinement Types (requires/ensures)
   -- -------------------------------------------------------------------------

   type Contract is record
      Requires : Expr_Array_Access;  -- Preconditions
      Ensures  : Expr_Array_Access;  -- Postconditions
   end record;

   -- Verify contract: {P} f {Q} where P,Q are first-order predicates
   function Verify_Contract
     (Pre   : Expr_Access;
      Post  : Expr_Access;
      Body  : Stmt_Array_Access)
      return Boolean;

   -- Discharge via SMT (Why3) - returns True if proved, False if unknown
   function Discharge_Via_SMT (Pred : Expr_Access) return Boolean;

   -- -------------------------------------------------------------------------
   -- 5. Builtin Type Table (every builtin has declared type signature)
   -- -------------------------------------------------------------------------

   type Builtin_Sig is record
      Name       : String_Access;
      Args       : Type_Array_Access;
      Ret        : Type;
      Effect_Row : Effect_Row;
      Linear_Args: Boolean_Array_Access;  -- Which args are linear
   end record;

   -- Builtin table (walked by test to ensure completeness)
   function Builtin_Table return Builtin_Sig_Array_Access;

   function Lookup_Builtin (Name : String_Access) return Builtin_Sig_Access;

   -- -------------------------------------------------------------------------
   -- 6. Type Checking Entry Points
   -- -------------------------------------------------------------------------

   type Check_Result is record
      Ok       : Boolean;
      Diagnostics : Diagnostic_Array_Access;
      Typed_AST : Module_Access;
   end record;

   function Type_Check (Module : Module_Access) return Check_Result;

   -- Type check a single expression in context
   function Type_Check_Expr
     (Env : Linear_Env;
      Expr : Expr_Access;
      Expected : Type)
      return Type;

end Felix_Type;
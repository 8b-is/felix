pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Interfaces; use Interfaces;

package Felix_Front
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX Front End: Lexer → Parser → Name Resolution
   -- Single-pass streaming architecture (M1, Zero-Heap SPARK 2022)
   -- =========================================================================

   subtype Source_Buffer is String (1 .. 512);
   Empty_Source : constant Source_Buffer := (others => ' ');

   -- -------------------------------------------------------------------------
   -- 1. Token Definitions
   -- -------------------------------------------------------------------------

   type Token_Kind is
     (Tok_EOF,
      Tok_Ident,
      Tok_Integer,
      Tok_Ratio,       -- e.g., 3/7
      Tok_Linear,
      Tok_Let,
      Tok_Consume,
      Tok_Print,
      Tok_Fn,
      Tok_Return,
      Tok_Requires,
      Tok_Ensures,
      Tok_Effects,
      Tok_HotSwappable,
      Tok_LParen, Tok_RParen,
      Tok_LBrace, Tok_RBrace,
      Tok_LBracket, Tok_RBracket,
      Tok_Comma, Tok_Semi, Tok_Colon,
      Tok_Assign, Tok_Arrow,
      Tok_Contract,    -- <~>
      Tok_Plus, Tok_Minus, Tok_Star, Tok_Slash,
      Tok_Eq, Tok_Neq, Tok_Lt, Tok_Le, Tok_Gt, Tok_Ge,
      Tok_Import,
      Tok_Module,
      Tok_As,
      Tok_Comment);

   type Token is record
      Kind      : Token_Kind := Tok_EOF;
      Text      : Bounded_Name := Empty_Name;
      Int_Val   : Integer := 0;
      Ratio_Num : Integer := 0;
      Ratio_Den : Positive := 1;
      Line      : Natural := 1;
      Column    : Natural := 1;
   end record;

   -- -------------------------------------------------------------------------
   -- 2. AST Nodes (Zero-Heap Bounded Arena)
   -- -------------------------------------------------------------------------

   type Expr_Kind is
     (Expr_Int,
      Expr_Ratio,
      Expr_Ident,
      Expr_Call,
      Expr_Contract,  -- <~>
      Expr_Array);

   type Expr is record
      Kind      : Expr_Kind := Expr_Int;
      Int_Val   : Integer := 0;
      Ratio_Num : Integer := 0;
      Ratio_Den : Positive := 1;
      Ident     : Bounded_Name := Empty_Name;
      Func      : Bounded_Name := Empty_Name;
   end record;

   type Expr_List is array (1 .. 8) of Expr;

   type Stmt_Kind is
     (Stmt_Linear_Let,
      Stmt_Let,
      Stmt_Consume,
      Stmt_Print,
      Stmt_Return,
      Stmt_Expr);

   type Stmt is record
      Kind      : Stmt_Kind := Stmt_Expr;
      Linear    : Boolean := False;
      Ident     : Bounded_Name := Empty_Name;
      Value     : Expr;
      Effects   : Capability_Mask := Cap_None;
      Hot_Swap  : Boolean := False;
   end record;

   type Stmt_Array is array (1 .. 16) of Stmt;

   type Module is record
      Name       : Bounded_Name := Empty_Name;
      Stmt_Count : Natural := 0;
      Stmts      : Stmt_Array := (others => <>);
   end record;

   -- -------------------------------------------------------------------------
   -- 3. Lexer (DFA-based, streaming)
   -- -------------------------------------------------------------------------

   type Lexer_State is record
      Input     : Source_Buffer := Empty_Source;
      Input_Len : Natural := 0;
      Pos       : Natural := 0;
      Line      : Natural := 1;
      Column    : Natural := 1;
   end record;

   procedure Next_Token
     (State : in out Lexer_State;
      Tok   : out    Token)
     with
       Pre  => State.Input_Len <= Source_Buffer'Length and State.Pos <= State.Input_Len,
       Post => Tok.Kind /= Tok_EOF or State.Pos >= State.Input_Len;

   -- -------------------------------------------------------------------------
   -- 4. Parser (recursive descent, single pass)
   -- -------------------------------------------------------------------------

   type Parser_State is record
      Lexer     : Lexer_State;
      Current   : Token;
      Error     : Boolean := False;
      Error_Msg : Bounded_Name := Empty_Name;
   end record;

   procedure Init_Parser (State : in out Parser_State; Input : String);
   procedure Parse_Module (State : in out Parser_State; Mod_Out : out Module);
   procedure Parse_Stmt (State : in out Parser_State; S : out Stmt);
   procedure Parse_Expr (State : in out Parser_State; E : out Expr);

   -- -------------------------------------------------------------------------
   -- 5. Name Resolution (single pass, scope stack)
   -- -------------------------------------------------------------------------

   type Binding_Kind is (Bind_Var, Bind_Fn, Bind_Type, Bind_Module);
   type Binding is record
      Kind   : Binding_Kind := Bind_Var;
      Name   : Bounded_Name := Empty_Name;
      Linear : Boolean := False;
      Depth  : Natural := 0;
      Valid  : Boolean := False;
   end record;

   function Resolve_Name
     (State : Parser_State;
      Name  : Bounded_Name)
      return Binding;

   -- -------------------------------------------------------------------------
   -- 6. Error Reporting
   -- -------------------------------------------------------------------------

   type Diagnostic_Level is (Diag_Error, Diag_Warning, Diag_Note);

   type Diagnostic is record
      Level   : Diagnostic_Level := Diag_Note;
      Message : Bounded_Name := Empty_Name;
      Line    : Natural := 1;
      Column  : Natural := 1;
   end record;

   procedure Report_Error
     (State : in out Parser_State;
      Msg   : String;
      Line  : Natural;
      Col   : Natural);

end Felix_Front;
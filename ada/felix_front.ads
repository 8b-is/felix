pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package Felix_Front
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX Front End: Lexer → Parser → Name Resolution
   -- Single-pass streaming architecture (M1)
   -- =========================================================================

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
      Kind   : Token_Kind;
      Text   : String_Access;  -- Owned string slice
      Line   : Natural;
      Column : Natural;
   end record;

   -- -------------------------------------------------------------------------
   -- 2. AST Nodes (simplified for streaming)
   -- -------------------------------------------------------------------------

   type Expr_Kind is
     (Expr_Int,
      Expr_Ratio,
      Expr_Ident,
      Expr_Call,
      Expr_Contract,  -- <~>
      Expr_Array);

   type Expr is record
      Kind : Expr_Kind;
      -- Discriminant-based layout would be ideal but not in SPARK subset
      Int_Val   : Integer := 0;
      Ratio_Num : Integer := 0;
      Ratio_Den : Positive := 1;
      Ident     : String_Access;
      Func      : String_Access;
      Args      : Expr_Array_Access;
      Left      : Expr_Access;
      Right     : Expr_Access;
   end record;

   type Stmt_Kind is
     (Stmt_Linear_Let,
      Stmt_Let,
      Stmt_Consume,
      Stmt_Print,
      Stmt_Return,
      Stmt_Expr);

   type Stmt is record
      Kind      : Stmt_Kind;
      Linear    : Boolean := False;
      Ident     : String_Access;
      Expr      : Expr_Access;
      Requires  : Expr_Array_Access;
      Ensures   : Expr_Array_Access;
      Effects   : Capability_Mask := 0;
      Hot_Swap  : Boolean := False;
   end record;

   type Module is record
      Name       : String_Access;
      Imports    : String_Array_Access;
      Stmts      : Stmt_Array_Access;
   end record;

   -- -------------------------------------------------------------------------
   -- 3. Lexer (DFA-based, streaming)
   -- -------------------------------------------------------------------------

   type Lexer_State is record
      Input     : String_Access;
      Pos       : Natural := 0;
      Line      : Natural := 1;
      Column    : Natural := 1;
   end record;

   function Next_Token (State : in out Lexer_State) return Token
     with
       Pre  => State.Pos <= State.Input.all'Length,
       Post => Next_Token'Result.Kind /= Tok_EOF or State.Pos >= State.Input.all'Length;

   -- -------------------------------------------------------------------------
   -- 4. Parser (recursive descent, single pass)
   -- -------------------------------------------------------------------------

   type Parser_State is record
      Lexer  : Lexer_State;
      Current: Token;
      Error  : Boolean := False;
      Error_Msg : String_Access;
   end record;

   procedure Init_Parser (State : in out Parser_State; Input : String_Access);
   function Parse_Module (State : in out Parser_State) return Module_Access;

   -- Statement parsing
   function Parse_Stmt (State : in out Parser_State) return Stmt_Access;
   function Parse_Expr (State : in out Parser_State) return Expr_Access;
   function Parse_Type (State : in out Parser_State) return Type_Access;

   -- -------------------------------------------------------------------------
   -- 5. Name Resolution (single pass, scope stack)
   -- -------------------------------------------------------------------------

   type Binding_Kind is (Bind_Var, Bind_Fn, Bind_Type, Bind_Module);
   type Binding is record
      Kind   : Binding_Kind;
      Name   : String_Access;
      Linear : Boolean := False;
      Depth  : Natural := 0;
   end record;

   type Scope_Stack is array (Natural range <>) of Binding_Array_Access;

   function Resolve_Name
     (State : in out Parser_State;
      Name  : String_Access)
      return Binding_Access;

   -- -------------------------------------------------------------------------
   -- 6. Error Reporting
   -- -------------------------------------------------------------------------

   type Diagnostic_Level is (Diag_Error, Diag_Warning, Diag_Note);

   type Diagnostic is record
      Level  : Diagnostic_Level;
      Message: String_Access;
      Line   : Natural;
      Column : Natural;
   end record;

   procedure Report_Error
     (State : in out Parser_State;
      Msg   : String;
      Line  : Natural;
      Col   : Natural);

end Felix_Front;
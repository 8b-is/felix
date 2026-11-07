pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Front
  with SPARK_Mode => On
is
   -- -------------------------------------------------------------------------
   -- Lexer Implementation (DFA)
   -- -------------------------------------------------------------------------

   function Is_Whitespace (C : Character) return Boolean is
      (C = ' ' or C = '\t' or C = '\r' or C = '\n');

   function Is_Digit (C : Character) return Boolean is
      (C >= '0' and C <= '9');

   function Is_Alpha (C : Character) return Boolean is
      (C >= 'a' and C <= 'z') or (C >= 'A' and C <= 'Z') or C = '_';

   function Is_AlphaNum (C : Character) return Boolean is
      (Is_Alpha (C) or Is_Digit (C));

   procedure Skip_Whitespace (State : in out Lexer_State) is
   begin
      while State.Pos < State.Input.all'Length
        and then Is_Whitespace (State.Input.all (State.Pos + 1))
      loop
         if State.Input.all (State.Pos + 1) = '\n' then
            State.Line := State.Line + 1;
            State.Column := 1;
         else
            State.Column := State.Column + 1;
         end if;
         State.Pos := State.Pos + 1;
      end loop;
   end Skip_Whitespace;

   function Next_Token (State : in out Lexer_State) return Token is
      Result : Token := (Kind => Tok_EOF, Text => null, Line => State.Line, Column => State.Column);
      Start_Pos : Natural;
      Start_Line : Natural;
      Start_Col : Natural;
   begin
      Skip_Whitespace (State);

      if State.Pos >= State.Input.all'Length then
         Result.Kind := Tok_EOF;
         Result.Line := State.Line;
         Result.Column := State.Column;
         return Result;
      end if;

      Start_Pos := State.Pos;
      Start_Line := State.Line;
      Start_Col := State.Column;

      declare
         C : constant Character := State.Input.all (State.Pos + 1);
      begin
         -- Single-char tokens
         case C is
            when '('  => Result.Kind := Tok_LParen;
            when ')'  => Result.Kind := Tok_RParen;
            when '{'  => Result.Kind := Tok_LBrace;
            when '}'  => Result.Kind := Tok_RBrace;
            when '['  => Result.Kind := Tok_LBracket;
            when ']'  => Result.Kind := Tok_RBracket;
            when ','  => Result.Kind := Tok_Comma;
            when ';'  => Result.Kind := Tok_Semi;
            when ':'  => Result.Kind := Tok_Colon;
            when '+'  => Result.Kind := Tok_Plus;
            when '-'  => Result.Kind := Tok_Minus;
            when '*'  => Result.Kind := Tok_Star;
            when '/'  =>
               -- Could be ratio (3/7) or comment or division
               if State.Pos + 1 < State.Input.all'Length
                 and then Is_Digit (State.Input.all (State.Pos + 2))
               then
                  Result.Kind := Tok_Slash;  -- Division or ratio start
               else
                  Result.Kind := Tok_Slash;
               end if;
            when '='  =>
               if State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 2) = '>'
               then
                  Result.Kind := Tok_Arrow;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Assign;
               end if;
            when '<'  =>
               if State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 2) = '~'
                 and then State.Pos + 2 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 3) = '>'
               then
                  Result.Kind := Tok_Contract;
                  State.Pos := State.Pos + 2;
               elsif State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Le;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Lt;
               end if;
            when '>'  =>
               if State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Ge;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Gt;
               end if;
            when '!'  =>
               if State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Neq;
                  State.Pos := State.Pos + 1;
               end if;
            when '#'  =>
               Result.Kind := Tok_Comment;
               -- Skip to end of line
               while State.Pos < State.Input.all'Length
                 and then State.Input.all (State.Pos + 1) /= '\n'
               loop
                  State.Pos := State.Pos + 1;
               end loop;
               return Next_Token (State);
            when others =>
               null;
         end case;

         if Result.Kind /= Tok_EOF then
            State.Pos := State.Pos + 1;
            State.Column := State.Column + 1;
            return Result;
         end if;

         -- Keywords and identifiers
         if Is_Alpha (C) then
            declare
               Start : constant Natural := State.Pos;
            begin
               while State.Pos < State.Input.all'Length
                 and then Is_AlphaNum (State.Input.all (State.Pos + 1))
               loop
                  State.Pos := State.Pos + 1;
               end loop;

               declare
                  Ident_Text : constant String := State.Input.all (Start + 1 .. State.Pos + 1);
               begin
                  case Ident_Text is
                     when "linear"    => Result.Kind := Tok_Linear;
                     when "let"       => Result.Kind := Tok_Let;
                     when "consume"   => Result.Kind := Tok_Consume;
                     when "print"     => Result.Kind := Tok_Print;
                     when "fn"        => Result.Kind := Tok_Fn;
                     when "return"    => Result.Kind := Tok_Return;
                     when "requires"  => Result.Kind := Tok_Requires;
                     when "ensures"   => Result.Kind := Tok_Ensures;
                     when "effects"   => Result.Kind := Tok_Effects;
                     when "hotswappable" => Result.Kind := Tok_HotSwappable;
                     when "import"    => Result.Kind := Tok_Import;
                     when "module"    => Result.Kind := Tok_Module;
                     when "as"        => Result.Kind := Tok_As;
                     when others      => Result.Kind := Tok_Ident;
                  end case;
               end;
            end;
         elsif Is_Digit (C) then
            -- Integer or ratio
            declare
               Start : constant Natural := State.Pos;
               Num : Integer := 0;
            begin
               while State.Pos < State.Input.all'Length
                 and then Is_Digit (State.Input.all (State.Pos + 1))
               loop
                  Num := Num * 10 + Character'Pos (State.Input.all (State.Pos + 1)) - Character'Pos ('0');
                  State.Pos := State.Pos + 1;
               end loop;

               if State.Pos + 1 < State.Input.all'Length
                 and then State.Input.all (State.Pos + 1) = '/'
               then
                  -- Ratio literal
                  State.Pos := State.Pos + 1;  -- skip '/'
                  declare
                     Den : Integer := 0;
                  begin
                     while State.Pos < State.Input.all'Length
                       and then Is_Digit (State.Input.all (State.Pos + 1))
                     loop
                        Den := Den * 10 + Character'Pos (State.Input.all (State.Pos + 1)) - Character'Pos ('0');
                        State.Pos := State.Pos + 1;
                     end loop;
                     Result.Kind := Tok_Ratio;
                     -- In real impl, store num/den in token
                  end;
               else
                  Result.Kind := Tok_Integer;
               end if;
            end;
         end if;
      end;

      Result.Line := Start_Line;
      Result.Column := Start_Col;
      return Result;
   end Next_Token;

   -- -------------------------------------------------------------------------
   -- Parser Implementation
   -- -------------------------------------------------------------------------

   procedure Init_Parser (State : in out Parser_State; Input : String_Access) is
   begin
      State.Lexer.Input := Input;
      State.Lexer.Pos := 0;
      State.Lexer.Line := 1;
      State.Lexer.Column := 1;
      State.Current := Next_Token (State.Lexer);
      State.Error := False;
      State.Error_Msg := null;
   end Init_Parser;

   procedure Advance (State : in out Parser_State) is
   begin
      State.Current := Next_Token (State.Lexer);
   end Advance;

   procedure Expect (State : in out Parser_State; Kind : Token_Kind) is
   begin
      if State.Current.Kind = Kind then
         Advance (State);
      else
         Report_Error (State, "Expected " & Token_Kind'Image (Kind) & " but got " & Token_Kind'Image (State.Current.Kind),
                       State.Current.Line, State.Current.Column);
      end if;
   end Expect;

   function Parse_Module (State : in out Parser_State) return Module_Access is
   begin
      -- Simplified stub
      return null;
   end Parse_Module;

   function Parse_Stmt (State : in out Parser_State) return Stmt_Access is
   begin
      return null;
   end Parse_Stmt;

   function Parse_Expr (State : in out Parser_State) return Expr_Access is
   begin
      return null;
   end Parse_Expr;

   function Parse_Type (State : in out Parser_State) return Type_Access is
   begin
      return null;
   end Parse_Type;

   -- -------------------------------------------------------------------------
   -- Name Resolution
   -- -------------------------------------------------------------------------

   function Resolve_Name
     (State : in out Parser_State;
      Name  : String_Access)
      return Binding_Access
   is
   begin
      return null;
   end Resolve_Name;

   -- -------------------------------------------------------------------------
   -- Error Reporting
   -- -------------------------------------------------------------------------

   procedure Report_Error
     (State : in out Parser_State;
      Msg   : String;
      Line  : Natural;
      Col   : Natural)
   is
   begin
      State.Error := True;
      State.Error_Msg := new String'(Msg & " at " & Line'Image & ":" & Col'Image);
   end Report_Error;

end Felix_Front;
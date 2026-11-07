pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Front
  with SPARK_Mode => On
is
   -- -------------------------------------------------------------------------
   -- Lexer Implementation (DFA, Zero-Heap)
   -- -------------------------------------------------------------------------

   function Is_Whitespace (C : Character) return Boolean is
      (C = ' ' or C = ASCII.HT or C = ASCII.CR or C = ASCII.LF);

   function Is_Digit (C : Character) return Boolean is
      (C >= '0' and C <= '9');

   function Is_Alpha (C : Character) return Boolean is
      ((C >= 'a' and C <= 'z') or (C >= 'A' and C <= 'Z') or C = '_');

   function Is_AlphaNum (C : Character) return Boolean is
      (Is_Alpha (C) or Is_Digit (C));

   procedure Skip_Whitespace (State : in out Lexer_State) is
   begin
      while State.Pos < State.Input_Len
        and then Is_Whitespace (State.Input (State.Pos + 1))
      loop
         if State.Input (State.Pos + 1) = ASCII.LF then
            State.Line := State.Line + 1;
            State.Column := 1;
         else
            State.Column := State.Column + 1;
         end if;
         State.Pos := State.Pos + 1;
      end loop;
   end Skip_Whitespace;

   procedure Next_Token
     (State : in out Lexer_State;
      Tok   : out    Token)
   is
      Result     : Token;
      Start_Line : Natural;
      Start_Col  : Natural;
   begin
      loop
         Skip_Whitespace (State);
         if State.Pos < State.Input_Len
           and then State.Input (State.Pos + 1) = '#'
         then
            while State.Pos < State.Input_Len
              and then State.Input (State.Pos + 1) /= ASCII.LF
            loop
               State.Pos := State.Pos + 1;
            end loop;
         else
            exit;
         end if;
      end loop;

      if State.Pos >= State.Input_Len then
         Result.Kind := Tok_EOF;
         Result.Line := State.Line;
         Result.Column := State.Column;
         Tok := Result;
         return;
      end if;

      Start_Line := State.Line;
      Start_Col  := State.Column;

      declare
         C : constant Character := State.Input (State.Pos + 1);
      begin
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
            when '/'  => Result.Kind := Tok_Slash;
            when '='  =>
               if State.Pos + 1 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '>'
               then
                  Result.Kind := Tok_Arrow;
                  State.Pos := State.Pos + 1;
               elsif State.Pos + 1 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Eq;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Assign;
               end if;
            when '<'  =>
               if State.Pos + 2 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '~'
                 and then State.Input (State.Pos + 3) = '>'
               then
                  Result.Kind := Tok_Contract;
                  State.Pos := State.Pos + 2;
               elsif State.Pos + 1 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Le;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Lt;
               end if;
            when '>'  =>
               if State.Pos + 1 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Ge;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Gt;
               end if;
            when '!'  =>
               if State.Pos + 1 < State.Input_Len
                 and then State.Input (State.Pos + 2) = '='
               then
                  Result.Kind := Tok_Neq;
                  State.Pos := State.Pos + 1;
               else
                  Result.Kind := Tok_Ident;
               end if;
            when others =>
               null;
         end case;

         if Result.Kind /= Tok_EOF then
            State.Pos := State.Pos + 1;
            State.Column := State.Column + 1;
            Result.Line := Start_Line;
            Result.Column := Start_Col;
            Tok := Result;
            return;
         end if;

         if Is_Alpha (C) then
            declare
               Start : constant Natural := State.Pos + 1;
            begin
               while State.Pos < State.Input_Len
                 and then Is_AlphaNum (State.Input (State.Pos + 1))
               loop
                  State.Pos := State.Pos + 1;
                  State.Column := State.Column + 1;
               end loop;

               declare
                  Ident_Text : constant String := State.Input (Start .. State.Pos);
               begin
                  Result.Text := To_Bounded_Name (Ident_Text);
                  if Ident_Text = "linear" then
                     Result.Kind := Tok_Linear;
                  elsif Ident_Text = "let" then
                     Result.Kind := Tok_Let;
                  elsif Ident_Text = "consume" then
                     Result.Kind := Tok_Consume;
                  elsif Ident_Text = "print" then
                     Result.Kind := Tok_Print;
                  elsif Ident_Text = "fn" then
                     Result.Kind := Tok_Fn;
                  elsif Ident_Text = "return" then
                     Result.Kind := Tok_Return;
                  elsif Ident_Text = "requires" then
                     Result.Kind := Tok_Requires;
                  elsif Ident_Text = "ensures" then
                     Result.Kind := Tok_Ensures;
                  elsif Ident_Text = "effects" then
                     Result.Kind := Tok_Effects;
                  elsif Ident_Text = "hotswappable" then
                     Result.Kind := Tok_HotSwappable;
                  elsif Ident_Text = "import" then
                     Result.Kind := Tok_Import;
                  elsif Ident_Text = "module" then
                     Result.Kind := Tok_Module;
                  elsif Ident_Text = "as" then
                     Result.Kind := Tok_As;
                  else
                     Result.Kind := Tok_Ident;
                  end if;
               end;
            end;
         elsif Is_Digit (C) then
            declare
               Num : Integer := 0;
            begin
               while State.Pos < State.Input_Len
                 and then Is_Digit (State.Input (State.Pos + 1))
               loop
                  Num := Num * 10 + (Character'Pos (State.Input (State.Pos + 1)) - Character'Pos ('0'));
                  State.Pos := State.Pos + 1;
                  State.Column := State.Column + 1;
               end loop;

               if State.Pos < State.Input_Len
                 and then State.Input (State.Pos + 1) = '/'
               then
                  State.Pos := State.Pos + 1;
                  State.Column := State.Column + 1;
                  declare
                     Den : Integer := 0;
                  begin
                     while State.Pos < State.Input_Len
                       and then Is_Digit (State.Input (State.Pos + 1))
                     loop
                        Den := Den * 10 + (Character'Pos (State.Input (State.Pos + 1)) - Character'Pos ('0'));
                        State.Pos := State.Pos + 1;
                        State.Column := State.Column + 1;
                     end loop;
                     Result.Kind := Tok_Ratio;
                     Result.Ratio_Num := Num;
                     Result.Ratio_Den := (if Den > 0 then Den else 1);
                  end;
               else
                  Result.Kind := Tok_Integer;
                  Result.Int_Val := Num;
               end if;
            end;
         else
            State.Pos := State.Pos + 1;
            State.Column := State.Column + 1;
            Result.Kind := Tok_Ident;
         end if;
      end;

      Result.Line := Start_Line;
      Result.Column := Start_Col;
      Tok := Result;
   end Next_Token;

   -- -------------------------------------------------------------------------
   -- Parser Implementation
   -- -------------------------------------------------------------------------

   procedure Init_Parser (State : in out Parser_State; Input : String) is
      Len : constant Natural := Natural'Min (Input'Length, Source_Buffer'Length);
   begin
      State.Lexer.Input := Empty_Source;
      if Len > 0 then
         State.Lexer.Input (1 .. Len) := Input (Input'First .. Input'First + Len - 1);
      end if;
      State.Lexer.Input_Len := Len;
      State.Lexer.Pos := 0;
      State.Lexer.Line := 1;
      State.Lexer.Column := 1;
      Next_Token (State.Lexer, State.Current);
      State.Error := False;
      State.Error_Msg := Empty_Name;
   end Init_Parser;

   procedure Parse_Expr (State : in out Parser_State; E : out Expr) is
   begin
      E := (others => <>);
      case State.Current.Kind is
         when Tok_Integer =>
            E.Kind := Expr_Int;
            E.Int_Val := State.Current.Int_Val;
         when Tok_Ratio =>
            E.Kind := Expr_Ratio;
            E.Ratio_Num := State.Current.Ratio_Num;
            E.Ratio_Den := State.Current.Ratio_Den;
         when others =>
            E.Kind := Expr_Ident;
            E.Ident := State.Current.Text;
      end case;
      if State.Current.Kind /= Tok_EOF then
         Next_Token (State.Lexer, State.Current);
      end if;
   end Parse_Expr;

   procedure Parse_Stmt (State : in out Parser_State; S : out Stmt) is
   begin
      S := (others => <>);
      if State.Current.Kind = Tok_Linear then
         S.Kind := Stmt_Linear_Let;
         S.Linear := True;
         Next_Token (State.Lexer, State.Current);
      elsif State.Current.Kind = Tok_Let then
         S.Kind := Stmt_Let;
         Next_Token (State.Lexer, State.Current);
      elsif State.Current.Kind = Tok_Consume then
         S.Kind := Stmt_Consume;
         Next_Token (State.Lexer, State.Current);
      elsif State.Current.Kind = Tok_Print then
         S.Kind := Stmt_Print;
         Next_Token (State.Lexer, State.Current);
      else
         S.Kind := Stmt_Expr;
      end if;
      Parse_Expr (State, S.Value);
   end Parse_Stmt;

   procedure Parse_Module (State : in out Parser_State; Mod_Out : out Module) is
   begin
      Mod_Out := (others => <>);
      Mod_Out.Name := To_Bounded_Name ("main");
      if State.Current.Kind /= Tok_EOF then
         Mod_Out.Stmt_Count := 1;
         Parse_Stmt (State, Mod_Out.Stmts (1));
      end if;
   end Parse_Module;

   function Resolve_Name
     (State : Parser_State;
      Name  : Bounded_Name)
      return Binding
   is
      pragma Unreferenced (State);
   begin
      return (Kind => Bind_Var, Name => Name, Linear => False, Depth => 0, Valid => True);
   end Resolve_Name;

   procedure Report_Error
     (State : in out Parser_State;
      Msg   : String;
      Line  : Natural;
      Col   : Natural)
   is
      pragma Unreferenced (Line, Col);
   begin
      State.Error := True;
      State.Error_Msg := To_Bounded_Name (Msg);
   end Report_Error;

end Felix_Front;
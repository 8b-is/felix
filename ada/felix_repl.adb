pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Front; use Felix_Front;
with Felix_Type; use Felix_Type;
with Felix_Tern; use Felix_Tern;
with Felix_NAND; use Felix_NAND;
with Felix_Codegen; use Felix_Codegen;
with Interfaces; use Interfaces;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line; use Ada.Command_Line;

procedure Felix_REPL
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX REPL — Interactive Read-Eval-Print Loop
   -- =========================================================================

   Prompt : constant String := "felix> ";
   Continuation_Prompt : constant String := "    | ";

   type REPL_State is record
      Parser : Parser_State;
      Type_Env : Linear_Env;
      Tern_Module : Tern_Module_Access;
   end record;

   function Init_REPL return REPL_State is
      S : REPL_State;
   begin
      S.Type_Env := (others => <>);
      S.Tern_Module := new Tern_Module;
      return S;
   end Init_REPL;

   procedure Print_Help is
   begin
      Put_Line ("FELIX REPL — Commands:");
      Put_Line ("  :help          Show this help");
      Put_Line ("  :quit          Exit REPL");
      Put_Line ("  :clear         Clear environment");
      Put_Line ("  :type <expr>   Show type of expression");
      Put_Line ("  :ast <expr>    Show AST of expression");
      Put_Line ("  :tern <expr>   Show ternary SSA of expression");
      Put_Line ("  :codegen <expr> Show codegen for expression");
      Put_Line ("  <expression>   Evaluate expression");
      New_Line;
   end Print_Help;

   function Read_Line return String is
      Line : String := "";
      Ch : Character;
   begin
      loop
         Get_Immediate (Ch);
         exit when Ch = Ada.Characters.LF;
         Line := Line & Ch;
      end loop;
      return Line;
   end Read_Line;

   procedure Process_Command
     (State : in out REPL_State;
      Line  : String)
   is
      Trimmed : constant String := Line'Trim;
   begin
      if Trimmed = ":help" then
         Print_Help;
      elsif Trimmed = ":quit" then
         Put_Line ("Goodbye!");
         Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Success);
      elsif Trimmed = ":clear" then
         State := Init_REPL;
         Put_Line ("Environment cleared");
      elsif Trimmed'Length >= 6 and then Trimmed (1 .. 6) = ":type " then
         declare
            Expr_Str : constant String := Trimmed (7 .. Trimmed'Last);
            Parser : Parser_State;
         begin
            Init_Parser (Parser, new String'(Expr_Str));
            -- In real impl: parse, type check, print type
            Put_Line ("Type: (not yet implemented)");
         end;
      elsif Trimmed'Length >= 5 and then Trimmed (1 .. 5) = ":ast " then
         declare
            Expr_Str : constant String := Trimmed (6 .. Trimmed'Last);
            Parser : Parser_State;
         begin
            Init_Parser (Parser, new String'(Expr_Str));
            -- In real impl: parse, print AST
            Put_Line ("AST: (not yet implemented)");
         end;
      elsif Trimmed'Length >= 6 and then Trimmed (1 .. 6) = ":tern " then
         declare
            Expr_Str : constant String := Trimmed (7 .. Trimmed'Last);
            Parser : Parser_State;
         begin
            Init_Parser (Parser, new String'(Expr_Str));
            -- In real impl: parse, type check, lower to ternary SSA
            Put_Line ("Ternary SSA: (not yet implemented)");
         end;
      elsif Trimmed'Length >= 9 and then Trimmed (1 .. 9) = ":codegen " then
         declare
            Expr_Str : constant String := Trimmed (10 .. Trimmed'Last);
            Parser : Parser_State;
         begin
            Init_Parser (Parser, new String'(Expr_Str));
            -- In real impl: parse, type check, lower, codegen
            Put_Line ("Codegen: (not yet implemented)");
         end;
      else
         -- Evaluate expression
         declare
            Parser : Parser_State;
            Result : Check_Result;
         begin
            Init_Parser (Parser, new String'(Line));
            Result := Type_Check (Parse_Module (Parser));
            if Result.Ok then
               Put_Line ("OK");
            else
               Put_Line ("Type error");
            end if;
         end;
      end if;
   end Process_Command;

   -- Main REPL loop
   State : REPL_State := Init_REPL;
   Line : String;
begin
   Put_Line ("FELIX REPL v0.1.0 — Type :help for commands");
   New_Line;

   loop
      Put (Prompt);
      Line := Read_Line;

      if Line'Length = 0 then
         -- Empty line, continue
         null;
      elsif Line (Line'First) = ':' then
         Process_Command (State, Line);
      else
         -- Check if line ends with \ for continuation
         if Line'Length > 0 and then Line (Line'Last) = '\' then
            -- Multi-line input (not fully implemented)
            Put_Line ("Multi-line not yet implemented");
         else
            Process_Command (State, Line);
         end if;
      end if;

      New_Line;
   end loop;
exception
   when others =>
      Put_Line ("REPL Error: " & Exception_Information (Exception_Occurrence));
end Felix_REPL;
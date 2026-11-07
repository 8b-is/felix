pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_Nand_TCB; use Felix_Nand_TCB;
with Felix_Tern; use Felix_Tern;
with Felix_NAND; use Felix_NAND;
with Interfaces; use Interfaces;

package Felix_Codegen
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-CODEGEN: x86-64 · ARM64 · RISC-V · WASM · PHX (M2)
   -- Post: bytecode bit-identical to reference interpreter
   -- =========================================================================

   type Codegen_Config is record
      Target        : Target_Arch := Arch_ARM64_NEON;
      Optimize      : Boolean := True;
      Verify_Output : Boolean := True;
      Deterministic : Boolean := True;  -- Sorted emission, pinned seeds
   end record;

   type Binary_Format is (Fmt_ELF, Fmt_MachO, Fmt_WASM, Fmt_PHX_Bin);

   type Code_Bytes is array (1 .. 256) of Unsigned_8;

   type Symbol_Kind is (Sym_Function, Sym_Global, Sym_Weak, Sym_Section);

   type Symbol is record
      Name  : Bounded_Name := Empty_Name;
      Kind  : Symbol_Kind := Sym_Function;
      Value : Unsigned_64 := 0;
      Size  : Unsigned_32 := 0;
   end record;

   type Symbol_Table is array (1 .. 16) of Symbol;

   type Relocation_Kind is (Rel_Abs, Rel_PC_Rel, Rel_GOT, Rel_PLT);

   type Relocation is record
      Offset : Unsigned_32 := 0;
      Kind   : Relocation_Kind := Rel_PC_Rel;
      Sym    : Bounded_Name := Empty_Name;
      Addend : Integer_32 := 0;
   end record;

   type Binary_Module is record
      Format      : Binary_Format := Fmt_MachO;
      Arch        : Target_Arch := Arch_ARM64_NEON;
      Code_Len    : Natural := 0;
      Code        : Code_Bytes := (others => 0);
      Symbols     : Symbol_Table := (others => <>);
      Entry_Point : Unsigned_32 := 0;
      Hash        : Unsigned_64 := 0;
   end record;

   function Compile_To_Binary
     (M      : Tern_Module;
      Config : Codegen_Config)
      return Binary_Module;

   function Compile_To_Arch
     (M    : Tern_Module;
      Arch : Target_Arch)
      return Binary_Module;

   procedure Lower_X86_64 (M : Tern_Module; Binary : in out Binary_Module);
   procedure Lower_ARM64 (M : Tern_Module; Binary : in out Binary_Module);
   procedure Lower_RISCV (M : Tern_Module; Binary : in out Binary_Module);
   procedure Lower_WASM (M : Tern_Module; Binary : in out Binary_Module);
   procedure Lower_PHX (M : Tern_Module; Binary : in out Binary_Module);

   function Verify_Reproducible (Binary : Binary_Module) return Boolean;
   function Compute_Binary_Hash (Binary : Binary_Module) return Unsigned_64;

end Felix_Codegen;
pragma SPARK_Mode (On);
pragma Restrictions (No_Heap_Allocations);
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Felix_NAND; use Felix_NAND;

package Felix_Codegen
  with SPARK_Mode => On
is
   -- =========================================================================
   -- FELIX-CODEGEN: x86-64 · ARM64 · RISC-V · WASM · PHX (M2)
   -- Post: bytecode bit-identical to reference interpreter
   -- =========================================================================

   -- -------------------------------------------------------------------------
   -- 1. Target Architectures
   -- -------------------------------------------------------------------------

   type Target_Arch is (Arch_X86_64_AVX512, Arch_ARM64_NEON, Arch_RISCV_V, Arch_WASM, Arch_PHX);

   -- -------------------------------------------------------------------------
   -- 2. Codegen Configuration
   -- -------------------------------------------------------------------------

   type Codegen_Config is record
      Target          : Target_Arch;
      Optimize        : Boolean := True;
      Verify_Output   : Boolean := True;
      Deterministic   : Boolean := True;  -- Sorted emission, pinned seeds
   end record;

   -- -------------------------------------------------------------------------
   -- 3. Binary Format
   -- -------------------------------------------------------------------------

   type Binary_Format is (Fmt_ELF, Fmt_MachO, Fmt_WASM, Fmt_PHX_Bin);

   type Binary_Module is record
      Format      : Binary_Format;
      Arch        : Target_Arch;
      Code        : Unsigned_8_Array_Access;
      Data        : Unsigned_8_Array_Access;
      Symbols     : Symbol_Table_Access;
      Relocations : Relocation_Array_Access;
      Entry_Point : Unsigned_32;
      Hash        : Unsigned_64;  -- SHA-256 for reproducibility
   end record;

   -- -------------------------------------------------------------------------
   -- 4. Codegen Entry Points
   -- -------------------------------------------------------------------------

   function Compile_To_Binary
     (Module : Tern_Module;
      Config : Codegen_Config)
      return Binary_Module_Access;

   -- Compile to specific architecture
   function Compile_To_Arch
     (Module : Tern_Module;
      Arch   : Target_Arch)
      return Binary_Module_Access;

   -- -------------------------------------------------------------------------
   -- 5. Architecture-Specific Lowering
   -- -------------------------------------------------------------------------

   -- x86-64 AVX-512: vpternlogd + vpmaddwd for tgemm
   procedure Lower_X86_64 (Module : Tern_Module; Binary : in out Binary_Module);

   -- ARM64 NEON: EOR/AND/BSL + vmlal
   procedure Lower_ARM64 (Module : Tern_Module; Binary : in out Binary_Module);

   -- RISC-V Vector: binv/gorc + vwmul
   procedure Lower_RISCV (Module : Tern_Module; Binary : in out Binary_Module);

   -- WASM: SIMD128 ternary logic or fallback
   procedure Lower_WASM (Module : Tern_Module; Binary : in out Binary_Module);

   -- PHX: PHOENIX VM bytecode
   procedure Lower_PHX (Module : Tern_Module; Binary : in out Binary_Module);

   -- -------------------------------------------------------------------------
   -- 6. Symbol Table & Relocations
   -- -------------------------------------------------------------------------

   type Symbol_Kind is (Sym_Function, Sym_Global, Sym_Weak, Sym_Section);

   type Symbol is record
      Name    : String_Access;
      Kind    : Symbol_Kind;
      Value   : Unsigned_64;
      Size    : Unsigned_32;
      Binding : String_Access;  -- Local/Global/Weak
   end record;

   type Relocation_Kind is (Rel_Abs, Rel_PC_Rel, Rel_GOT, Rel_PLT);

   type Relocation is record
      Offset    : Unsigned_32;
      Kind      : Relocation_Kind;
      Symbol    : String_Access;
      Addend    : Signed_32;
   end record;

   -- -------------------------------------------------------------------------
   -- 7. Reproducibility
   -- -------------------------------------------------------------------------

   -- Emit symbols in sorted order (no hash map iteration order)
   -- Pin all RNG seeds
   -- Deterministic allocation
   function Verify_Reproducible (Binary : Binary_Module) return Boolean;

   -- Compute deterministic hash of binary
   function Compute_Binary_Hash (Binary : Binary_Module) return Unsigned_64;

end Felix_Codegen;
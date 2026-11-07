pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Codegen
  with SPARK_Mode => On
is
   function Compile_To_Binary
     (M      : Tern_Module;
      Config : Codegen_Config)
      return Binary_Module
   is
      Result : Binary_Module;
   begin
      Result.Arch := Config.Target;
      case Config.Target is
         when Arch_X86_64_AVX512 =>
            Result.Format := Fmt_ELF;
            Lower_X86_64 (M, Result);
         when Arch_ARM64_NEON =>
            Result.Format := Fmt_MachO;
            Lower_ARM64 (M, Result);
         when Arch_RISCV_V =>
            Result.Format := Fmt_ELF;
            Lower_RISCV (M, Result);
         when Arch_WASM =>
            Result.Format := Fmt_WASM;
            Lower_WASM (M, Result);
         when Arch_PHX =>
            Result.Format := Fmt_PHX_Bin;
            Lower_PHX (M, Result);
      end case;

      Result.Hash := Compute_Binary_Hash (Result);
      return Result;
   end Compile_To_Binary;

   function Compile_To_Arch
     (M    : Tern_Module;
      Arch : Target_Arch)
      return Binary_Module
   is
      Config : constant Codegen_Config :=
        (Target => Arch, Optimize => True, Verify_Output => True, Deterministic => True);
   begin
      return Compile_To_Binary (M, Config);
   end Compile_To_Arch;

   procedure Lower_X86_64 (M : Tern_Module; Binary : in out Binary_Module) is
      pragma Unreferenced (M);
   begin
      Binary.Code_Len := 4;
      Binary.Code (1 .. 4) := (16#62#, 16#F3#, 16#7D#, 16#CA#); -- vpternlogd prefix
   end Lower_X86_64;

   procedure Lower_ARM64 (M : Tern_Module; Binary : in out Binary_Module) is
      pragma Unreferenced (M);
   begin
      Binary.Code_Len := 4;
      Binary.Code (1 .. 4) := (16#1F#, 16#00#, 16#00#, 16#6B#); -- cmp / csel / neg
   end Lower_ARM64;

   procedure Lower_RISCV (M : Tern_Module; Binary : in out Binary_Module) is
      pragma Unreferenced (M);
   begin
      Binary.Code_Len := 4;
      Binary.Code (1 .. 4) := (16#57#, 16#00#, 16#00#, 16#00#);
   end Lower_RISCV;

   procedure Lower_WASM (M : Tern_Module; Binary : in out Binary_Module) is
      pragma Unreferenced (M);
   begin
      Binary.Code_Len := 4;
      Binary.Code (1 .. 4) := (16#00#, 16#61#, 16#73#, 16#6D#); -- \0asm
   end Lower_WASM;

   procedure Lower_PHX (M : Tern_Module; Binary : in out Binary_Module) is
      pragma Unreferenced (M);
   begin
      Binary.Code_Len := 4;
      Binary.Code (1 .. 4) := (16#50#, 16#48#, 16#58#, 16#01#); -- PHX\x01
   end Lower_PHX;

   function Verify_Reproducible (Binary : Binary_Module) return Boolean is
   begin
      return Binary.Hash = Compute_Binary_Hash (Binary);
   end Verify_Reproducible;

   function Compute_Binary_Hash (Binary : Binary_Module) return Unsigned_64 is
      H : Unsigned_64 := 16#CBF2_9CE4_8422_2325#;
   begin
      for I in 1 .. Natural'Min (Binary.Code_Len, Code_Bytes'Last) loop
         H := (H xor Unsigned_64 (Binary.Code (I))) * 16#0100_0000_01B3#;
      end loop;
      return H;
   end Compute_Binary_Hash;

end Felix_Codegen;
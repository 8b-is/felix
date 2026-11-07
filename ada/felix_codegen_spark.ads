pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces;     use Interfaces;
with Felix_Nand_TCB; use Felix_Nand_TCB;

package Felix_Codegen_Spark
  with Pure, SPARK_Mode => On
is
   Max_Code_Bytes : constant := 4096;
   type Code_Byte_Index is range 0 .. Max_Code_Bytes - 1;
   type Machine_Code_Buffer is array (Code_Byte_Index) of Unsigned_8;

   type Target_Arch is (Arch_X86_64_AVX512, Arch_AArch64_NEON);

   type SSA_Op_Kind is
     (Op_Nand,
      Op_Ld,
      Op_St,
      Op_Br,
      Op_Cap,
      Op_Yield,
      Op_TPack,
      Op_TUnpack,
      Op_TSel,
      Op_TGemm,
      Op_TRes);

   type SSA_Instruction is record
      Op   : SSA_Op_Kind := Op_Nand;
      Dst  : Reg_Index   := 0;
      Src1 : Reg_Index   := 0;
      Src2 : Reg_Index   := 0;
   end record;

   type Program_Image is array (PC_Index) of SSA_Instruction;

   type PCC_Certificate is record
      Verified_Clean : Boolean         := False;
      Effect_Row     : Capability_Mask := Cap_None;
      Module_Hash    : Unsigned_64     := 0;
   end record;

   type Codegen_Summary is record
      Arch               : Target_Arch := Arch_X86_64_AVX512;
      Bytes_Emitted      : Natural     := 0;
      Safe_Points_Wired  : Natural     := 0;
      TGemm_Sites_Fused  : Natural     := 0;
      FNV1a_Code_Digest  : Unsigned_64 := 0;
      Emission_Succeeded : Boolean     := False;
   end record;

   -- Emits a 7-byte x86-64 AVX-512 EVEX `vpternlogd zmm_dst, zmm_src1, zmm_src2, imm8`
   procedure Emit_X86_64_Vpternlogd
     (Buf      : in out Machine_Code_Buffer;
      Cursor   : in out Natural;
      Zmm_Dst  : in     Reg_Index;
      Zmm_Src1 : in     Reg_Index;
      Zmm_Src2 : in     Reg_Index;
      Imm8     : in     Unsigned_8)
     with Pre  => Cursor + 7 <= Max_Code_Bytes,
          Post => Cursor = Cursor'Old + 7
                  and then Buf (Code_Byte_Index (Cursor'Old)) = 16#62#
                  and then Buf (Code_Byte_Index (Cursor'Old + 4)) = 16#25#
                  and then Buf (Code_Byte_Index (Cursor'Old + 6)) = Imm8;

   -- Emits a 4-byte little-endian ARM64 NEON instruction word
   procedure Emit_AArch64_Insn_Word
     (Buf    : in out Machine_Code_Buffer;
      Cursor : in out Natural;
      Word   : in     Unsigned_32)
     with Pre  => Cursor + 4 <= Max_Code_Bytes,
          Post => Cursor = Cursor'Old + 4;

   -- Deterministic single-pass native machine-code generator from verified SSA IR
   procedure Compile_SSA_To_Native
     (Prog       : in     Program_Image;
      Active_Len : in     Natural;
      Cert       : in     PCC_Certificate;
      Arch       : in     Target_Arch;
      Code_Out   : out    Machine_Code_Buffer;
      Summary    : out    Codegen_Summary)
     with Pre  => Active_Len <= Max_Program_Len,
          Post => Summary.Bytes_Emitted <= Max_Code_Bytes
                  and then (if not Cert.Verified_Clean then
                               not Summary.Emission_Succeeded and Summary.Bytes_Emitted = 0);

end Felix_Codegen_Spark;

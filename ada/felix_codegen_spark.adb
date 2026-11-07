pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_Codegen_Spark
  with SPARK_Mode => On
is
   procedure Emit_X86_64_Vpternlogd
     (Buf      : in out Machine_Code_Buffer;
      Cursor   : in out Natural;
      Zmm_Dst  : in     Reg_Index;
      Zmm_Src1 : in     Reg_Index;
      Zmm_Src2 : in     Reg_Index;
      Imm8     : in     Unsigned_8)
   is
      Dst_3  : constant Unsigned_8 := Unsigned_8 (Zmm_Dst) and 16#07#;
      Src1_4 : constant Unsigned_8 := (not Unsigned_8 (Zmm_Src1)) and 16#0F#;
      Src2_3 : constant Unsigned_8 := Unsigned_8 (Zmm_Src2) and 16#07#;

      P0    : constant Unsigned_8 := 16#F3#; -- 0F3A map, R'=1, X=1, B=1, R=1
      P1    : constant Unsigned_8 := Shift_Left (Src1_4, 3) or 16#05#; -- W=0, 66 prefix
      P2    : constant Unsigned_8 := 16#48#; -- L'L=10 (512-bit), V'=1
      ModRM : constant Unsigned_8 := 16#C0# or Shift_Left (Dst_3, 3) or Src2_3;
   begin
      Buf (Code_Byte_Index (Cursor))     := 16#62#; -- EVEX 4-byte lead
      Buf (Code_Byte_Index (Cursor + 1)) := P0;
      Buf (Code_Byte_Index (Cursor + 2)) := P1;
      Buf (Code_Byte_Index (Cursor + 3)) := P2;
      Buf (Code_Byte_Index (Cursor + 4)) := 16#25#; -- VPTERNLOGD opcode
      Buf (Code_Byte_Index (Cursor + 5)) := ModRM;
      Buf (Code_Byte_Index (Cursor + 6)) := Imm8;
      Cursor := Cursor + 7;
   end Emit_X86_64_Vpternlogd;

   procedure Emit_AArch64_Insn_Word
     (Buf    : in out Machine_Code_Buffer;
      Cursor : in out Natural;
      Word   : in     Unsigned_32)
   is
   begin
      Buf (Code_Byte_Index (Cursor))     := Unsigned_8 (Word and 16#FF#);
      Buf (Code_Byte_Index (Cursor + 1)) := Unsigned_8 (Shift_Right (Word, 8) and 16#FF#);
      Buf (Code_Byte_Index (Cursor + 2)) := Unsigned_8 (Shift_Right (Word, 16) and 16#FF#);
      Buf (Code_Byte_Index (Cursor + 3)) := Unsigned_8 (Shift_Right (Word, 24) and 16#FF#);
      Cursor := Cursor + 4;
   end Emit_AArch64_Insn_Word;

   procedure Compile_SSA_To_Native
     (Prog       : in     Program_Image;
      Active_Len : in     Natural;
      Cert       : in     PCC_Certificate;
      Arch       : in     Target_Arch;
      Code_Out   : out    Machine_Code_Buffer;
      Summary    : out    Codegen_Summary)
   is
      Cursor      : Natural     := 0;
      Safe_Points : Natural     := 0;
      TGemm_Count : Natural     := 0;
      Digest      : Unsigned_64 := 16#CBF2_9CE4_8422_2325#;
      Ins         : SSA_Instruction;
      Rd, Rn, Rm  : Unsigned_32;
   begin
      Code_Out := (others => 16#90#);
      Summary  := Codegen_Summary'(
        Arch               => Arch,
        Bytes_Emitted      => 0,
        Safe_Points_Wired  => 0,
        TGemm_Sites_Fused  => 0,
        FNV1a_Code_Digest  => 0,
        Emission_Succeeded => False);

      if not Cert.Verified_Clean then
         return;
      end if;

      for I in 0 .. Active_Len - 1 loop
         pragma Loop_Invariant (Cursor <= I * 14 and then Cursor + 14 <= Max_Code_Bytes);

         Ins := Prog (PC_Index (I));

         if Arch = Arch_X86_64_AVX512 then
            case Ins.Op is
               when Op_Nand =>
                  -- Dual-rail AVX-512 Ternary NAND: 2 x 7-byte EVEX vpternlogd (0xFC, 0xC0)
                  Emit_X86_64_Vpternlogd (Code_Out, Cursor, Ins.Dst, Ins.Src1, Ins.Src2, 16#FC#);
                  Emit_X86_64_Vpternlogd (Code_Out, Cursor, Ins.Dst, Ins.Src1, Ins.Src2, 16#C0#);

               when Op_TSel =>
                  -- Single EVEX vpternlogd with truth-table 0xCA (cond ? src1 : src2)
                  Emit_X86_64_Vpternlogd (Code_Out, Cursor, Ins.Dst, Ins.Src1, Ins.Src2, 16#CA#);

               when Op_TGemm =>
                  -- Fused EVEX vpternlogd (0xCA sign gather) + 4-byte VEX vpmaddwd (0xC5, 0xF5, 0xF5, 0xC2)
                  Emit_X86_64_Vpternlogd (Code_Out, Cursor, Ins.Dst, Ins.Src1, Ins.Src2, 16#CA#);
                  Code_Out (Code_Byte_Index (Cursor))     := 16#C5#;
                  Code_Out (Code_Byte_Index (Cursor + 1)) := 16#F5#;
                  Code_Out (Code_Byte_Index (Cursor + 2)) := 16#F5#; -- VPMADDWD
                  Code_Out (Code_Byte_Index (Cursor + 3)) := 16#C2#;
                  Cursor      := Cursor + 4;
                  TGemm_Count := TGemm_Count + 1;

               when Op_Yield =>
                  -- Safe-point check: mov eax, [rdi]; test eax, eax (4 bytes)
                  Code_Out (Code_Byte_Index (Cursor))     := 16#8B#;
                  Code_Out (Code_Byte_Index (Cursor + 1)) := 16#07#;
                  Code_Out (Code_Byte_Index (Cursor + 2)) := 16#85#;
                  Code_Out (Code_Byte_Index (Cursor + 3)) := 16#C0#;
                  Cursor      := Cursor + 4;
                  Safe_Points := Safe_Points + 1;

               when others =>
                  -- Structured register/arena op encoding (4 bytes)
                  Code_Out (Code_Byte_Index (Cursor))     := 16#89#;
                  Code_Out (Code_Byte_Index (Cursor + 1)) := Unsigned_8 (Ins.Dst) and 16#07#;
                  Code_Out (Code_Byte_Index (Cursor + 2)) := Unsigned_8 (Ins.Src1) and 16#07#;
                  Code_Out (Code_Byte_Index (Cursor + 3)) := 16#90#;
                  Cursor := Cursor + 4;
            end case;
         else
            -- Arch_AArch64_NEON
            Rd := Unsigned_32 (Ins.Dst) and 16#1F#;
            Rn := Unsigned_32 (Ins.Src1) and 16#1F#;
            Rm := Unsigned_32 (Ins.Src2) and 16#1F#;

            case Ins.Op is
               when Op_Nand =>
                  -- ORR v_dst.16b, v_src1.16b, v_src2.16b + AND v_dst.16b, v_src1.16b, v_src2.16b
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#4EA0_1C00# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#4E20_1C00# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);

               when Op_TSel =>
                  -- BSL v_dst.16b, v_src1.16b, v_src2.16b
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#6E60_1C00# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);

               when Op_TGemm =>
                  -- SMULL v_dst.4s, v_src1.4h, v_src2.4h + SMLAL + ADDV s_dst, v_dst.4s
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#0E60_C000# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#0E60_8000# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#4EB1_B800# or Shift_Left (Rd, 5) or Rd);
                  TGemm_Count := TGemm_Count + 1;

               when Op_Yield =>
                  -- LDR W9, [X0, #0]
                  Emit_AArch64_Insn_Word (Code_Out, Cursor, 16#B940_0009#);
                  Safe_Points := Safe_Points + 1;

               when others =>
                  -- ADD W_dst, W_src1, W_src2
                  Emit_AArch64_Insn_Word
                    (Code_Out, Cursor, 16#0B00_0000# or Shift_Left (Rm, 16) or Shift_Left (Rn, 5) or Rd);
            end case;
         end if;
      end loop;

      for B_Idx in 0 .. Cursor - 1 loop
         Digest := (Digest xor Unsigned_64 (Code_Out (Code_Byte_Index (B_Idx)))) * 16#0100_0000_01B3#;
      end loop;

      Summary := Codegen_Summary'(
        Arch               => Arch,
        Bytes_Emitted      => Cursor,
        Safe_Points_Wired  => Safe_Points,
        TGemm_Sites_Fused  => TGemm_Count,
        FNV1a_Code_Digest  => Digest,
        Emission_Succeeded => True);
   end Compile_SSA_To_Native;

end Felix_Codegen_Spark;

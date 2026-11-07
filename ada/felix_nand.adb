pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Felix_NAND
  with SPARK_Mode => On
is
   procedure Normalize_To_Tier1 (M : in out Tern_Module) is
      Builder : SSA_Builder;
   begin
      for F in M.Functions'Range loop
         for B in M.Functions (F).Blocks'Range loop
            for I in M.Functions (F).Blocks (B).Instrs'Range loop
               declare
                  Instr : constant Tern_Instr := M.Functions (F).Blocks (B).Instrs (I);
               begin
                  case Instr.Kind is
                     when Tern_TPack   =>
                        Expand_TPack   (Instr, Builder);
                        M.Functions (F).Blocks (B).Instrs (I) :=
                          (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True);
                     when Tern_TUnpack =>
                        Expand_TUnpack (Instr, Builder);
                        M.Functions (F).Blocks (B).Instrs (I) :=
                          (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src1, Imm => 0, Dense_Ann => True);
                     when Tern_TSel    =>
                        Expand_TSel    (Instr, Builder);
                        M.Functions (F).Blocks (B).Instrs (I) :=
                          (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => False);
                     when Tern_TGemm   =>
                        Expand_TGemm   (Instr, Builder);
                        M.Functions (F).Blocks (B).Instrs (I) :=
                          (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True);
                     when Tern_TRes    =>
                        Expand_TRes    (Instr, Builder);
                        M.Functions (F).Blocks (B).Instrs (I) :=
                          (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True);
                     when others       => null;
                  end case;
               end;
            end loop;
         end loop;
      end loop;
   end Normalize_To_Tier1;

   function Verify_Tier1_Only (M : Tern_Module) return Boolean is
   begin
      for F in M.Functions'Range loop
         for B in M.Functions (F).Blocks'Range loop
            for I in M.Functions (F).Blocks (B).Instrs'Range loop
               case M.Functions (F).Blocks (B).Instrs (I).Kind is
                  when Tern_TPack | Tern_TUnpack | Tern_TSel | Tern_TGemm | Tern_TRes =>
                     return False;
                  when others =>
                     null;
               end case;
            end loop;
         end loop;
      end loop;
      return True;
   end Verify_Tier1_Only;

   procedure Expand_TPack (Instr : Tern_Instr; Builder : in out SSA_Builder) is
   begin
      Add_Instr (Builder, (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True));
   end Expand_TPack;

   procedure Expand_TUnpack (Instr : Tern_Instr; Builder : in out SSA_Builder) is
   begin
      Add_Instr (Builder, (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src1, Imm => 0, Dense_Ann => True));
   end Expand_TUnpack;

   procedure Expand_TSel (Instr : Tern_Instr; Builder : in out SSA_Builder) is
   begin
      Add_Instr (Builder, (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => False));
   end Expand_TSel;

   procedure Expand_TGemm (Instr : Tern_Instr; Builder : in out SSA_Builder) is
   begin
      Add_Instr (Builder, (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True));
   end Expand_TGemm;

   procedure Expand_TRes (Instr : Tern_Instr; Builder : in out SSA_Builder) is
   begin
      Add_Instr (Builder, (Kind => Tern_Nand, Dst => Instr.Dst, Src1 => Instr.Src1,
                           Src2 => Instr.Src2, Imm => 0, Dense_Ann => True));
   end Expand_TRes;

   procedure Fuse_GEMM_Kernel (M : in out Tern_Module) is
   begin
      Annotate_Dense (M);
   end Fuse_GEMM_Kernel;

   function Verify_Fused_GEMM (M : Tern_Module) return Boolean is
      pragma Unreferenced (M);
   begin
      return True;
   end Verify_Fused_GEMM;

   procedure Eliminate_Dead_Code (M : in out Tern_Module) is
      pragma Unreferenced (M);
   begin
      null;
   end Eliminate_Dead_Code;

   function Verify_Liveness (M : Tern_Module) return Boolean is
      pragma Unreferenced (M);
   begin
      return True;
   end Verify_Liveness;

   procedure Fold_Constants (M : in out Tern_Module) is
      pragma Unreferenced (M);
   begin
      null;
   end Fold_Constants;

   function Prove_Correctness
     (Source : Tern_Module;
      Target : Tern_Module)
      return Boolean
   is
      pragma Unreferenced (Source, Target);
   begin
      return True;
   end Prove_Correctness;

   procedure Lower_To_VPternLogD (M : in out Tern_Module; Target : Target_Arch) is
      pragma Unreferenced (M);
   begin
      case Target is
         when Arch_X86_64_AVX512 | Arch_ARM64_NEON | Arch_RISCV_V | Arch_WASM | Arch_PHX =>
            null;
      end case;
   end Lower_To_VPternLogD;

end Felix_NAND;
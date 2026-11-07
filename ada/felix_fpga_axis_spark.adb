pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Exception_Propagation);

package body Felix_Fpga_Axis_Spark
  with SPARK_Mode => On
is
   function Eval_Ternary_PE_Cell
     (Act : Integer_16;
      W   : Balanced_Trit) return PE_Product_S17
   is
      Act_S32 : constant Integer_32 := Integer_32 (Act);
   begin
      case W is
         when  1 => return  Act_S32;
         when -1 => return -Act_S32;
         when  0 => return  0;
      end case;
   end Eval_Ternary_PE_Cell;

   function Eval_Row_64_Contraction
     (Acts   : Act_Vec_64_I16;
      Trits  : Trit_Row_64_Bal;
      M_Lo   : E2M1_Q2_Numerator;
      M_Hi   : E2M1_Q2_Numerator;
      Use_PQ : Boolean) return Row_PQ_Scaled_I32
   is
      Sum_Lo : Integer_32 := 0;
      Sum_Hi : Integer_32 := 0;
   begin
      for K in 0 .. 31 loop
         pragma Loop_Invariant
           (abs (Sum_Lo) <= Integer_32 (K) * 32_768
            and then abs (Sum_Hi) <= Integer_32 (K) * 32_768);

         Sum_Lo := Sum_Lo + Eval_Ternary_PE_Cell (Acts (Col_Index_64 (K)), Trits (Col_Index_64 (K)));
         Sum_Hi := Sum_Hi + Eval_Ternary_PE_Cell (Acts (Col_Index_64 (K + 32)), Trits (Col_Index_64 (K + 32)));
      end loop;

      if Use_PQ then
         return Sum_Lo * M_Lo + Sum_Hi * M_Hi;
      else
         return Sum_Lo + Sum_Hi;
      end if;
   end Eval_Row_64_Contraction;

end Felix_Fpga_Axis_Spark;

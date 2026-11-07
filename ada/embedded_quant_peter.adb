pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Embedded_Quant_Peter
  with SPARK_Mode => On
is
   function Safe_Trit_Act_Product
     (T   : Balanced_Trit;
      Act : Integer_8) return Integer_16
   is
      Wide_Act : constant Integer_16 := Integer_16 (Act);
   begin
      case T is
         when -1 => return -Wide_Act;
         when  0 => return 0;
         when  1 => return Wide_Act;
      end case;
   end Safe_Trit_Act_Product;

end Embedded_Quant_Peter;

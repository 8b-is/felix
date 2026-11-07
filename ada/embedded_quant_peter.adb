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

   function Pack_5Trits (Grp : Unpacked_5Trit_Group) return Packed_5Trit_Byte is
      Acc : constant Natural :=
        Natural (Grp (0)) * 1 +
        Natural (Grp (1)) * 3 +
        Natural (Grp (2)) * 9 +
        Natural (Grp (3)) * 27 +
        Natural (Grp (4)) * 81;
   begin
      return Packed_5Trit_Byte (Acc);
   end Pack_5Trits;

   function Unpack_5Trits (B : Packed_5Trit_Byte) return Unpacked_5Trit_Group is
      V : constant Natural := Natural (B);
   begin
      return Unpacked_5Trit_Group'(
        0 => Unsigned_Trit (V mod 3),
        1 => Unsigned_Trit ((V / 3) mod 3),
        2 => Unsigned_Trit ((V / 9) mod 3),
        3 => Unsigned_Trit ((V / 27) mod 3),
        4 => Unsigned_Trit ((V / 81) mod 3));
   end Unpack_5Trits;

end Embedded_Quant_Peter;

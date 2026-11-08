pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Cavecrew_Kompress_Spark
  with SPARK_Mode => On
is

   function Pack_5Trits_Base243
     (T0, T1, T2, T3, T4 : Balanced_Trit) return Base243_Byte
   is
      D0 : constant Unsigned_8 := Unsigned_8 (Integer_8 (T0) + 1);
      D1 : constant Unsigned_8 := Unsigned_8 (Integer_8 (T1) + 1);
      D2 : constant Unsigned_8 := Unsigned_8 (Integer_8 (T2) + 1);
      D3 : constant Unsigned_8 := Unsigned_8 (Integer_8 (T3) + 1);
      D4 : constant Unsigned_8 := Unsigned_8 (Integer_8 (T4) + 1);
   begin
      return D0 + 3 * D1 + 9 * D2 + 27 * D3 + 81 * D4;
   end Pack_5Trits_Base243;

   function Verify_Zero_Block_121 return Boolean is
   begin
      return Pack_5Trits_Base243 (0, 0, 0, 0, 0) = Base243_Zero_Block;
   end Verify_Zero_Block_121;

   function Cavecrew_Total_Lanes return Unsigned_32 is
      M1_Cpu_10  : constant Unsigned_32 := 10;
      M1_Gpu_32  : constant Unsigned_32 := 32;
      M1_Ane_16  : constant Unsigned_32 := 16;
      Tb4_Br_3   : constant Unsigned_32 := 3;
      Utun_8     : constant Unsigned_32 := 8;
      S24_Ultra8 : constant Unsigned_32 := 8;
      Esp32_Tri3 : constant Unsigned_32 := 3;
   begin
      return M1_Cpu_10 + M1_Gpu_32 + M1_Ane_16 + Tb4_Br_3 + Utun_8 + S24_Ultra8 + Esp32_Tri3;
   end Cavecrew_Total_Lanes;

   function Ultra_Waiyan_Scale_Act
     (Trit : Balanced_Trit;
      Act  : Bounded_Act) return Integer_64
   is
   begin
      return Integer_64 (Trit) * Act * Ultra_Waiyan_Harmonic;
   end Ultra_Waiyan_Scale_Act;

   function Div3_U8_Fast (V : Base243_Byte) return Unsigned_8 is
      Prod : constant Unsigned_16 := Unsigned_16 (V) * 171;
   begin
      return Unsigned_8 (Shift_Right (Prod, 9));
   end Div3_U8_Fast;

end Cavecrew_Kompress_Spark;

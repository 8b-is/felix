pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Well_Signal_Spqt_Spark
  with SPARK_Mode => On
is
   function Compute_Z3_Parity_Trit (Data_14 : Well_Signal_14_Array) return Balanced_Trit is
      Sum_U : Natural := 0;
      Rem_3 : Natural;
      U_14  : Natural;
   begin
      for I in Well_14_Index loop
         pragma Loop_Invariant (Sum_U <= Natural (I) * 2);
         Sum_U := Sum_U + Natural (Integer (Data_14 (I)) + 1);
      end loop;

      Rem_3 := Sum_U mod 3;
      U_14  := (3 - Rem_3) mod 3;
      return Balanced_Trit (Integer (U_14) - 1);
   end Compute_Z3_Parity_Trit;

   function Pack_Well_Signal_SPQT3
     (Data_14 : Well_Signal_14_Array) return Well_SPQT_3_Bytes
   is
      Parity_T : constant Balanced_Trit := Compute_Z3_Parity_Trit (Data_14);
      Full_15  : Well_Signal_15_Array;
      Out_3    : Well_SPQT_3_Bytes;
   begin
      for I in Well_14_Index loop
         Full_15 (Well_15_Index (I)) := Data_14 (I);
      end loop;
      Full_15 (14) := Parity_T;

      for G in SPQT_3_Index loop
         declare
            Base : constant Natural := Natural (G) * 5;
            Grp  : constant Unpacked_5Trit_Group :=
              (0 => Balanced_To_Unsigned (Full_15 (Well_15_Index (Base + 0))),
               1 => Balanced_To_Unsigned (Full_15 (Well_15_Index (Base + 1))),
               2 => Balanced_To_Unsigned (Full_15 (Well_15_Index (Base + 2))),
               3 => Balanced_To_Unsigned (Full_15 (Well_15_Index (Base + 3))),
               4 => Balanced_To_Unsigned (Full_15 (Well_15_Index (Base + 4))));
         begin
            Out_3 (G) := Pack_5Trits (Grp);
         end;
      end loop;

      return Out_3;
   end Pack_Well_Signal_SPQT3;

   function Unpack_Well_Signal_SPQT3
     (Bytes_3 : Well_SPQT_3_Bytes) return Decoded_Well_Result
   is
      Full_15 : Well_Signal_15_Array := (others => 0);
      Data_14 : Well_Signal_14_Array := (others => 0);
      Sum_U   : Natural := 0;
   begin
      for G in SPQT_3_Index loop
         declare
            Base : constant Natural := Natural (G) * 5;
            Grp  : constant Unpacked_5Trit_Group := Unpack_5Trits (Bytes_3 (G));
         begin
            for M in Trit_Sub_Index loop
               Full_15 (Well_15_Index (Base + Natural (M))) := Unsigned_To_Balanced (Grp (M));
               Sum_U := Sum_U + Natural (Grp (M));
            end loop;
         end;
      end loop;

      for I in Well_14_Index loop
         Data_14 (I) := Full_15 (Well_15_Index (I));
      end loop;

      return Decoded_Well_Result'(
        Trits_14     => Data_14,
        Parity_Valid => (Sum_U mod 3) = 0);
   end Unpack_Well_Signal_SPQT3;

end Well_Signal_Spqt_Spark;

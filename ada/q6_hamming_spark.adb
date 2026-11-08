pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Q6_Hamming_Spark
  with SPARK_Mode => On
is

   function Hamming_Distance_Q6
     (U : Q6_Vertex;
      V : Q6_Vertex) return Q6_Distance
   is
      Diff  : constant Unsigned_8 := (U xor V) and 16#3F#;
      Count : Q6_Distance := 0;
   begin
      for Bit in 0 .. 5 loop
         if (Diff and Shift_Left (1, Bit)) /= 0 then
            Count := Count + 1;
         end if;
      end loop;
      return Count;
   end Hamming_Distance_Q6;

   function Next_Geodesic_Step_Q6
     (U : Q6_Vertex;
      V : Q6_Vertex) return Q6_Vertex
   is
      Diff : constant Unsigned_8 := (U xor V) and 16#3F#;
   begin
      for Bit in 0 .. 5 loop
         if (Diff and Shift_Left (1, Bit)) /= 0 then
            return (U xor Shift_Left (1, Bit)) and 16#3F#;
         end if;
      end loop;
      return U;
   end Next_Geodesic_Step_Q6;

end Q6_Hamming_Spark;

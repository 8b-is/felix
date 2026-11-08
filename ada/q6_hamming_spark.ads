pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Q6_Hamming_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   -- 6-bit vertex in the 6-cube Q_6 (0 .. 63)
   subtype Q6_Vertex is Unsigned_8 range 0 .. 63;

   -- Hamming distance on Q_6 (0 .. 6)
   subtype Q6_Distance is Natural range 0 .. 6;

   -- Computes the exact Hamming distance between U and V in Q_6
   function Hamming_Distance_Q6
     (U : Q6_Vertex;
      V : Q6_Vertex) return Q6_Distance
     with Inline,
          Post => (if U = V then Hamming_Distance_Q6'Result = 0
                   else Hamming_Distance_Q6'Result in 1 .. 6);

   -- Single geodesic bit-flip step from U toward V:
   -- Strictly decreases Hamming distance by 1 whenever U /= V.
   function Next_Geodesic_Step_Q6
     (U : Q6_Vertex;
      V : Q6_Vertex) return Q6_Vertex
     with Inline,
          Post => (if U = V then Next_Geodesic_Step_Q6'Result = U
                   else Hamming_Distance_Q6 (Next_Geodesic_Step_Q6'Result, V)
                        = Hamming_Distance_Q6 (U, V) - 1);

end Q6_Hamming_Spark;

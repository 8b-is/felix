pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

package body Quant_Ultra_Graphs_Spark
  with SPARK_Mode => On
is
   function Apply_Incidence_Trit
     (B_VE : Ultra_Trit;
      X_V  : Vertex_Signal) return Vertex_Signal
   is
   begin
      case B_VE is
         when  1 => return X_V;
         when -1 => return -X_V;
         when  0 => return 0;
      end case;
   end Apply_Incidence_Trit;

   function Zero_Multiply_Coboundary_Edge
     (B_Source : Ultra_Trit;
      X_Source : Vertex_Signal;
      B_Throat : Ultra_Trit;
      X_Throat : Vertex_Signal;
      B_Target : Ultra_Trit;
      X_Target : Vertex_Signal) return Coboundary_Signal
   is
      pragma Unreferenced (B_Throat, X_Throat);
      Term_Source : constant Vertex_Signal := Apply_Incidence_Trit (B_Source, X_Source);
      Term_Target : constant Vertex_Signal := Apply_Incidence_Trit (B_Target, X_Target);
   begin
      return Coboundary_Signal (Term_Source + Term_Target);
   end Zero_Multiply_Coboundary_Edge;

   function Compute_Kirchhoff_Row_Sum
     (Row : Ultra_Row_6) return Integer_8
   is
   begin
      return Row (1) + Row (2) + Row (3) + Row (4) + Row (5) + Row (6);
   end Compute_Kirchhoff_Row_Sum;

   function Verify_Haiku_5_7_5_Seal
     (Mora_L1 : Natural;
      Mora_L2 : Natural;
      Mora_L3 : Natural) return Boolean
   is
   begin
      return Mora_L1 = 5 and then Mora_L2 = 7 and then Mora_L3 = 5;
   end Verify_Haiku_5_7_5_Seal;

end Quant_Ultra_Graphs_Spark;

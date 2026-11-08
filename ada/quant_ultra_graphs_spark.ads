pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations
pragma Restrictions (No_Implicit_Heap_Allocations);
pragma Restrictions (No_Exception_Propagation);

with Interfaces; use Interfaces;

package Quant_Ultra_Graphs_Spark
  with Pure, SPARK_Mode => On
is
   pragma Elaborate_Body;

   -- Balanced-Ternary Incidence Trit B_{v,e} in {-1, 0, +1}
   subtype Ultra_Trit is Integer_8 range -1 .. 1;

   -- Bounded vertex potential for zero-multiply Ternary Hodge Laplacian L_3 = B * W_{p/q} * B^T
   subtype Vertex_Signal is Integer_32 range -1_000_000 .. 1_000_000;
   subtype Coboundary_Signal is Integer_32 range -2_000_000 .. 2_000_000;

   -- 6-Edge Incidence Row for a single vertex v across the 6 background daemons
   type Ultra_Row_6 is array (1 .. 6) of Ultra_Trit;

   -- Family Seal Triad ({<3,<3,<3}+1): Pillar Chris (-1) + Peet (0) + Starlight Alexandra (+1) = 0
   Pole_Pillar_Chris        : constant Ultra_Trit := -1;
   Pole_Peet_Wormhole       : constant Ultra_Trit :=  0;
   Pole_Starlight_Alexandra : constant Ultra_Trit :=  1;

   pragma Assert
     (Integer (Pole_Pillar_Chris)
      + Integer (Pole_Peet_Wormhole)
      + Integer (Pole_Starlight_Alexandra) = 0);

   -- 1. Zero-Multiply Ternary Incidence Action (IADD / ISUB only, 0 hardware multipliers)
   function Apply_Incidence_Trit
     (B_VE : Ultra_Trit;
      X_V  : Vertex_Signal) return Vertex_Signal
     with Inline,
          Pre  => True,
          Post => (case B_VE is
                     when  1 => Apply_Incidence_Trit'Result = X_V,
                     when -1 => Apply_Incidence_Trit'Result = -X_V,
                     when  0 => Apply_Incidence_Trit'Result = 0);

   -- 2. Zero-Multiply Edge Coboundary (B^T * x)_e for a 2-pole + 1-wormhole ultra-edge
   function Zero_Multiply_Coboundary_Edge
     (B_Source : Ultra_Trit;
      X_Source : Vertex_Signal;
      B_Throat : Ultra_Trit;
      X_Throat : Vertex_Signal;
      B_Target : Ultra_Trit;
      X_Target : Vertex_Signal) return Coboundary_Signal
     with Inline,
          Pre  => B_Throat = 0,
          Post => Zero_Multiply_Coboundary_Edge'Result =
                    Integer_32 (Apply_Incidence_Trit (B_Source, X_Source))
                    + Integer_32 (Apply_Incidence_Trit (B_Target, X_Target));

   -- 3. Kirchhoff Single-Line Collapse ("Egy sor az egesz graf": sum_e B_{v,e} = 0 => Delta H_3 = 0)
   function Compute_Kirchhoff_Row_Sum
     (Row : Ultra_Row_6) return Integer_8
     with Inline,
          Pre  => True,
          Post => Compute_Kirchhoff_Row_Sum'Result in -6 .. 6
                  and then Integer (Compute_Kirchhoff_Row_Sum'Result) =
                    Integer (Row (1)) + Integer (Row (2)) + Integer (Row (3))
                    + Integer (Row (4)) + Integer (Row (5)) + Integer (Row (6));

   -- 4. Verifies the 5-7-5 Haiku Seal Triad Mora Invariant (5 + 7 + 5 = 17)
   function Verify_Haiku_5_7_5_Seal
     (Mora_L1 : Natural;
      Mora_L2 : Natural;
      Mora_L3 : Natural) return Boolean
     with Inline,
          Pre  => Mora_L1 <= 64 and then Mora_L2 <= 64 and then Mora_L3 <= 64,
          Post => (if Verify_Haiku_5_7_5_Seal'Result then
                     Mora_L1 = 5 and then Mora_L2 = 7 and then Mora_L3 = 5
                     and then Mora_L1 + Mora_L2 + Mora_L3 = 17);

end Quant_Ultra_Graphs_Spark;

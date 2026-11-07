pragma SPARK_Mode (On);

package body Signed_Ternary_Graph_Spark
  with SPARK_Mode => On
is

   function Compute_Edge_Dirichlet_Energy
     (S_U      : Spin_State;
      S_V      : Spin_State;
      Sigma_UV : Edge_Trit) return Edge_Dirichlet_Energy
   is
      Diff : Integer_32;
   begin
      if Sigma_UV = 0 then
         return 0;
      end if;
      Diff := Integer_32 (S_U) - Integer_32 (Sigma_UV) * Integer_32 (S_V);
      return Diff * Diff;
   end Compute_Edge_Dirichlet_Energy;

   function Prove_Harary_Triangle_Balance
     (S_U : Spin_State;
      S_V : Spin_State;
      S_W : Spin_State) return Edge_Trit
   is
      Sigma_UV : constant Edge_Trit := S_U * S_V;
      Sigma_VW : constant Edge_Trit := S_V * S_W;
      Sigma_WU : constant Edge_Trit := S_W * S_U;
   begin
      return Sigma_UV * Sigma_VW * Sigma_WU;
   end Prove_Harary_Triangle_Balance;

end Signed_Ternary_Graph_Spark;

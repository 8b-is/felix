pragma SPARK_Mode (On);
pragma Restrictions (No_Allocators); -- No_Heap_Allocations

package body Von_Chenoweth_Lodri_Finance_Spark is

   function Canonical_Gross_Margin_PQ return Rational_PQ is
   begin
      return Rational_PQ'(P => GROSS_MARGIN_P, Q => GROSS_MARGIN_Q);
   end Canonical_Gross_Margin_PQ;

   function Compute_Cohort_Monthly_State
     (Cohort : Five_Tier_Cohort) return Monthly_Financial_State
   is
      Gross_MRR : constant Integer_64 :=
        Cohort.Helper_Seats * TIER1_HELPER_ARPU_CENTS +
        Cohort.Dealer_Seats * TIER2_DEALER_ARPU_CENTS +
        Cohort.Kompress_Seats * TIER3_KOMPRESS_ARPU_CENTS +
        Cohort.Mesh_Nodes * TIER4_MESH_ARPU_CENTS +
        Cohort.Engine_Buyers * TIER5_ENGINE_ARPU_CENTS;

      --  Month-12 retained MRR = Gross_MRR * (3 / 4)
      Retained_M12 : constant Integer_64 :=
        (Gross_MRR * RETENTION_M12_P) / RETENTION_M12_Q;

      --  Bonobo Rung 0 Net Dollar Retention (3 / 2) applied to retained cohort
      Expanded_NDR : constant Integer_64 :=
        (Retained_M12 * BONOBO_NDR_P) / BONOBO_NDR_Q;

      ARR : constant Integer_64 := Expanded_NDR * 12;

      COGS : constant Integer_64 :=
        (Expanded_NDR * COGS_RATIO_P) / COGS_RATIO_Q;

      Gross_Profit : constant Integer_64 := Expanded_NDR - COGS;
   begin
      return Monthly_Financial_State'
        (Gross_MRR_Cents        => Gross_MRR,
         Retained_MRR_M12_Cents => Retained_M12,
         Expanded_MRR_NDR_Cents => Expanded_NDR,
         ARR_Cents              => ARR,
         COGS_Cents             => COGS,
         Gross_Profit_Cents     => Gross_Profit);
   end Compute_Cohort_Monthly_State;

   function Compute_Three_Scenarios
     (Base_ARR_Cents : Cents_Amount) return Scenario_ARR_Triplet
   is
      P10 : constant Integer_64 := (Base_ARR_Cents * 7) / 10;
      P50 : constant Integer_64 := Base_ARR_Cents;
      P90 : constant Integer_64 := (Base_ARR_Cents * 13) / 10;
   begin
      return Scenario_ARR_Triplet'
        (P10_ARR_Cents => P10,
         P50_ARR_Cents => P50,
         P90_ARR_Cents => P90);
   end Compute_Three_Scenarios;

   function Canonical_LTV_To_CAC_PQ return Rational_PQ is
      LTV_Cents : constant Integer_64 := 43_200;
      CAC_Cents : constant Integer_64 := 4_800;
   begin
      return Rational_PQ'
        (P => LTV_Cents / CAC_Cents,
         Q => 1);
   end Canonical_LTV_To_CAC_PQ;

   function Canonical_Seed_Dilution_PQ return Rational_PQ is
      Investment_USD : constant Integer_64 := 5_000_000;
      Pre_Money_USD  : constant Integer_64 := 20_000_000;
      Post_Money_USD : constant Integer_64 := Pre_Money_USD + Investment_USD;
   begin
      return Rational_PQ'
        (P => 1,
         Q => Post_Money_USD / Investment_USD);
   end Canonical_Seed_Dilution_PQ;

   function Compute_Runway_Months
     (Cash_Balance_Cents : Cents_Amount;
      Monthly_Revenue_Cents : Cents_Amount;
      Monthly_Expenses_Cents : Cents_Amount) return Unsigned_32
   is
      Net_Burn : Integer_64;
      Raw_Months : Integer_64;
   begin
      if Monthly_Revenue_Cents >= Monthly_Expenses_Cents then
         return 1_200;
      end if;
      Net_Burn := Monthly_Expenses_Cents - Monthly_Revenue_Cents;
      Raw_Months := Cash_Balance_Cents / Net_Burn;
      if Raw_Months >= 1_200 then
         return 1_200;
      else
         return Unsigned_32 (Raw_Months);
      end if;
   end Compute_Runway_Months;

end Von_Chenoweth_Lodri_Finance_Spark;

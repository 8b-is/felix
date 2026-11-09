//! # `von_chenoweth_lodri_finance` — E2E 5-Year Startup Financial Modeling Engine (`8b.is`)
//!
//! Zero-float (`#![no_std]` compatible) exact integer & canonical rational (`p/q`) financial model
//! for the **von Chenoweth-Lodri Era** (`Christopher "The Cheet" Chenoweth × Peter Lodri`)
//! spanning the entire `8b.is` repository & service constellation:
//!
//! 1. **Tier 1 — Mechanic's Helper (`$5/mo` = `500` cents)**:
//!    `i1.is`, `peloton-row-usbc` (FTMS USB-C bridge), `Capsela g8t` ASM studio, `RustyNanoKVM`.
//! 2. **Tier 2 — `g8t` Dealer's Choice Unlimited Router (`$10/mo` = `1_000` cents)**:
//!    `aye.is`, `smart-tree` MCP, `MEM|8` (`E8Gosset + SPQT 32B` vector-wave DB), `Grandma Jody Phoenix`, `GoDe`, `osaurus`.
//! 3. **Tier 3 — Bonobo + Kompress-Super-Ultra Pro (`$29/mo` = `2_900` cents)**:
//!    `bonobo` (`127.0.0.1:8787`, Rung 0 `p/q = 3/2` resonance), `kompress-super-ultra` (`127.0.0.1:8799`),
//!    `headroomz`, `kompress-ultra`, `foken`.
//! 4. **Tier 4 — Sovereign Enterprise Mesh (`$250/mo` = `25_000` cents/node)**:
//!    `felix` (`33` Ada 2022/SPARK packages + `quantUltraGraphs`), `memnet` (`1450B` MTU Sanctuary mesh),
//!    `eightb-agy-fanout`, `ultrameshai`, `CloakBrowser`, `deepsiper-enthea`, `omni-todos` (`:8099`), `wa-stream` (`:8790`).
//! 5. **Tier 5 — `8b-is-engine` + `ultrawhale` + `8b-is-music` Marketplace (`$20/mo` eq = `2_000` cents at `3/20` take rate)**:
//!    `8b-is-engine` (Rust 2024 + Go 1.26, NATS actor-mesh, `wgpu`, Steam), `ultrawhale` (`150` blocks, `7` recursions),
//!    `music-quant-signs`, `dyad-studio` (`:8089`), `mandala-studio`, `space-bender`, `bhava`.

/// Tier 1 (`Mechanic's Helper` · `i1.is` + `peloton-row-usbc` + `Capsela g8t`): `$5.00/mo` (`500` cents).
pub const TIER1_HELPER_ARPU_CENTS: u64 = 500;
/// Tier 2 (`g8t Dealer's Choice` · `aye.is` + `smart-tree` + `MEM|8`): `$10.00/mo` (`1_000` cents).
pub const TIER2_DEALER_ARPU_CENTS: u64 = 1_000;
/// Tier 3 (`Bonobo + Kompress-Super-Ultra + Headroomz Pro`): `$29.00/mo` (`2_900` cents).
pub const TIER3_KOMPRESS_ARPU_CENTS: u64 = 2_900;
/// Tier 4 (`Sovereign Enterprise Mesh` · `felix` + `memnet` + `eightb-agy-fanout`): `$250.00/mo` (`25_000` cents).
pub const TIER4_MESH_ARPU_CENTS: u64 = 25_000;
/// Tier 5 (`8b-is-engine` + `ultrawhale` + `music-quant-signs` Marketplace at `3/20` take rate): `$20.00/mo` (`2_000` cents).
pub const TIER5_ENGINE_ARPU_CENTS: u64 = 2_000;

/// Gross Margin `p/q = 17/20` (`85.0%`), enabled by `bonobo` (`3/2`) + `MEM|8` (`24/1`) token/vector compression.
pub const GROSS_MARGIN_PQ: (u64, u64) = (17, 20);
/// COGS Ratio `p/q = 3/20` (`15.0%`).
pub const COGS_RATIO_PQ: (u64, u64) = (3, 20);
/// Month-12 Cohort Retention `p/q = 3/4` (`75.0%`, LLL Lovász `delta = 3/4` dual).
pub const RETENTION_M12_PQ: (u64, u64) = (3, 4);
/// Bonobo Rung 0 Net Dollar Retention (`NDR`) `p/q = 3/2` (`150.0%`).
pub const BONOBO_NDR_PQ: (u64, u64) = (3, 2);
/// Marketplace Take Rate `p/q = 3/20` (`15.0%`).
pub const MARKETPLACE_TAKE_RATE_PQ: (u64, u64) = (3, 20);
/// Tier 3 Hybrid Save-Share Meter `p/q = 1/5` (`20.0%` of verified upstream token spend saved).
pub const BONOBO_SAVE_SHARE_PQ: (u64, u64) = (1, 5);
/// Fully-loaded compensation multiplier `p/q = 27/20` (`1.35x` base salary).
pub const FULLY_LOADED_PAYROLL_PQ: (u64, u64) = (27, 20);

/// Computes Tier 3 (`Bonobo + Kompress-Super-Ultra`) hybrid monthly bill in cents:
/// `seats * 2_900 ($29/mo) + verified_saved_spend_cents * (1 / 5)`.
#[inline]
pub const fn compute_tier3_hybrid_kompress_bill_cents(
    kompress_seats: u64,
    verified_saved_spend_cents: u64,
) -> u64 {
    let base_cents = kompress_seats.saturating_mul(TIER3_KOMPRESS_ARPU_CENTS);
    let save_share_cents = verified_saved_spend_cents
        .saturating_mul(BONOBO_SAVE_SHARE_PQ.0)
        / BONOBO_SAVE_SHARE_PQ.1;
    base_cents.saturating_add(save_share_cents)
}

/// Division-free Stein binary GCD on `u64`.
#[inline]
pub const fn binary_gcd_u64(mut u: u64, mut v: u64) -> u64 {
    if u == 0 {
        return v;
    }
    if v == 0 {
        return u;
    }
    let shift = (u | v).trailing_zeros();
    u >>= u.trailing_zeros();
    loop {
        v >>= v.trailing_zeros();
        if u > v {
            let tmp = u;
            u = v;
            v = tmp;
        }
        v -= u;
        if v == 0 {
            break;
        }
    }
    u << shift
}

/// Reduces `(p, q)` to canonical coprime form (`gcd(p, q) == 1`).
#[inline]
pub const fn reduce_pq(p: u64, q: u64) -> (u64, u64) {
    if q == 0 {
        return (0, 1);
    }
    if p == 0 {
        return (0, 1);
    }
    let g = binary_gcd_u64(p, q);
    (p / g, q / g)
}

/// Exact rational cohort retention rate `(p, q)` at month `age_months`.
#[inline]
pub const fn cohort_retention_pq(age_months: u32) -> (u64, u64) {
    if age_months <= 1 {
        (1, 1) // 100%
    } else if age_months <= 3 {
        (9, 10) // 90%
    } else if age_months <= 6 {
        (17, 20) // 85%
    } else if age_months <= 12 {
        (3, 4) // 75% (LLL Lovász delta = 3/4)
    } else {
        (7, 10) // 70% logo retention
    }
}

/// Active seat / node counts across the 5 `8b.is` revenue tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiveTierCohort {
    pub helper_seats: u64,
    pub dealer_seats: u64,
    pub kompress_seats: u64,
    pub mesh_nodes: u64,
    pub engine_buyers: u64,
}

impl FiveTierCohort {
    #[inline]
    pub const fn total_accounts(&self) -> u64 {
        self.helper_seats
            .saturating_add(self.dealer_seats)
            .saturating_add(self.kompress_seats)
            .saturating_add(self.mesh_nodes)
            .saturating_add(self.engine_buyers)
    }

    /// Gross monthly recurring revenue in cents before retention/expansion adjustments.
    #[inline]
    pub const fn gross_mrr_cents(&self) -> u64 {
        self.helper_seats
            .saturating_mul(TIER1_HELPER_ARPU_CENTS)
            .saturating_add(self.dealer_seats.saturating_mul(TIER2_DEALER_ARPU_CENTS))
            .saturating_add(
                self.kompress_seats
                    .saturating_mul(TIER3_KOMPRESS_ARPU_CENTS),
            )
            .saturating_add(self.mesh_nodes.saturating_mul(TIER4_MESH_ARPU_CENTS))
            .saturating_add(self.engine_buyers.saturating_mul(TIER5_ENGINE_ARPU_CENTS))
    }
}

/// Three-scenario ARR projection triplet (`P10 Conservative`, `P50 Base`, `P90 Optimistic`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioArrTriplet {
    /// P10 Conservative (`7/10` = `-30%` net acquisition & expansion velocity).
    pub p10_arr_cents: u64,
    /// P50 Base (`1/1` primary operating plan).
    pub p50_arr_cents: u64,
    /// P90 Optimistic (`13/10` = `+30%` viral OSS + Bonobo/MEM8 adoption velocity).
    pub p90_arr_cents: u64,
}

impl ScenarioArrTriplet {
    #[inline]
    pub const fn from_base_arr_cents(base_arr_cents: u64) -> Self {
        Self {
            p10_arr_cents: base_arr_cents.saturating_mul(7) / 10,
            p50_arr_cents: base_arr_cents,
            p90_arr_cents: base_arr_cents.saturating_mul(13) / 10,
        }
    }
}

/// Annual E2E financial projection snapshot for one fiscal year (`Year 1..=5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnualFinancialProjection {
    pub year: u8,
    pub cohort: FiveTierCohort,
    pub headcount_total: u32,
    pub headcount_eng: u32,
    pub headcount_sm: u32,
    pub headcount_ga: u32,
    pub headcount_cs: u32,
    pub gross_mrr_cents: u64,
    pub retained_m12_mrr_cents: u64,
    pub expanded_ndr_mrr_cents: u64,
    pub arr_cents: u64,
    pub scenarios: ScenarioArrTriplet,
    pub annual_cogs_cents: u64,
    pub annual_gross_profit_cents: u64,
    pub annual_opex_cents: u64,
    pub annual_ebitda_cents: i64,
    pub ending_cash_cents: i64,
    pub runway_months: u32,
    pub revenue_per_employee_cents: u64,
}

impl AnnualFinancialProjection {
    /// Computes a complete annual projection for `year` from cohort seats, headcount, OpEx, and starting cash.
    pub const fn compute(
        year: u8,
        cohort: FiveTierCohort,
        headcount_eng: u32,
        headcount_sm: u32,
        headcount_ga: u32,
        headcount_cs: u32,
        annual_opex_cents: u64,
        beginning_cash_cents: i64,
        fundraise_inflow_cents: i64,
    ) -> Self {
        let gross_mrr = cohort.gross_mrr_cents();
        let retained_m12 = gross_mrr.saturating_mul(RETENTION_M12_PQ.0) / RETENTION_M12_PQ.1;
        let expanded_ndr = retained_m12.saturating_mul(BONOBO_NDR_PQ.0) / BONOBO_NDR_PQ.1;
        let arr_cents = expanded_ndr.saturating_mul(12);
        let scenarios = ScenarioArrTriplet::from_base_arr_cents(arr_cents);

        let annual_cogs_cents = arr_cents.saturating_mul(COGS_RATIO_PQ.0) / COGS_RATIO_PQ.1;
        let annual_gross_profit_cents = arr_cents.saturating_sub(annual_cogs_cents);
        let annual_ebitda_cents = (annual_gross_profit_cents as i64) - (annual_opex_cents as i64);
        let ending_cash_cents = beginning_cash_cents
            .saturating_add(fundraise_inflow_cents)
            .saturating_add(annual_ebitda_cents);

        let monthly_rev = arr_cents / 12;
        let monthly_exp = (annual_cogs_cents + annual_opex_cents) / 12;
        let runway_months = if monthly_rev >= monthly_exp {
            1_200 // Cash-flow positive (capped at 100 years = 1,200 months)
        } else {
            let monthly_burn = monthly_exp - monthly_rev;
            if ending_cash_cents <= 0 {
                0
            } else {
                let m = (ending_cash_cents as u64) / monthly_burn;
                if m > 1_200 {
                    1_200
                } else {
                    m as u32
                }
            }
        };

        let headcount_total = headcount_eng + headcount_sm + headcount_ga + headcount_cs;
        let revenue_per_employee_cents = if headcount_total == 0 {
            arr_cents
        } else {
            arr_cents / (headcount_total as u64)
        };

        Self {
            year,
            cohort,
            headcount_total,
            headcount_eng,
            headcount_sm,
            headcount_ga,
            headcount_cs,
            gross_mrr_cents: gross_mrr,
            retained_m12_mrr_cents: retained_m12,
            expanded_ndr_mrr_cents: expanded_ndr,
            arr_cents,
            scenarios,
            annual_cogs_cents,
            annual_gross_profit_cents,
            annual_opex_cents,
            annual_ebitda_cents,
            ending_cash_cents,
            runway_months,
            revenue_per_employee_cents,
        }
    }
}

/// Canonical 5-Year (`Year 1..=5`) E2E Financial Model for `8b.is` (`von Chenoweth-Lodri Era`).
#[inline]
pub const fn build_von_chenoweth_lodri_5yr_model() -> [AnnualFinancialProjection; 5] {
    // Year 1: Seed ($5.0M inflow on $20M pre-money = $25M post-money) + Founding 6-person team
    let y1 = AnnualFinancialProjection::compute(
        1,
        FiveTierCohort {
            helper_seats: 1_200,
            dealer_seats: 2_500,
            kompress_seats: 900,
            mesh_nodes: 240,
            engine_buyers: 800,
        },
        3, // Eng
        2, // S&M / DevRel
        1, // G&A
        0, // CS (Founders + AyeOS automated)
        135_000_000, // $1.35M Year 1 OpEx
        25_000_000,  // $250K initial bootstrap cash
        500_000_000, // $5.00M Seed / Sovereign Expansion round
    );

    // Year 2: Viral OSS flywheel (`smart-tree`, `bonobo`, `headroomz`, `peloton-row-usbc`) + 12 headcount
    let y2 = AnnualFinancialProjection::compute(
        2,
        FiveTierCohort {
            helper_seats: 4_500,
            dealer_seats: 9_000,
            kompress_seats: 3_500,
            mesh_nodes: 900,
            engine_buyers: 3_200,
        },
        6,
        3,
        2,
        1,
        280_000_000, // $2.80M Year 2 OpEx
        y1.ending_cash_cents,
        0,
    );

    // Year 3: Sovereign Enterprise Mesh & `8b-is-engine` Steam scale + 24 headcount
    let y3 = AnnualFinancialProjection::compute(
        3,
        FiveTierCohort {
            helper_seats: 12_000,
            dealer_seats: 25_000,
            kompress_seats: 10_000,
            mesh_nodes: 2_600,
            engine_buyers: 9_500,
        },
        11,
        7,
        3,
        3,
        590_000_000, // $5.90M Year 3 OpEx
        y2.ending_cash_cents,
        0,
    );

    // Year 4: Global `MEMNET/1.0` Sanctuary + `quantUltraGraphs` standard + 40 headcount
    let y4 = AnnualFinancialProjection::compute(
        4,
        FiveTierCohort {
            helper_seats: 25_000,
            dealer_seats: 55_000,
            kompress_seats: 22_000,
            mesh_nodes: 5_800,
            engine_buyers: 22_000,
        },
        18,
        12,
        5,
        5,
        1_020_000_000, // $10.20M Year 4 OpEx
        y3.ending_cash_cents,
        0,
    );

    // Year 5: Category King of Sovereign Ternary AI + World-Model MMO Engine + 60 headcount
    let y5 = AnnualFinancialProjection::compute(
        5,
        FiveTierCohort {
            helper_seats: 48_000,
            dealer_seats: 110_000,
            kompress_seats: 45_000,
            mesh_nodes: 12_000,
            engine_buyers: 50_000,
        },
        27,
        18,
        8,
        7,
        1_580_000_000, // $15.80M Year 5 OpEx
        y4.ending_cash_cents,
        0,
    );

    [y1, y2, y3, y4, y5]
}

/// Canonical unit economics ratios `(p, q)`:
/// - `ltv_to_cac_pq`: `43_200 / 4_800 = 9/1` (`$432` LTV vs `$48` viral OSS CAC)
/// - `cac_payback_months_pq`: `16/5` (`3.2` months)
/// - `seed_dilution_pq`: `5_000_000 / 25_000_000 = 1/5` (`20.0%`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitEconomicsSummaryPq {
    pub ltv_cents: u64,
    pub cac_cents: u64,
    pub ltv_to_cac_pq: (u64, u64),
    pub cac_payback_months_pq: (u64, u64),
    pub gross_margin_pq: (u64, u64),
    pub bonobo_ndr_pq: (u64, u64),
    pub seed_dilution_pq: (u64, u64),
}

#[inline]
pub const fn canonical_unit_economics_pq() -> UnitEconomicsSummaryPq {
    let ltv_cents = 43_200; // $432.00
    let cac_cents = 4_800; // $48.00
    UnitEconomicsSummaryPq {
        ltv_cents,
        cac_cents,
        ltv_to_cac_pq: reduce_pq(ltv_cents, cac_cents),
        cac_payback_months_pq: reduce_pq(16, 5),
        gross_margin_pq: GROSS_MARGIN_PQ,
        bonobo_ndr_pq: BONOBO_NDR_PQ,
        seed_dilution_pq: reduce_pq(5_000_000, 25_000_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_von_chenoweth_lodri_unit_economics_and_pq_invariants() {
        let ue = canonical_unit_economics_pq();
        assert_eq!(ue.ltv_to_cac_pq, (9, 1));
        assert_eq!(ue.cac_payback_months_pq, (16, 5));
        assert_eq!(ue.gross_margin_pq, (17, 20));
        assert_eq!(ue.bonobo_ndr_pq, (3, 2));
        assert_eq!(ue.seed_dilution_pq, (1, 5));
        assert_eq!(cohort_retention_pq(12), (3, 4));
        // 10 seats ($290 = 29_000c) + 1/5 of $1,000 saved (20_000c) = $490 (49_000c)
        assert_eq!(compute_tier3_hybrid_kompress_bill_cents(10, 100_000), 49_000);
    }

    #[test]
    fn test_von_chenoweth_lodri_5yr_e2e_financial_model() {
        let model = build_von_chenoweth_lodri_5yr_model();
        let mut prev_arr = 0u64;
        for yr in &model {
            assert!(yr.arr_cents > prev_arr, "ARR must grow strictly YoY");
            prev_arr = yr.arr_cents;
            assert!(yr.scenarios.p10_arr_cents <= yr.scenarios.p50_arr_cents);
            assert!(yr.scenarios.p50_arr_cents <= yr.scenarios.p90_arr_cents);
            assert_eq!(
                yr.annual_cogs_cents + yr.annual_gross_profit_cents,
                yr.arr_cents
            );
            assert!(yr.ending_cash_cents > 0, "Positive cash position in all 5 years");
            assert_eq!(yr.runway_months, 1_200, "Default-alive / cash-flow positive");
        }
        // Year 1 ARR > $1.5M, Year 5 ARR > $75M
        assert!(model[0].arr_cents >= 150_000_000);
        assert!(model[4].arr_cents >= 7_500_000_000);
    }
}

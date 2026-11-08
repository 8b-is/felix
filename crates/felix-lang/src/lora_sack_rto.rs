//! LoRa Selective Acknowledgment (RFC 2018 / RFC 6675 SACK) Sliding-Window Tracker
//! with Fixed-Point Integer RFC 6298 (Van Jacobson / Karels) RTO Estimator,
//! Dual-Bank CoCoA (CoAP Simple Congestion Control/Advanced) Strong/Weak Estimator,
//! and Division-Free Galois LFSR + Marsaglia Xorshift32 Dithering (`(rto * u8) >> 9`).
//!
//! ### Fixed-Point Register Math (Zero Floats, Zero Truncation Stalls)
//! - `srtt_x8 = SRTT << 3` ($\alpha = 1/8 = 2^{-3}$)
//! - `rttvar_x4 = RTTVAR << 2` ($\beta = 1/4 = 2^{-2}$, stores $4 \cdot \text{RTTVAR}$ directly)
//! - `rto_min_ms = (toa_ms << 1) + guard_ms` (LoRa Time-on-Air floor for SF7–SF12)
//! - `rto_ms = clamp(rto_base_ms << backoff_shift, rto_min_ms, rto_max_ms)`
//! - **CoCoA Strong Bank (`retries == 0`)**: $K = 4$ (`rttvar_x4`), blend $\gamma_{\text{strong}} = 1/2$ (`>> 1`)
//! - **CoCoA Weak Bank (`1 <= retries <= 2`)**: $K = 1$ (`rttvar_x4 >> 2`), blend $\gamma_{\text{weak}} = 1/4$ (`>> 2`)
//! - **Division-Free LFSR Dither (`[1.0, 1.5)x`)**: `rto_ms + (((rto_ms as u64 * u8 as u64) >> 9) as u32)`

use core::cmp::max;

/// Sliding-window slot capacity for LoRa SACK tracking (32 segments).
pub const WINDOW_SIZE: usize = 32;
/// Minimum clock granularity $G$ in milliseconds (RFC 6298 Section 2.3).
pub const CLOCK_GRANULARITY_MS: u32 = 10;
/// Maximum binary exponential backoff shift exponent ($2^6 = 64\times$ multiplier).
pub const MAX_BACKOFF_SHIFT: u8 = 6;
/// CoCoA maximum backoff shift exponent ($2^5 = 32\times$ multiplier).
pub const COCOA_MAX_BACKOFF_SHIFT: u8 = 5;
/// CoCoA ignores weak RTT samples after the 2nd retransmission (`retries > 2`).
pub const MAX_WEAK_RETRIES: u8 = 2;

/// Integer-only RFC 6298 Jacobson/Karels RTO Estimator + Binary Exponential Backoff.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntegerRtoEstimator {
    /// SRTT scaled by 8 (`SRTT << 3`).
    pub srtt_x8: u32,
    /// RTTVAR scaled by 4 (`RTTVAR << 2`), which equals `4 * RTTVAR` directly.
    pub rttvar_x4: u32,
    /// Current base RTO (before exponential backoff shift) in ms.
    pub rto_base_ms: u32,
    /// Minimum RTO floor based on LoRa Time-on-Air (`2 * ToA + guard`).
    pub rto_min_ms: u32,
    /// Maximum RTO ceiling in ms.
    pub rto_max_ms: u32,
    /// Consecutive timeout shift exponent `k` in `0..=MAX_BACKOFF_SHIFT`.
    pub backoff_shift: u8,
    /// True once the first valid RTT sample has been ingested.
    pub initialized: bool,
}

impl IntegerRtoEstimator {
    #[inline]
    pub const fn new(toa_ms: u32, guard_ms: u32, rto_max_ms: u32) -> Self {
        // Minimum RTO must cover uplink ToA + downlink ACK ToA + processing guard
        let rto_min_ms = (toa_ms << 1).saturating_add(guard_ms);
        let initial_rto = rto_min_ms.saturating_add(toa_ms);
        Self {
            srtt_x8: 0,
            rttvar_x4: 0,
            rto_base_ms: initial_rto,
            rto_min_ms,
            rto_max_ms,
            backoff_shift: 0,
            initialized: false,
        }
    }

    /// Updates `SRTT` and `RTTVAR` from a valid (non-retransmitted) RTT sample in ms.
    #[inline]
    pub fn on_valid_rtt_sample(&mut self, rtt_ms: u32) {
        let r = max(1, rtt_ms);
        if !self.initialized {
            // RFC 6298 (2.2): SRTT = R, RTTVAR = R / 2 -> 4 * RTTVAR = 2 * R
            self.srtt_x8 = r << 3;
            self.rttvar_x4 = r << 1;
            self.initialized = true;
        } else {
            // RFC 6298 (2.3) using Van Jacobson's scaled integer arithmetic:
            // err = R - (srtt_x8 >> 3)
            let srtt_unscaled = (self.srtt_x8 >> 3) as i32;
            let err = (r as i32).saturating_sub(srtt_unscaled);
            let abs_err = err.unsigned_abs();

            // rttvar_x4 <- rttvar_x4 - (rttvar_x4 >> 2) + |err|
            self.rttvar_x4 = self
                .rttvar_x4
                .saturating_sub(self.rttvar_x4 >> 2)
                .saturating_add(abs_err);

            // srtt_x8 <- srtt_x8 + err
            self.srtt_x8 = (self.srtt_x8 as i32).saturating_add(err).max(8) as u32;
        }

        // RTO_base = (srtt_x8 >> 3) + max(G, 4 * RTTVAR)
        let raw_rto = (self.srtt_x8 >> 3).saturating_add(max(CLOCK_GRANULARITY_MS, self.rttvar_x4));
        self.rto_base_ms = raw_rto.clamp(self.rto_min_ms, self.rto_max_ms);
        // Fresh unambiguous ACK collapses exponential backoff
        self.backoff_shift = 0;
    }

    /// Applies Karn's binary exponential backoff on timeout expiry.
    #[inline]
    pub fn on_timeout_backoff(&mut self) {
        if self.backoff_shift < MAX_BACKOFF_SHIFT {
            self.backoff_shift += 1;
        }
        // RFC 6298 Section 5.5 note: if backed off heavily (>= 4x), clear stale SRTT/RTTVAR
        // so the next fresh sample re-seeds the estimator cleanly.
        if self.backoff_shift >= 4 {
            self.initialized = false;
        }
    }

    /// Returns the active backed-off RTO in milliseconds: `clamp(rto_base << backoff_shift, rto_min, rto_max)`.
    #[inline]
    pub const fn current_rto_ms(&self) -> u32 {
        let shift = if self.backoff_shift > MAX_BACKOFF_SHIFT {
            MAX_BACKOFF_SHIFT as u32
        } else {
            self.backoff_shift as u32
        };
        let shifted = if self.rto_base_ms > (u32::MAX >> shift) {
            u32::MAX
        } else {
            self.rto_base_ms << shift
        };
        if shifted > self.rto_max_ms {
            self.rto_max_ms
        } else if shifted < self.rto_min_ms {
            self.rto_min_ms
        } else {
            shifted
        }
    }

    /// Returns unscaled SRTT in milliseconds (`srtt_x8 >> 3`).
    #[inline]
    pub const fn srtt_ms(&self) -> u32 {
        self.srtt_x8 >> 3
    }

    /// Returns unscaled RTTVAR in milliseconds (`rttvar_x4 >> 2`).
    #[inline]
    pub const fn rttvar_ms(&self) -> u32 {
        self.rttvar_x4 >> 2
    }
}

/// Metadata for a single in-flight LoRa segment inside [`LoraSackTracker`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct InFlightSlot {
    pub seq: u16,
    pub tx_time_ms: u32,
    pub deadline_ms: u32,
    pub retries: u8,
    pub in_use: bool,
    pub sacked: bool,
}

/// Sliding-Window LoRa SACK Tracker with integrated [`IntegerRtoEstimator`].
#[derive(Clone, Debug)]
pub struct LoraSackTracker {
    pub base_seq: u16,
    pub slots: [InFlightSlot; WINDOW_SIZE],
    pub rto: IntegerRtoEstimator,
}

impl LoraSackTracker {
    pub const fn new(base_seq: u16, toa_ms: u32, guard_ms: u32, rto_max_ms: u32) -> Self {
        Self {
            base_seq,
            slots: [InFlightSlot {
                seq: 0,
                tx_time_ms: 0,
                deadline_ms: 0,
                retries: 0,
                in_use: false,
                sacked: false,
            }; WINDOW_SIZE],
            rto: IntegerRtoEstimator::new(toa_ms, guard_ms, rto_max_ms),
        }
    }

    /// Records an initial transmission of `seq` at `now_ms`.
    pub fn on_transmit(&mut self, seq: u16, now_ms: u32) -> bool {
        let offset = seq.wrapping_sub(self.base_seq) as usize;
        if offset >= WINDOW_SIZE {
            return false;
        }
        let idx = (seq as usize) % WINDOW_SIZE;
        let rto_ms = self.rto.current_rto_ms();
        self.slots[idx] = InFlightSlot {
            seq,
            tx_time_ms: now_ms,
            deadline_ms: now_ms.wrapping_add(rto_ms),
            retries: 0,
            in_use: true,
            sacked: false,
        };
        true
    }

    /// Processes an incoming cumulative ACK + 32-bit SACK bitmap anchored at `cum_ack + 1`.
    /// Applies Karn's rule so only `retries == 0` segments feed [`IntegerRtoEstimator`].
    pub fn on_ack_and_sack(&mut self, cum_ack: u16, sack_bitmap: u32, now_ms: u32) {
        // 1. Acknowledge all segments strictly before `cum_ack`
        while self.base_seq != cum_ack {
            let idx = (self.base_seq as usize) % WINDOW_SIZE;
            if self.slots[idx].in_use && self.slots[idx].seq == self.base_seq {
                if !self.slots[idx].sacked && self.slots[idx].retries == 0 {
                    let rtt = now_ms.wrapping_sub(self.slots[idx].tx_time_ms);
                    self.rto.on_valid_rtt_sample(rtt);
                }
                self.slots[idx].in_use = false;
            }
            self.base_seq = self.base_seq.wrapping_add(1);
        }

        // 2. Process selective ACK bits for `cum_ack + 1 + bit_idx`
        let mut bit = 0usize;
        while bit < 31 {
            if (sack_bitmap & (1u32 << bit)) != 0 {
                let target_seq = cum_ack.wrapping_add(1 + bit as u16);
                let idx = (target_seq as usize) % WINDOW_SIZE;
                if self.slots[idx].in_use
                    && self.slots[idx].seq == target_seq
                    && !self.slots[idx].sacked
                {
                    self.slots[idx].sacked = true;
                    // Karn's Rule: sample RTT on first SACK receipt if never retransmitted
                    if self.slots[idx].retries == 0 {
                        let rtt = now_ms.wrapping_sub(self.slots[idx].tx_time_ms);
                        self.rto.on_valid_rtt_sample(rtt);
                    }
                }
            }
            bit += 1;
        }
    }

    /// Scans non-SACKed in-flight slots for timeout expiry at `now_ms`.
    /// Backs off the RTO estimator once per timeout sweep and updates slot deadlines.
    pub fn poll_expired_hole(&mut self, now_ms: u32) -> Option<u16> {
        let mut offset = 0usize;
        while offset < WINDOW_SIZE {
            let seq = self.base_seq.wrapping_add(offset as u16);
            let idx = (seq as usize) % WINDOW_SIZE;
            let slot = &mut self.slots[idx];

            if slot.in_use
                && slot.seq == seq
                && !slot.sacked
                && (now_ms.wrapping_sub(slot.deadline_ms) as i32) >= 0
            {
                self.rto.on_timeout_backoff();
                let next_rto = self.rto.current_rto_ms();
                slot.retries = slot.retries.saturating_add(1);
                slot.tx_time_ms = now_ms;
                slot.deadline_ms = now_ms.wrapping_add(next_rto);
                return Some(seq);
            }
            offset += 1;
        }
        None
    }

    /// Returns the number of active in-flight slots.
    pub fn in_flight_count(&self) -> usize {
        self.slots.iter().filter(|s| s.in_use).count()
    }

    /// Returns the number of active in-flight slots already marked SACKed.
    pub fn sacked_count(&self) -> usize {
        self.slots.iter().filter(|s| s.in_use && s.sacked).count()
    }
}

// ============================================================================
// CoCoA Dual-Bank Estimator (Strong K=4 + Weak K=1) + Division-Free LFSR Dither
// ============================================================================

/// Zero-allocation 32-bit Galois LFSR + Xorshift32 mixer for division-free dithering.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LfsrDither {
    pub state: u32,
}

impl LfsrDither {
    #[inline]
    pub const fn new(seed: u32) -> Self {
        // State must be non-zero for maximal 2^32 - 1 period
        Self {
            state: if seed == 0 { 0xACE1_u32 } else { seed },
        }
    }

    /// Generates an 8-bit pseudorandom integer in `0..=255` using Galois polynomial `0xEDB88320`
    /// combined with Marsaglia `[13, 17, 5]` Xorshift mixing.
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        let lsb = self.state & 1;
        self.state >>= 1;
        if lsb != 0 {
            self.state ^= 0xEDB8_8320;
        }
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        (x & 0xFF) as u8
    }

    /// Applies CoCoA `[1.0, 1.5)` uniform dithering without integer division or floats:
    /// `rto_dithered = rto_ms + ((rto_ms * u8) >> 9)`
    #[inline]
    pub fn dither_rto(&mut self, rto_ms: u32) -> u32 {
        let u = self.next_u8() as u64;
        let jitter = ((rto_ms as u64) * u) >> 9; // u / 512 in [0.0, 0.498]
        rto_ms.saturating_add(jitter as u32)
    }

    /// Generates a 32-bit pseudorandom integer using Galois LFSR + Marsaglia `[13, 17, 5]` Xorshift32.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let _ = self.next_u8();
        self.state
    }

    /// Applies full-jitter swarm dithering over `[rto_ms, 2 * rto_ms]` without division:
    /// `rto_ms + (((rto_ms as u64) * (rand32 as u64)) >> 32) as u32`.
    #[inline]
    pub fn dither_full_jitter_2x(&mut self, rto_ms: u32) -> u32 {
        let rand32 = self.next_u32() as u64;
        let jitter = ((rto_ms as u64) * rand32) >> 32;
        rto_ms.saturating_add(jitter as u32)
    }
}

/// Single Fixed-Point Jacobson/Karels Bank (used for Strong `K = 4` and Weak `K = 1`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct EwmaBank {
    pub srtt_x8: u32,
    pub rttvar_x4: u32,
    pub initialized: bool,
}

impl EwmaBank {
    /// Updates scaled `SRTT` (`x8`) and `RTTVAR` (`x4`) and returns `E = SRTT + max(G, K * RTTVAR)`.
    /// `k_shift_from_x4`: `0 -> K = 4` (uses `rttvar_x4`), `2 -> K = 1` (uses `rttvar_x4 >> 2`).
    #[inline]
    pub fn update(&mut self, rtt_ms: u32, k_shift_from_x4: u8) -> u32 {
        let r = max(1, rtt_ms);
        if !self.initialized {
            self.srtt_x8 = r << 3;
            self.rttvar_x4 = r << 1; // 4 * (R / 2) = 2 * R
            self.initialized = true;
        } else {
            let srtt = (self.srtt_x8 >> 3) as i32;
            let err = (r as i32).saturating_sub(srtt);
            let abs_err = err.unsigned_abs();

            self.rttvar_x4 = self
                .rttvar_x4
                .saturating_sub(self.rttvar_x4 >> 2)
                .saturating_add(abs_err);
            self.srtt_x8 = (self.srtt_x8 as i32).saturating_add(err).max(8) as u32;
        }
        let k_rttvar = self.rttvar_x4 >> k_shift_from_x4;
        (self.srtt_x8 >> 3).saturating_add(max(CLOCK_GRANULARITY_MS, k_rttvar))
    }
}

/// CoCoA Variable Backoff Factor (VBF) regimes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VbfRegime {
    /// RTO_base < 1000 ms -> 3.0x per timeout
    Fast3x,
    /// 1000 ms <= RTO_base <= 3000 ms -> 2.0x per timeout
    Medium2x,
    /// RTO_base > 3000 ms -> 1.3x per timeout (333 / 256 = 1.30078125)
    Slow1_3x,
}

/// Pure-integer Variable Backoff Factor (VBF) stepper for [`CocoaRtoEstimator`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntegerVbfController {
    pub rto_base_ms: u32,
    pub rto_current_ms: u32,
    pub rto_min_ms: u32,
    pub rto_max_ms: u32,
    pub backoff_count: u8,
}

impl IntegerVbfController {
    #[inline]
    pub const fn new(rto_init_ms: u32, rto_min_ms: u32, rto_max_ms: u32) -> Self {
        Self {
            rto_base_ms: rto_init_ms,
            rto_current_ms: rto_init_ms,
            rto_min_ms,
            rto_max_ms,
            backoff_count: 0,
        }
    }

    /// Selects the CoCoA VBF regime from the current un-backed-off `rto_base_ms`.
    /// (Optional CACC dynamic thresholds can substitute `rto_strong / 3` and `5 * rto_strong / 3`.)
    #[inline]
    pub const fn active_regime(&self) -> VbfRegime {
        if self.rto_base_ms < 1_000 {
            VbfRegime::Fast3x
        } else if self.rto_base_ms <= 3_000 {
            VbfRegime::Medium2x
        } else {
            VbfRegime::Slow1_3x
        }
    }

    /// Returns the maximum allowed consecutive backoffs based on `rto_base_ms`
    /// so slow LoRa SF11/SF12 links do not stall indefinitely.
    #[inline]
    pub const fn max_backoffs_for_base(&self) -> u8 {
        if self.rto_base_ms < 3_000 {
            4
        } else if self.rto_base_ms < 6_000 {
            3
        } else if self.rto_base_ms < 12_000 {
            2
        } else {
            1
        }
    }

    /// Applies one Variable Backoff Factor step to `rto_current_ms` using pure integer arithmetic.
    /// Includes both the Q8 multiply-shift `(rto * 333) >> 8` (1.30078x) and the
    /// multiplier-free 3-tap shift `rto + (rto >> 2) + (rto >> 5) + (rto >> 6)` (1.296875x).
    #[inline]
    pub fn step_vbf_on_timeout(&mut self, use_pure_shifts_only: bool) -> u32 {
        let max_k = self.max_backoffs_for_base();
        if self.backoff_count < max_k {
            self.backoff_count += 1;
            let rto = self.rto_current_ms;
            let twice = if rto > (u32::MAX >> 1) {
                u32::MAX
            } else {
                rto << 1
            };

            let scaled = match self.active_regime() {
                // 3.0x = rto + (rto << 1)
                VbfRegime::Fast3x => rto.saturating_add(twice),
                // 2.0x = rto << 1
                VbfRegime::Medium2x => twice,
                // 1.3x without floats:
                VbfRegime::Slow1_3x => {
                    if use_pure_shifts_only {
                        // 1 + 1/4 + 1/32 + 1/64 = 83/64 = 1.296875x (zero multiplication instructions)
                        rto.saturating_add(rto >> 2)
                            .saturating_add(rto >> 5)
                            .saturating_add(rto >> 6)
                    } else {
                        // 333 / 256 = 1.30078125x (single 64-bit integer multiply + shift)
                        (((rto as u64) * 333) >> 8) as u32
                    }
                }
            };

            self.rto_current_ms = scaled.clamp(self.rto_min_ms, self.rto_max_ms);
        }
        self.rto_current_ms
    }

    /// Syncs `rto_current_ms` whenever Strong or Weak CoCoA updates `rto_base_ms`.
    #[inline]
    pub fn sync_from_base(&mut self, new_base_ms: u32, reset_backoff: bool) {
        self.rto_base_ms = new_base_ms.clamp(self.rto_min_ms, self.rto_max_ms);
        if reset_backoff || self.backoff_count == 0 {
            self.backoff_count = 0;
            self.rto_current_ms = self.rto_base_ms;
        }
    }
}

/// Dual-Bank CoCoA Integer RTO Estimator with Variable Backoff Factor (VBF), Aging & LFSR Dithering.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CocoaRtoEstimator {
    pub strong: EwmaBank,
    pub weak: EwmaBank,
    pub rto_init_ms: u32,
    pub rto_base_ms: u32,
    pub rto_min_ms: u32,
    pub rto_max_ms: u32,
    pub vbf: IntegerVbfController,
    pub use_pure_shifts_only: bool,
    pub last_update_ms: u32,
    pub sample_weak_from_last_tx: bool,
    pub dither: LfsrDither,
}

impl CocoaRtoEstimator {
    #[inline]
    pub const fn new(toa_ms: u32, guard_ms: u32, rto_max_ms: u32, seed: u32) -> Self {
        let rto_min_ms = (toa_ms << 1).saturating_add(guard_ms);
        let rto_init_ms = rto_min_ms.saturating_add(toa_ms);
        Self {
            strong: EwmaBank {
                srtt_x8: 0,
                rttvar_x4: 0,
                initialized: false,
            },
            weak: EwmaBank {
                srtt_x8: 0,
                rttvar_x4: 0,
                initialized: false,
            },
            rto_init_ms,
            rto_base_ms: rto_init_ms,
            rto_min_ms,
            rto_max_ms,
            vbf: IntegerVbfController::new(rto_init_ms, rto_min_ms, rto_max_ms),
            use_pure_shifts_only: false,
            last_update_ms: 0,
            sample_weak_from_last_tx: true,
            dither: LfsrDither::new(seed),
        }
    }

    /// Strong estimator update (`retries == 0`): `K = 4`, weight `= 0.5` (`>> 1`).
    #[inline]
    pub fn on_strong_sample(&mut self, rtt_ms: u32, now_ms: u32) {
        let e_strong = self.strong.update(rtt_ms, 0); // 0 shift => K = 4
        let delta = (e_strong as i32).saturating_sub(self.rto_base_ms as i32) >> 1;
        let blended = (self.rto_base_ms as i32).saturating_add(delta) as u32;
        self.rto_base_ms = blended.clamp(self.rto_min_ms, self.rto_max_ms);
        self.vbf.sync_from_base(self.rto_base_ms, true);
        self.last_update_ms = now_ms;
    }

    /// Weak estimator update (`1 <= retries <= MAX_WEAK_RETRIES`): `K = 1`, weight `= 0.25` (`>> 2`).
    #[inline]
    pub fn on_weak_sample(&mut self, rtt_ms: u32, now_ms: u32) {
        let e_weak = self.weak.update(rtt_ms, 2); // >> 2 shift => K = 1
        let delta = (e_weak as i32).saturating_sub(self.rto_base_ms as i32) >> 2;
        let blended = (self.rto_base_ms as i32).saturating_add(delta) as u32;
        self.rto_base_ms = blended.clamp(self.rto_min_ms, self.rto_max_ms);
        self.vbf.sync_from_base(self.rto_base_ms, false);
        self.last_update_ms = now_ms;
    }

    /// CoCoA RTO aging: decays stale low (`< 1s` for `> 16*RTO`) or high (`> 3s` for `> 4*RTO`) RTOs toward `rto_init_ms`.
    #[inline]
    pub fn maybe_age_rto(&mut self, now_ms: u32) {
        let idle_ms = now_ms.wrapping_sub(self.last_update_ms);
        let needs_aging = (self.rto_base_ms < 1000 && idle_ms > (self.rto_base_ms << 4))
            || (self.rto_base_ms > 3000 && idle_ms > (self.rto_base_ms << 2));
        if needs_aging {
            let delta = (self.rto_init_ms as i32).saturating_sub(self.rto_base_ms as i32) >> 1;
            self.rto_base_ms = (self.rto_base_ms as i32).saturating_add(delta) as u32;
            self.vbf.sync_from_base(self.rto_base_ms, false);
            self.last_update_ms = now_ms;
        }
    }

    /// Steps CoCoA Variable Backoff Factor (`VBF`) on timeout expiry (`3.0x`, `2.0x`, or `1.3x`).
    #[inline]
    pub fn on_timeout_backoff(&mut self) -> u32 {
        self.vbf.step_vbf_on_timeout(self.use_pure_shifts_only)
    }

    /// Computes VBF-backed-off + LFSR-dithered deadline interval in milliseconds.
    #[inline]
    pub fn next_dithered_rto_ms(&mut self) -> u32 {
        let active_rto = self
            .vbf
            .rto_current_ms
            .clamp(self.rto_min_ms, self.rto_max_ms);
        self.dither.dither_rto(active_rto).min(self.rto_max_ms)
    }
}

/// Slot state for [`LoraCocoaSackTracker`], recording both `first_tx_ms` and `last_tx_ms`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SackSlot {
    pub seq: u16,
    pub first_tx_ms: u32,
    pub last_tx_ms: u32,
    pub deadline_ms: u32,
    pub retries: u8,
    pub in_use: bool,
    pub sacked: bool,
}

/// Sliding-Window LoRa SACK Tracker with Dual-Bank [`CocoaRtoEstimator`] and LFSR Dithering.
#[derive(Clone, Debug)]
pub struct LoraCocoaSackTracker {
    pub base_seq: u16,
    pub slots: [SackSlot; WINDOW_SIZE],
    pub estimator: CocoaRtoEstimator,
}

impl LoraCocoaSackTracker {
    pub const fn new(
        base_seq: u16,
        toa_ms: u32,
        guard_ms: u32,
        rto_max_ms: u32,
        seed: u32,
    ) -> Self {
        Self {
            base_seq,
            slots: [SackSlot {
                seq: 0,
                first_tx_ms: 0,
                last_tx_ms: 0,
                deadline_ms: 0,
                retries: 0,
                in_use: false,
                sacked: false,
            }; WINDOW_SIZE],
            estimator: CocoaRtoEstimator::new(toa_ms, guard_ms, rto_max_ms, seed),
        }
    }

    /// Records an initial transmission of `seq` at `now_ms` with a dithered deadline.
    pub fn on_transmit(&mut self, seq: u16, now_ms: u32) -> bool {
        let offset = seq.wrapping_sub(self.base_seq) as usize;
        if offset >= WINDOW_SIZE {
            return false;
        }
        let idx = (seq as usize) % WINDOW_SIZE;
        let rto_ms = self.estimator.next_dithered_rto_ms();
        self.slots[idx] = SackSlot {
            seq,
            first_tx_ms: now_ms,
            last_tx_ms: now_ms,
            deadline_ms: now_ms.wrapping_add(rto_ms),
            retries: 0,
            in_use: true,
            sacked: false,
        };
        true
    }

    #[inline]
    fn ingest_slot_ack(&mut self, slot: &SackSlot, now_ms: u32) {
        if slot.retries == 0 {
            let rtt = now_ms.wrapping_sub(slot.first_tx_ms);
            self.estimator.on_strong_sample(rtt, now_ms);
        } else if slot.retries <= MAX_WEAK_RETRIES {
            let anchor = if self.estimator.sample_weak_from_last_tx {
                slot.last_tx_ms
            } else {
                slot.first_tx_ms
            };
            let rtt = now_ms.wrapping_sub(anchor);
            self.estimator.on_weak_sample(rtt, now_ms);
        }
    }

    /// Processes cumulative ACK and 32-bit SACK bitmask with Strong/Weak CoCoA routing.
    pub fn on_ack_and_sack(&mut self, cum_ack: u16, sack_bitmap: u32, now_ms: u32) {
        self.estimator.maybe_age_rto(now_ms);

        while self.base_seq != cum_ack {
            let idx = (self.base_seq as usize) % WINDOW_SIZE;
            let slot = self.slots[idx];
            if slot.in_use && slot.seq == self.base_seq {
                if !slot.sacked {
                    self.ingest_slot_ack(&slot, now_ms);
                }
                self.slots[idx].in_use = false;
            }
            self.base_seq = self.base_seq.wrapping_add(1);
        }

        let mut bit = 0usize;
        while bit < 31 {
            if (sack_bitmap & (1u32 << bit)) != 0 {
                let target_seq = cum_ack.wrapping_add(1 + bit as u16);
                let idx = (target_seq as usize) % WINDOW_SIZE;
                let slot = self.slots[idx];
                if slot.in_use && slot.seq == target_seq && !slot.sacked {
                    self.slots[idx].sacked = true;
                    self.ingest_slot_ack(&slot, now_ms);
                }
            }
            bit += 1;
        }
    }

    /// Scans non-SACKed in-flight slots for timeout expiry at `now_ms`, backing off and dithering the next deadline.
    pub fn poll_expired_hole(&mut self, now_ms: u32) -> Option<u16> {
        let mut offset = 0usize;
        while offset < WINDOW_SIZE {
            let seq = self.base_seq.wrapping_add(offset as u16);
            let idx = (seq as usize) % WINDOW_SIZE;
            let slot = &mut self.slots[idx];

            if slot.in_use
                && slot.seq == seq
                && !slot.sacked
                && (now_ms.wrapping_sub(slot.deadline_ms) as i32) >= 0
            {
                self.estimator.on_timeout_backoff();
                let next_rto = self.estimator.next_dithered_rto_ms();
                slot.retries = slot.retries.saturating_add(1);
                slot.last_tx_ms = now_ms;
                slot.deadline_ms = now_ms.wrapping_add(next_rto);
                return Some(seq);
            }
            offset += 1;
        }
        None
    }
}

// ============================================================================
// CoCoA Variable Backoff Factor (VBF: 3x / 2x / 1.3x) & ETSI Duty-Cycle Gate
// ============================================================================

/// 1 hour in milliseconds = maximum scaled credit capacity (`1 credit == 1 ms` of wall time).
pub const HOUR_CREDITS_MAX: u32 = 3_600_000;

/// ETSI EN 300 220-2 / LoRaWAN EU868 sub-band duty-cycle classes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DutyCycleBand {
    /// `10%` duty cycle (`869.4 - 869.65 MHz`): `T_off = 9 * ToA`, `Cost = 10 * ToA`.
    Band10Percent,
    /// `1%` duty cycle (`868.0 - 868.6 MHz`): `T_off = 99 * ToA`, `Cost = 100 * ToA`.
    Band1Percent,
    /// `0.1%` duty cycle (`863.0 - 865.0 MHz`): `T_off = 999 * ToA`, `Cost = 1000 * ToA`.
    Band0_1Percent,
}

impl DutyCycleBand {
    /// Computes quiet-time lockout `T_off = (1/d - 1) * ToA` using pure bit-shifts and additions.
    #[inline]
    pub const fn quiet_time_ms(self, toa_ms: u32) -> u32 {
        match self {
            // 9 * ToA = (ToA << 3) + ToA
            Self::Band10Percent => (toa_ms << 3).saturating_add(toa_ms),
            // 99 * ToA = 64*ToA + 32*ToA + 2*ToA + ToA
            Self::Band1Percent => (toa_ms << 6)
                .saturating_add(toa_ms << 5)
                .saturating_add(toa_ms << 1)
                .saturating_add(toa_ms),
            // 999 * ToA = 1024*ToA - 16*ToA - 8*ToA - ToA
            Self::Band0_1Percent => (toa_ms << 10)
                .saturating_sub(toa_ms << 4)
                .saturating_sub(toa_ms << 3)
                .saturating_sub(toa_ms),
        }
    }

    /// Computes token bucket debit `(1/d) * ToA` using pure bit-shifts and additions.
    #[inline]
    pub const fn credit_cost(self, toa_ms: u32) -> u32 {
        match self {
            // 10 * ToA = (ToA << 3) + (ToA << 1)
            Self::Band10Percent => (toa_ms << 3).saturating_add(toa_ms << 1),
            // 100 * ToA = 64*ToA + 32*ToA + 4*ToA
            Self::Band1Percent => (toa_ms << 6)
                .saturating_add(toa_ms << 5)
                .saturating_add(toa_ms << 2),
            // 1000 * ToA = 1024*ToA - 16*ToA - 8*ToA
            Self::Band0_1Percent => (toa_ms << 10)
                .saturating_sub(toa_ms << 4)
                .saturating_sub(toa_ms << 3),
        }
    }
}

/// Integer-only LoRa sub-band duty-cycle gate (`Token Bucket + T_off Horizon`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LoraDutyCycleGate {
    pub band: DutyCycleBand,
    /// Available scaled credits in `0..=HOUR_CREDITS_MAX` (refills at `+1` per elapsed ms).
    pub credits: u32,
    /// Last timestamp (ms) when the token bucket was refilled.
    pub last_refill_ms: u32,
    /// Earliest timestamp (ms) when the radio is allowed to transmit after `T_off`.
    pub next_tx_allowed_ms: u32,
}

impl LoraDutyCycleGate {
    #[inline]
    pub const fn new(band: DutyCycleBand, now_ms: u32) -> Self {
        Self {
            band,
            credits: HOUR_CREDITS_MAX,
            last_refill_ms: now_ms,
            next_tx_allowed_ms: now_ms,
        }
    }

    /// Computes projected credits at `now_ms` without mutating the bucket (`0..=HOUR_CREDITS_MAX`).
    #[inline]
    pub const fn projected_credits(&self, now_ms: u32) -> u32 {
        let elapsed_ms = now_ms.wrapping_sub(self.last_refill_ms);
        if (elapsed_ms as i32) > 0 {
            let cand = self.credits.saturating_add(elapsed_ms);
            if cand > HOUR_CREDITS_MAX {
                HOUR_CREDITS_MAX
            } else {
                cand
            }
        } else {
            self.credits
        }
    }

    /// Refills 1 credit per elapsed millisecond (capped at `HOUR_CREDITS_MAX`).
    #[inline]
    pub fn refill(&mut self, now_ms: u32) {
        let elapsed_ms = now_ms.wrapping_sub(self.last_refill_ms);
        if (elapsed_ms as i32) > 0 {
            self.credits =
                core::cmp::min(HOUR_CREDITS_MAX, self.credits.saturating_add(elapsed_ms));
            self.last_refill_ms = now_ms;
        }
    }

    /// Read-only check (`P6 Check-Then-Commit Split`): returns `Ok(())` if transmission of `toa_ms`
    /// is permitted at `now_ms`, or `Err(wait_ms)` with the exact millisecond delay until both
    /// `T_off` and rolling-hour credits clear.
    #[inline]
    pub const fn check_ready(&self, now_ms: u32, toa_ms: u32) -> Result<(), u32> {
        let effective_credits = self.projected_credits(now_ms);

        // 1. Check per-packet T_off quiet horizon
        let quiet_rem = self.next_tx_allowed_ms.wrapping_sub(now_ms) as i32;
        let wait_quiet_ms = if quiet_rem > 0 { quiet_rem as u32 } else { 0 };

        // 2. Check rolling-hour token bucket balance
        let cost = self.band.credit_cost(toa_ms);
        let wait_credit_ms = cost.saturating_sub(effective_credits);

        let total_wait_ms = if wait_quiet_ms > wait_credit_ms {
            wait_quiet_ms
        } else {
            wait_credit_ms
        };

        if total_wait_ms == 0 {
            Ok(())
        } else {
            Err(total_wait_ms)
        }
    }

    /// Commits a transmission of `toa_ms` at `now_ms`, refilling and deducting scaled credits
    /// and advancing `next_tx_allowed_ms` by `toa_ms + T_off`.
    #[inline]
    pub fn consume_airtime(&mut self, now_ms: u32, toa_ms: u32) -> Result<(), u32> {
        self.check_ready(now_ms, toa_ms)?;
        self.refill(now_ms);
        let cost = self.band.credit_cost(toa_ms);
        let t_off = self.band.quiet_time_ms(toa_ms);

        self.credits = self.credits.saturating_sub(cost);
        self.next_tx_allowed_ms = now_ms.wrapping_add(toa_ms.saturating_add(t_off));
        Ok(())
    }
}

/// Outcome of polling the SACK tracker with duty-cycle gating (`P3 Gate-Before-Mutate` & `P4 Deferral`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PollRetransmitOutcome {
    /// No SACK holes have expired yet.
    NoExpiredHoles,
    /// A SACK hole expired, but the sub-band duty cycle is locked for `wait_ms`.
    DeferredByDutyCycle { seq: u16, wait_ms: u32 },
    /// Airtime granted, VBF stepped, and segment `seq` is ready for immediate TX.
    TransmitReady { seq: u16, next_rto_ms: u32 },
}

/// Gated SACK hole expiry poll: defers VBF backoff if the LoRa sub-band is in `T_off` lockout.
#[inline]
pub fn poll_expired_hole_gated(
    slots: &mut [SackSlot; WINDOW_SIZE],
    base_seq: u16,
    vbf: &mut IntegerVbfController,
    dither: &mut LfsrDither,
    duty_gate: &mut LoraDutyCycleGate,
    toa_ms: u32,
    now_ms: u32,
) -> PollRetransmitOutcome {
    let mut offset = 0usize;
    while offset < WINDOW_SIZE {
        let seq = base_seq.wrapping_add(offset as u16);
        let idx = (seq as usize) % WINDOW_SIZE;
        let slot = &mut slots[idx];

        // Rollover-safe timestamp check: (now.wrapping_sub(deadline) as i32) >= 0
        if slot.in_use
            && slot.seq == seq
            && !slot.sacked
            && (now_ms.wrapping_sub(slot.deadline_ms) as i32) >= 0
        {
            // Gate-before-mutate: check & consume sub-band airtime BEFORE stepping VBF or incrementing retries
            match duty_gate.consume_airtime(now_ms, toa_ms) {
                Ok(()) => {
                    let vbf_rto = vbf.step_vbf_on_timeout(false);
                    let next_rto_ms = dither.dither_rto(vbf_rto).min(vbf.rto_max_ms);
                    slot.retries = slot.retries.saturating_add(1);
                    slot.last_tx_ms = now_ms;
                    slot.deadline_ms = now_ms.wrapping_add(next_rto_ms);
                    return PollRetransmitOutcome::TransmitReady { seq, next_rto_ms };
                }
                Err(wait_ms) => {
                    // Push deadline to the end of the duty-cycle lockout without burning a retry
                    slot.deadline_ms = now_ms.wrapping_add(wait_ms);
                    return PollRetransmitOutcome::DeferredByDutyCycle { seq, wait_ms };
                }
            }
        }
        offset += 1;
    }
    PollRetransmitOutcome::NoExpiredHoles
}

// ============================================================================
// Borealiscodes +++ Constellation Bridge (Unify · Breathe · Learn · Evolve)
// ============================================================================

/// External [`Borealiscodes`](https://github.com/Borealiscodes) public contribution anchor (#3 Ireland committer).
pub const BOREALISCODES_PUBLIC_CONTRIBUTIONS: u32 = 14_562;
/// Ireland GitHub committers leaderboard rank for `Borealiscodes`.
pub const BOREALISCODES_IRELAND_RANK: u8 = 3;
/// RFC 6675 SACK fast-retransmit threshold (3 SACKed segments ahead of hole).
pub const SACK_FAST_RETX_THRESHOLD: u8 = 3;

/// Represents the four-phase lifecycle of a unified Constellation node:
/// - `Unify` (Dr. Kenzō Tenma's equal-value axiom / `Zhen Yi` True Intent)
/// - `Breathe` (ETSI duty-cycle respiration / `Taixi` & tethering the Will-Horse `Yima`)
/// - `Learn` (CoCoA Strong/Weak dual-bank & Lunge × Grimmer synthesis / taming `Xinyuan`)
/// - `Evolve` (VBF `3x/2x/1.3x` + LFSR dithering / Sun Wukong `Douzhanshengfo`)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstellationPhase {
    /// Phase 1: State alignment, identity verification, and SACK window sync.
    Unify = 0,
    /// Phase 2: ETSI EN 300 220-2 quiet-time respiration and duty-cycle recharge.
    Breathe = 1,
    /// Phase 3: CoCoA Strong/Weak dual-bank RTT ingestion and SACK hole detection.
    Learn = 2,
    /// Phase 4: VBF (`3.0x` / `2.0x` / `1.3x`) scaling and LFSR+Xorshift32 swarm dithering.
    Evolve = 3,
}

impl ConstellationPhase {
    #[inline]
    pub const fn next(self) -> Self {
        match self {
            Self::Unify => Self::Breathe,
            Self::Breathe => Self::Learn,
            Self::Learn => Self::Evolve,
            Self::Evolve => Self::Unify,
        }
    }
}

/// Operating mode governed by the `+++` guard-time escape detector.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransceiverMode {
    /// Transparent LoRa packet framing and SACK transport mode.
    DataMode = 0,
    /// Constellation introspection, reconfiguration, and diff-merge mode (`+++`).
    CommandMergeMode = 1,
}

/// Deterministic `#![no_std]` detector for the classic `+++` escape sequence
/// requiring `guard_ms` (default `1,000 ms`) of silence before and after three `+` bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlusEscapeDetector {
    pub mode: TransceiverMode,
    pub plus_count: u8,
    pub last_rx_ms: u32,
    pub sequence_completed_ms: u32,
    pub guard_ms: u32,
}

impl PlusEscapeDetector {
    pub const DEFAULT_GUARD_MS: u32 = 1_000;

    #[inline]
    pub const fn new(guard_ms: u32) -> Self {
        Self {
            mode: TransceiverMode::DataMode,
            plus_count: 0,
            last_rx_ms: 0,
            sequence_completed_ms: 0,
            guard_ms,
        }
    }

    /// Feeds an incoming byte at `now_ms` and tracks the `+++` guard-time window.
    #[inline]
    pub fn on_byte(&mut self, byte: u8, now_ms: u32) {
        let idle_before = now_ms.wrapping_sub(self.last_rx_ms);
        if byte == b'+' {
            if self.plus_count == 0 {
                if idle_before >= self.guard_ms {
                    self.plus_count = 1;
                }
            } else if self.plus_count < 3 && idle_before < self.guard_ms {
                self.plus_count += 1;
                if self.plus_count == 3 {
                    self.sequence_completed_ms = now_ms;
                }
            } else {
                self.plus_count = 0;
            }
        } else {
            self.plus_count = 0;
        }
        self.last_rx_ms = now_ms;
    }

    /// Evaluates whether the trailing guard silence has elapsed after `+++` to enter `CommandMergeMode`.
    #[inline]
    pub fn poll_tick(&mut self, now_ms: u32) -> TransceiverMode {
        if self.plus_count == 3 {
            let idle_after = now_ms.wrapping_sub(self.sequence_completed_ms);
            if idle_after >= self.guard_ms {
                self.mode = TransceiverMode::CommandMergeMode;
                self.plus_count = 0;
            }
        }
        self.mode
    }

    /// Returns the transceiver from `CommandMergeMode` (`+++`) back to transparent `DataMode` (`ATO`).
    #[inline]
    pub fn exit_to_data_mode(&mut self, now_ms: u32) {
        self.mode = TransceiverMode::DataMode;
        self.plus_count = 0;
        self.last_rx_ms = now_ms;
    }
}

/// RFC 1982 serial-number wrap-around comparison `(a - b) as i32 < 0`.
#[inline]
pub const fn rfc1982_seq_lt_u32(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) < 0
}

/// Deterministic integer Semtech SX126x/SX127x Time-on-Air (`ToA`) in milliseconds
/// across `SF7..=SF12` (`bw_khz` in `{125, 250, 500}`), automatically enabling Low Data Rate
/// Optimization (`LDRO`) when $T_{\text{sym}} \ge 16\text{ ms}$ (`SF11`/`SF12` at $125\text{ kHz}$).
#[inline]
pub const fn semtech_sx126x_toa_ms(
    sf: u8,
    bw_khz: u16,
    payload_bytes: u8,
    coding_rate_denom: u8,
    explicit_header: bool,
    crc_on: bool,
) -> u32 {
    let sf_clamped = if sf < 7 {
        7u32
    } else if sf > 12 {
        12u32
    } else {
        sf as u32
    };
    let bw = if bw_khz == 0 { 125u32 } else { bw_khz as u32 };
    let cr = if coding_rate_denom < 5 {
        1u32
    } else if coding_rate_denom > 8 {
        4u32
    } else {
        (coding_rate_denom - 4) as u32
    };

    // Symbol duration in microseconds: T_sym_us = (2^SF * 1000) / bw_khz
    let t_sym_us = ((1u32 << sf_clamped) * 1_000) / bw;
    let de = if t_sym_us >= 16_000 { 1u32 } else { 0u32 };
    let ih = if explicit_header { 0u32 } else { 1u32 };
    let crc = if crc_on { 1u32 } else { 0u32 };

    // Numerator inside ceil: max(0, 8*PL - 4*SF + 28 + 16*CRC - 20*IH)
    let lhs = 8u32
        .saturating_mul(payload_bytes as u32)
        .saturating_add(28)
        .saturating_add(16 * crc);
    let rhs = (4 * sf_clamped).saturating_add(20 * ih);
    let num = lhs.saturating_sub(rhs);
    let denom = 4 * (sf_clamped.saturating_sub(2 * de));
    let ceil_blocks = if num == 0 { 0u32 } else { num.div_ceil(denom) };
    let payload_symbols = 8u32.saturating_add(ceil_blocks.saturating_mul(cr + 4));

    // Total time in microseconds = (T_preamble + T_payload) = ((8 + 4.25) + payload_symbols) * T_sym_us
    // = (49 + 4 * payload_symbols) * T_sym_us / 4
    let total_quarter_syms = 49u32.saturating_add(4 * payload_symbols);
    let total_us = (total_quarter_syms as u64) * (t_sym_us as u64) / 4;
    total_us.div_ceil(1_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_rto_estimator_sub_8ms_precision_and_toa_floor() {
        // LoRa SF9 link: ToA = 200ms, guard = 50ms => rto_min_ms = 450ms, initial_rto = 650ms
        let mut est = IntegerRtoEstimator::new(200, 50, 60_000);
        assert_eq!(est.rto_min_ms, 450);
        assert_eq!(est.rto_base_ms, 650);
        assert_eq!(est.current_rto_ms(), 650);

        // First sample R = 500ms -> srtt_x8 = 4000, rttvar_x4 = 1000, raw_rto = 1500ms
        est.on_valid_rtt_sample(500);
        assert!(est.initialized);
        assert_eq!(est.srtt_x8, 4000);
        assert_eq!(est.rttvar_x4, 1000);
        assert_eq!(est.rto_base_ms, 1500);

        // Sub-8ms delta: R = 503ms (err = +3ms). Unscaled integer math would truncate 3 >> 3 = 0,
        // whereas scaled srtt_x8 advances cleanly to 4003!
        est.on_valid_rtt_sample(503);
        assert_eq!(est.srtt_x8, 4003);
        assert_eq!(est.rttvar_x4, 753); // 1000 - 250 + 3 = 753
    }

    #[test]
    fn test_lora_sack_tracker_karns_rule_and_exponential_backoff() {
        let mut tracker = LoraSackTracker::new(100, 120, 60, 30_000);
        assert_eq!(tracker.rto.rto_min_ms, 300); // 2 * 120 + 60
        assert_eq!(tracker.rto.current_rto_ms(), 420); // 300 + 120

        // Transmit segments 100, 101, 102 at t = 1000ms
        assert!(tracker.on_transmit(100, 1000));
        assert!(tracker.on_transmit(101, 1000));
        assert!(tracker.on_transmit(102, 1000));
        assert_eq!(tracker.in_flight_count(), 3);

        // Out-of-order SACK arrives for seq 101 & 102 (cum_ack = 100, sack_bitmap = 0b11) at t = 1360ms (RTT = 360ms)
        // Because retries == 0 on 101 & 102, Karn's rule allows sampling RTT from SACK!
        tracker.on_ack_and_sack(100, 0b11, 1360);
        assert!(tracker.rto.initialized);
        assert_eq!(tracker.sacked_count(), 2);
        let rto_after_sack = tracker.rto.current_rto_ms();
        assert!(rto_after_sack >= 300);

        // Segment 100 (the hole) expires at t = 1500ms (deadline was 1420ms)
        let expired = tracker.poll_expired_hole(1500);
        assert_eq!(expired, Some(100));
        assert_eq!(tracker.rto.backoff_shift, 1);
        assert_eq!(tracker.rto.current_rto_ms(), rto_after_sack * 2);

        // Cumulative ACK arrives for 103 at t = 1900ms, closing retransmitted 100 (retries == 1)
        // and already-SACKed 101, 102. Karn's rule MUST NOT sample the retransmitted hole 100!
        let srtt_before = tracker.rto.srtt_x8;
        tracker.on_ack_and_sack(103, 0, 1900);
        assert_eq!(tracker.rto.srtt_x8, srtt_before);
        assert_eq!(tracker.base_seq, 103);
        assert_eq!(tracker.in_flight_count(), 0);

        // Transmit segment 103 at t = 2000ms and receive fresh ACK at t = 2350ms -> collapses backoff_shift to 0
        assert!(tracker.on_transmit(103, 2000));
        tracker.on_ack_and_sack(104, 0, 2350);
        assert_eq!(tracker.rto.backoff_shift, 0);
    }

    #[test]
    fn test_rfc6298_section_5_5_estimator_reseed_after_heavy_backoff() {
        let mut est = IntegerRtoEstimator::new(100, 50, 60_000);
        est.on_valid_rtt_sample(300);
        assert!(est.initialized);
        for _ in 0..4 {
            est.on_timeout_backoff();
        }
        assert_eq!(est.backoff_shift, 4);
        assert!(
            !est.initialized,
            "RFC 6298 5.5 must clear initialized at backoff_shift >= 4"
        );
        est.on_valid_rtt_sample(400);
        assert!(est.initialized);
        assert_eq!(est.backoff_shift, 0);
        assert_eq!(est.srtt_x8, 400 << 3);
    }

    #[test]
    fn test_cocoa_dual_bank_strong_weak_blending_aging_and_lfsr_dither() {
        let mut dither = LfsrDither::new(0);
        assert_eq!(dither.state, 0xACE1);
        for _ in 0..64 {
            let d = dither.dither_rto(1000);
            assert!(
                (1000..=1498).contains(&d),
                "LFSR dither {d} must lie in [1.0, 1.498] * 1000ms"
            );
        }

        let mut cocoa = LoraCocoaSackTracker::new(500, 120, 60, 30_000, 0x8B15_CAFE);
        assert_eq!(cocoa.estimator.rto_min_ms, 300);
        assert_eq!(cocoa.estimator.rto_init_ms, 420);

        // Transmit 500 and 501 at t = 1000ms
        assert!(cocoa.on_transmit(500, 1000));
        assert!(cocoa.on_transmit(501, 1000));

        // SACK 501 at t = 1360ms (retries == 0 -> Strong bank K=4, weight 1/2)
        cocoa.on_ack_and_sack(500, 0b1, 1360);
        assert!(cocoa.estimator.strong.initialized);
        assert!(!cocoa.estimator.weak.initialized);
        // Strong E = 360 + 2*360 = 1080ms; blended with 420ms -> 420 + (1080 - 420)/2 = 750ms
        assert_eq!(cocoa.estimator.rto_base_ms, 750);

        // Expire hole 500 at t = 2000ms -> retries becomes 1, last_tx_ms = 2000ms
        let hole = cocoa.poll_expired_hole(2000);
        assert_eq!(hole, Some(500));
        assert_eq!(cocoa.slots[(500usize) % WINDOW_SIZE].retries, 1);

        // Cumulative ACK 502 arrives at t = 2400ms -> 500 has retries == 1, so it feeds Weak bank (K=1, weight 1/4)!
        // Weak RTT from last_tx_ms = 2400 - 2000 = 400ms -> E_weak = 400 + (800 >> 2) = 600ms
        // Blended rto_base = 750 + ((600 - 750) >> 2) = 750 - 38 = 712ms
        cocoa.on_ack_and_sack(502, 0, 2400);
        assert!(cocoa.estimator.weak.initialized);
        assert_eq!(cocoa.estimator.weak.srtt_x8, 400 << 3);
        assert_eq!(cocoa.estimator.rto_base_ms, 712);

        // Idle aging check: rto_base_ms = 712 (< 1000ms), after > 16 * 712 = 11392ms idle (at t = 15000ms),
        // rto_base_ms decays halfway toward rto_init_ms (420ms): 712 + ((420 - 712) >> 1) = 566ms
        cocoa.estimator.maybe_age_rto(15_000);
        assert_eq!(cocoa.estimator.rto_base_ms, 566);
    }

    #[test]
    fn test_vbf_three_regimes_and_duty_cycle_gated_poll() {
        let mut fast = IntegerVbfController::new(400, 200, 60_000);
        assert_eq!(fast.active_regime(), VbfRegime::Fast3x);
        assert_eq!(fast.step_vbf_on_timeout(false), 1_200);
        assert_eq!(fast.step_vbf_on_timeout(false), 3_600);

        let mut med = IntegerVbfController::new(1_500, 200, 60_000);
        assert_eq!(med.active_regime(), VbfRegime::Medium2x);
        assert_eq!(med.step_vbf_on_timeout(false), 3_000);
        assert_eq!(med.step_vbf_on_timeout(false), 6_000);

        let mut slow_q8 = IntegerVbfController::new(4_000, 200, 60_000);
        assert_eq!(slow_q8.active_regime(), VbfRegime::Slow1_3x);
        assert_eq!(slow_q8.step_vbf_on_timeout(false), 5_203);

        let mut slow_shift = IntegerVbfController::new(4_000, 200, 60_000);
        assert_eq!(slow_shift.step_vbf_on_timeout(true), 5_187);

        // 1% sub-band with a 500 ms SF10 packet -> T_off = 49,500 ms, Cost = 50,000 credits
        let mut gate_1pct = LoraDutyCycleGate::new(DutyCycleBand::Band1Percent, 0);
        assert_eq!(DutyCycleBand::Band1Percent.quiet_time_ms(500), 49_500);
        assert_eq!(DutyCycleBand::Band1Percent.credit_cost(500), 50_000);
        assert_eq!(gate_1pct.consume_airtime(0, 500), Ok(()));
        assert_eq!(gate_1pct.credits, HOUR_CREDITS_MAX - 50_000);
        assert_eq!(gate_1pct.next_tx_allowed_ms, 50_000);
        assert_eq!(gate_1pct.check_ready(10_000, 500), Err(40_000));
        // Read-only check_ready (P6) does not mutate bucket state
        assert_eq!(gate_1pct.credits, HOUR_CREDITS_MAX - 50_000);
        assert_eq!(gate_1pct.check_ready(49_999, 500), Err(1));
        assert_eq!(gate_1pct.check_ready(50_000, 500), Ok(()));
        gate_1pct.refill(HOUR_CREDITS_MAX);
        assert_eq!(gate_1pct.credits, HOUR_CREDITS_MAX);

        // 10% sub-band (869.4 - 869.65 MHz): T_off = 9 * ToA = 4,500 ms, Cost = 10 * ToA = 5,000 credits
        let mut gate_10pct = LoraDutyCycleGate::new(DutyCycleBand::Band10Percent, 0);
        assert_eq!(DutyCycleBand::Band10Percent.quiet_time_ms(500), 4_500);
        assert_eq!(DutyCycleBand::Band10Percent.credit_cost(500), 5_000);
        assert_eq!(gate_10pct.consume_airtime(0, 500), Ok(()));
        assert_eq!(gate_10pct.check_ready(4_999, 500), Err(1));
        assert_eq!(gate_10pct.check_ready(5_000, 500), Ok(()));

        // Gated SACK poll test: verify DeferredByDutyCycle preserves retries == 0 and backoff_count == 0
        let mut slots = [SackSlot::default(); WINDOW_SIZE];
        slots[0] = SackSlot {
            seq: 0,
            first_tx_ms: 0,
            last_tx_ms: 0,
            deadline_ms: 1_200,
            retries: 0,
            in_use: true,
            sacked: false,
        };
        let mut vbf = IntegerVbfController::new(1_200, 300, 60_000);
        let mut dither = LfsrDither::new(0xACE1);
        let mut gate = LoraDutyCycleGate::new(DutyCycleBand::Band1Percent, 0);
        assert_eq!(gate.consume_airtime(0, 500), Ok(())); // Locks band until t = 50,000 ms

        let out_deferred =
            poll_expired_hole_gated(&mut slots, 0, &mut vbf, &mut dither, &mut gate, 500, 1_200);
        assert_eq!(
            out_deferred,
            PollRetransmitOutcome::DeferredByDutyCycle {
                seq: 0,
                wait_ms: 48_800
            }
        );
        assert_eq!(slots[0].retries, 0);
        assert_eq!(vbf.backoff_count, 0);

        // Once t = 50,000 ms arrives, gate grants airtime and VBF steps cleanly!
        let out_ready =
            poll_expired_hole_gated(&mut slots, 0, &mut vbf, &mut dither, &mut gate, 500, 50_000);
        assert!(matches!(
            out_ready,
            PollRetransmitOutcome::TransmitReady { seq: 0, .. }
        ));
        assert_eq!(slots[0].retries, 1);
        assert_eq!(vbf.backoff_count, 1);
    }

    #[test]
    fn test_borealiscodes_plus_escape_detector_and_sx126x_toa() {
        let mut det = PlusEscapeDetector::new(PlusEscapeDetector::DEFAULT_GUARD_MS);
        assert_eq!(det.mode, TransceiverMode::DataMode);

        // Normal data byte at t = 500 ms -> DataMode
        det.on_byte(b'A', 500);
        // Too-early '+' at t = 1,200 ms (< 1000 ms guard silence after 500 ms) -> ignored
        det.on_byte(b'+', 1_200);
        assert_eq!(det.plus_count, 0);

        // Guard silence of 1,000 ms elapsed at t = 2,200 ms -> first '+' accepted
        det.on_byte(b'+', 2_200);
        assert_eq!(det.plus_count, 1);
        det.on_byte(b'+', 2_300);
        assert_eq!(det.plus_count, 2);
        det.on_byte(b'+', 2_400);
        assert_eq!(det.plus_count, 3);

        // Before trailing guard silence elapses (t = 3,000 ms < 2,400 + 1,000), still DataMode
        assert_eq!(det.poll_tick(3_000), TransceiverMode::DataMode);
        // After 1,000 ms trailing guard silence (t = 3,400 ms), transitions to CommandMergeMode!
        assert_eq!(det.poll_tick(3_400), TransceiverMode::CommandMergeMode);

        // Cycle through all 4 Constellation phases: Unify -> Breathe -> Learn -> Evolve -> Unify
        let p0 = ConstellationPhase::Unify;
        let p1 = p0.next();
        let p2 = p1.next();
        let p3 = p2.next();
        assert_eq!(p1, ConstellationPhase::Breathe);
        assert_eq!(p2, ConstellationPhase::Learn);
        assert_eq!(p3, ConstellationPhase::Evolve);
        assert_eq!(p3.next(), ConstellationPhase::Unify);

        // Semtech SX126x ToA check: SF7 vs SF12 at 125 kHz (LDRO auto-enabled on SF12)
        let toa_sf7 = semtech_sx126x_toa_ms(7, 125, 64, 5, true, true);
        let toa_sf12 = semtech_sx126x_toa_ms(12, 125, 64, 5, true, true);
        assert!((100..=150).contains(&toa_sf7));
        assert!(toa_sf12 > 2_000);
        assert!(rfc1982_seq_lt_u32(u32::MAX - 5, 10));
    }
}

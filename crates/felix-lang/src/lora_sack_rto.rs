//! LoRa Selective Acknowledgment (RFC 2018 / RFC 6675 SACK) Sliding-Window Tracker
//! with Fixed-Point Integer RFC 6298 (Van Jacobson / Karels) RTO Estimator & Binary Exponential Backoff.
//!
//! ### Fixed-Point Register Math (Zero Floats, Zero Truncation Stalls)
//! - `srtt_x8 = SRTT << 3` ($\alpha = 1/8 = 2^{-3}$)
//! - `rttvar_x4 = RTTVAR << 2` ($\beta = 1/4 = 2^{-2}$, stores $4 \cdot \text{RTTVAR}$ directly)
//! - `rto_min_ms = (toa_ms << 1) + guard_ms` (LoRa Time-on-Air floor for SF7–SF12)
//! - `rto_ms = clamp(rto_base_ms << backoff_shift, rto_min_ms, rto_max_ms)`

use core::cmp::max;

/// Sliding-window slot capacity for LoRa SACK tracking (32 segments).
pub const WINDOW_SIZE: usize = 32;
/// Minimum clock granularity $G$ in milliseconds (RFC 6298 Section 2.3).
pub const CLOCK_GRANULARITY_MS: u32 = 10;
/// Maximum binary exponential backoff shift exponent ($2^6 = 64\times$ multiplier).
pub const MAX_BACKOFF_SHIFT: u8 = 6;

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

            if slot.in_use && slot.seq == seq && !slot.sacked {
                // Signed wrapping comparison handles 32-bit millisecond timer rollover
                if (now_ms.wrapping_sub(slot.deadline_ms) as i32) >= 0 {
                    self.rto.on_timeout_backoff();
                    let next_rto = self.rto.current_rto_ms();
                    slot.retries = slot.retries.saturating_add(1);
                    slot.tx_time_ms = now_ms;
                    slot.deadline_ms = now_ms.wrapping_add(next_rto);
                    return Some(seq);
                }
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
}

//! `felix/crates/felix-lang/src/m8_axum_webhook_server.rs`
//!
//! Self-Hosted Triadic Payment Webhook & Capability-Signed `.m8` Download Server (`M8CommerceWebhookServer`).
//! Implements self-contained FIPS 180-4 SHA-256 & constant-time RFC 2104 HMAC-SHA256 verification for
//! Stripe (Pole `+1`), Revolut Merchant (Pole `0`), and PayPal (Pole `-1`), minting Read-Only (`trit_perm = 0`)
//! `SignedM8DownloadTicket` tokens with Charm XVIII (`CAP_ROOT_SEAL` stripped) and one-time epoch revocation.

use crate::{
    agp_cap_iommu_tlb::VirtualCapHandle64,
    felix_phoenix::{CapEffectMask, CAP_ARENA, CAP_M8},
    ljodatal_18_charms::{verify_charm_18_sovereign_secrecy, CAP_ROOT_SEAL},
    m8_genesis_container::{fnv1a_64_const, M8PersonaContainer, OMNI_GENESIS_SEED_PHRASE},
    phoenix_cap_abi::{
        CapAbiFault, PhoenixArenaBackend, PhoenixCapOp, PhoenixCapabilityAbiTable, CAP_BLOCK_BYTES,
    },
    well_signal_resolution::WellRecord,
    well_signal_spqt_espnow::{pack_signal_table_spqt3, WELL_SIGNAL_SPQT_BYTES},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum allowed clock skew / replay window for incoming payment webhooks (`300` seconds = 5 minutes).
pub const WEBHOOK_MAX_REPLAY_SKEW_SECS: u64 = 300;
/// Default validity window for an issued `.m8` capability download ticket (`3600` seconds = 1 hour).
pub const DOWNLOAD_TICKET_TTL_SECS: u64 = 3_600;

// ============================================================================
// 1. Self-Contained FIPS 180-4 SHA-256 & RFC 2104 Constant-Time HMAC-SHA256
// ============================================================================

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Computes the exact FIPS 180-4 SHA-256 digest of `data`.
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((data.len() + 9).div_ceil(64) * 64);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Computes RFC 2104 `HMAC-SHA256(key, message)`.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = sha256_digest(key);
        key_block[0..32].copy_from_slice(&hashed);
    } else {
        key_block[0..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5Cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner_input = Vec::with_capacity(64 + message.len());
    inner_input.extend_from_slice(&ipad);
    inner_input.extend_from_slice(message);
    let inner_hash = sha256_digest(&inner_input);

    let mut outer_input = [0u8; 96];
    outer_input[0..64].copy_from_slice(&opad);
    outer_input[64..96].copy_from_slice(&inner_hash);
    sha256_digest(&outer_input)
}

/// Branchless constant-time equality check over two 32-byte HMAC-SHA256 digests
/// (Charm X: *Túnriður Villa* constant-time information-flow invariant).
#[inline]
pub fn constant_time_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

pub fn hex_encode_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

pub fn hex_decode_32(hex_str: &str) -> Option<[u8; 32]> {
    let trimmed = hex_str.trim();
    if trimmed.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    let bytes = trimmed.as_bytes();
    for i in 0..32 {
        let hi = decode_hex_nibble(bytes[i * 2])?;
        let lo = decode_hex_nibble(bytes[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

#[inline]
const fn decode_hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ============================================================================
// 2. Triadic Payment Provider Pole & Webhook Event Payloads
// ============================================================================

/// Maps the three checkout providers onto the balanced triad `{-1, 0, +1}` ($\sum = 0$).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(i8)]
pub enum PaymentGatewayPole {
    /// Pole `+1`: Stripe Checkout (`checkout.session.completed`)
    StripePlus = 1,
    /// Pole `0`: Revolut Merchant API (`ORDER_COMPLETED`)
    RevolutZero = 0,
    /// Pole `-1`: PayPal Commerce (`PAYMENT.CAPTURE.COMPLETED`)
    PayPalMinus = -1,
}

impl PaymentGatewayPole {
    /// Maps each payment pole into its dedicated 9-state Futhark *ætt* (`0..=8`, `9..=17`, `18..=26`).
    #[inline]
    pub const fn futhark_aett_base(self) -> u8 {
        match self {
            Self::StripePlus => 0,   // Freyr's ætt (0..=8)
            Self::RevolutZero => 9,  // Hagal's ætt (9..=17)
            Self::PayPalMinus => 18, // Týr's ætt (18..=26)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutWebhookPayload {
    pub event_id: String,
    pub event_type: String,
    pub order_reference: String,
    pub persona_seed_sku: String,
    pub amount_minor_units: u64,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookVerificationError {
    MissingOrMalformedSignatureHeader,
    InvalidHexSignature,
    HmacSignatureMismatch,
    ReplayTimestampOutOfBounds {
        webhook_ts: u64,
        server_now: u64,
        max_skew: u64,
    },
    UnsupportedEventType(String),
    ZeroPaymentAmount,
    MalformedJsonPayload(String),
    Charm18RootSealViolation,
    CapabilityTableExhausted,
    TicketInvalidOrExpired,
    CapabilityAccessFault(CapAbiFault),
}

// ============================================================================
// 3. Capability-Signed `.m8` Download Ticket (`SignedM8DownloadTicket`)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedM8DownloadTicket {
    pub gateway_pole: PaymentGatewayPole,
    pub event_id: String,
    pub order_reference: String,
    /// Opaque 64-bit PHOENIX capability handle pointing to the Read-Only (`trit_perm = 0`) `.m8` slice.
    pub virt_cap_handle: VirtualCapHandle64,
    /// Exported capability mask (`CAP_ARENA | CAP_M8`), guaranteed free of `CAP_ROOT_SEAL`.
    pub granted_download_caps: CapEffectMask,
    /// 64-bit FNV-1a Proof-Carrying Code seal of the underlying `M8PersonaContainer`.
    pub container_pcc_seal_u64: u64,
    /// 3 SPQT Base-243 bytes encoding the 4-register `SignalResolutionTable` (`handle, name, container, address`).
    pub well_signal_spqt3: [u8; WELL_SIGNAL_SPQT_BYTES],
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    /// 32-byte enclave HMAC-SHA256 signature binding the capability handle, PCC seal, and expiry.
    pub enclave_hmac_sig_hex: String,
}

impl SignedM8DownloadTicket {
    #[allow(clippy::too_many_arguments)]
    pub fn canonical_signing_bytes(
        gateway_pole: PaymentGatewayPole,
        event_id: &str,
        order_reference: &str,
        virt_cap_handle: VirtualCapHandle64,
        granted_download_caps: CapEffectMask,
        container_pcc_seal_u64: u64,
        well_signal_spqt3: &[u8; WELL_SIGNAL_SPQT_BYTES],
        issued_at_unix: u64,
        expires_at_unix: u64,
    ) -> Vec<u8> {
        format!(
            "M8TICKET:v1:pole={}:ev={}:ord={}:cap={:#018x}:mask={:#010x}:pcc={:#018x}:spqt={:02x}{:02x}{:02x}:iat={}:exp={}",
            gateway_pole as i8,
            event_id,
            order_reference,
            virt_cap_handle.0,
            granted_download_caps,
            container_pcc_seal_u64,
            well_signal_spqt3[0],
            well_signal_spqt3[1],
            well_signal_spqt3[2],
            issued_at_unix,
            expires_at_unix
        )
        .into_bytes()
    }
}

// ============================================================================
// 4. Self-Hosted Commerce Enclave & PHOENIX Capability Provisioner
// ============================================================================

pub struct M8WebhookSecrets {
    pub stripe_webhook_secret: Vec<u8>,
    pub revolut_webhook_secret: Vec<u8>,
    pub paypal_webhook_secret: Vec<u8>,
    pub paypal_webhook_id: String,
    /// Internal Charm XVIII (`CAP_ROOT_SEAL`) master signing secret.
    enclave_root_seal_secret: [u8; 32],
}

impl M8WebhookSecrets {
    pub fn new(
        stripe_secret: &str,
        revolut_secret: &str,
        paypal_secret: &str,
        paypal_webhook_id: &str,
        enclave_master_phrase: &str,
    ) -> Self {
        Self {
            stripe_webhook_secret: stripe_secret.as_bytes().to_vec(),
            revolut_webhook_secret: revolut_secret.as_bytes().to_vec(),
            paypal_webhook_secret: paypal_secret.as_bytes().to_vec(),
            paypal_webhook_id: paypal_webhook_id.to_string(),
            enclave_root_seal_secret: sha256_digest(enclave_master_phrase.as_bytes()),
        }
    }
}

pub struct M8CommerceWebhookServer<B: PhoenixArenaBackend> {
    pub secrets: M8WebhookSecrets,
    pub cap_table: PhoenixCapabilityAbiTable<B>,
    /// Idempotent ledger of processed webhook `event_id -> SignedM8DownloadTicket`.
    pub idempotent_ledger: BTreeMap<String, SignedM8DownloadTicket>,
    /// Stores the serialized `.m8` container length (in bytes) per capability slot.
    pub slot_m8_byte_len: BTreeMap<u8, usize>,
    pub next_slot_cursor: u8,
    pub next_arena_block_cursor: u32,
}

impl<B: PhoenixArenaBackend> M8CommerceWebhookServer<B> {
    pub fn new(secrets: M8WebhookSecrets, backend: B) -> Self {
        Self {
            secrets,
            cap_table: PhoenixCapabilityAbiTable::new(backend),
            idempotent_ledger: BTreeMap::new(),
            slot_m8_byte_len: BTreeMap::new(),
            next_slot_cursor: 1,
            next_arena_block_cursor: 0,
        }
    }

    // ------------------------------------------------------------------------
    // Gateway 1 (Pole +1): Stripe `Stripe-Signature: t=<unix>,v1=<hex>`
    // ------------------------------------------------------------------------
    pub fn handle_stripe_webhook(
        &mut self,
        stripe_sig_header: &str,
        raw_body: &[u8],
        server_now_unix: u64,
    ) -> Result<SignedM8DownloadTicket, WebhookVerificationError> {
        let mut timestamp_opt: Option<u64> = None;
        let mut sig_v1_opt: Option<[u8; 32]> = None;

        for part in stripe_sig_header.split(',') {
            let trimmed = part.trim();
            if let Some(t_str) = trimmed.strip_prefix("t=") {
                timestamp_opt = t_str.parse::<u64>().ok();
            } else if let Some(v1_str) = trimmed.strip_prefix("v1=") {
                sig_v1_opt = hex_decode_32(v1_str);
            }
        }

        let ts =
            timestamp_opt.ok_or(WebhookVerificationError::MissingOrMalformedSignatureHeader)?;
        let received_hmac = sig_v1_opt.ok_or(WebhookVerificationError::InvalidHexSignature)?;

        Self::verify_replay_window(ts, server_now_unix)?;

        let mut signed_payload = format!("{ts}.").into_bytes();
        signed_payload.extend_from_slice(raw_body);
        let expected_hmac = hmac_sha256(&self.secrets.stripe_webhook_secret, &signed_payload);

        if !constant_time_eq_32(&expected_hmac, &received_hmac) {
            return Err(WebhookVerificationError::HmacSignatureMismatch);
        }

        let payload: CheckoutWebhookPayload = serde_json::from_slice(raw_body)
            .map_err(|e| WebhookVerificationError::MalformedJsonPayload(e.to_string()))?;

        if payload.event_type != "checkout.session.completed" {
            return Err(WebhookVerificationError::UnsupportedEventType(
                payload.event_type,
            ));
        }

        self.mint_idempotent_m8_ticket(PaymentGatewayPole::StripePlus, payload, server_now_unix)
    }

    // ------------------------------------------------------------------------
    // Gateway 2 (Pole 0): Revolut `Revolut-Request-Timestamp` & `Revolut-Signature: v1=<hex>`
    // ------------------------------------------------------------------------
    pub fn handle_revolut_webhook(
        &mut self,
        revolut_timestamp_header: &str,
        revolut_sig_header: &str,
        raw_body: &[u8],
        server_now_unix: u64,
    ) -> Result<SignedM8DownloadTicket, WebhookVerificationError> {
        let raw_ts = revolut_timestamp_header
            .trim()
            .parse::<u64>()
            .map_err(|_| WebhookVerificationError::MissingOrMalformedSignatureHeader)?;
        let ts_secs = if raw_ts > 100_000_000_000 {
            raw_ts / 1_000
        } else {
            raw_ts
        };

        let v1_hex = revolut_sig_header
            .trim()
            .strip_prefix("v1=")
            .ok_or(WebhookVerificationError::MissingOrMalformedSignatureHeader)?;
        let received_hmac =
            hex_decode_32(v1_hex).ok_or(WebhookVerificationError::InvalidHexSignature)?;

        Self::verify_replay_window(ts_secs, server_now_unix)?;

        let mut signed_payload = format!("v1.{}.", revolut_timestamp_header.trim()).into_bytes();
        signed_payload.extend_from_slice(raw_body);
        let expected_hmac = hmac_sha256(&self.secrets.revolut_webhook_secret, &signed_payload);

        if !constant_time_eq_32(&expected_hmac, &received_hmac) {
            return Err(WebhookVerificationError::HmacSignatureMismatch);
        }

        let payload: CheckoutWebhookPayload = serde_json::from_slice(raw_body)
            .map_err(|e| WebhookVerificationError::MalformedJsonPayload(e.to_string()))?;

        if payload.event_type != "ORDER_COMPLETED" {
            return Err(WebhookVerificationError::UnsupportedEventType(
                payload.event_type,
            ));
        }

        self.mint_idempotent_m8_ticket(PaymentGatewayPole::RevolutZero, payload, server_now_unix)
    }

    // ------------------------------------------------------------------------
    // Gateway 3 (Pole -1): PayPal `PayPal-Transmission-Id`, `Time`, `Sig`
    // ------------------------------------------------------------------------
    pub fn handle_paypal_webhook(
        &mut self,
        transmission_id: &str,
        transmission_time_secs: &str,
        transmission_sig_header: &str,
        raw_body: &[u8],
        server_now_unix: u64,
    ) -> Result<SignedM8DownloadTicket, WebhookVerificationError> {
        let ts = transmission_time_secs
            .trim()
            .parse::<u64>()
            .map_err(|_| WebhookVerificationError::MissingOrMalformedSignatureHeader)?;

        let sig_hex = transmission_sig_header
            .trim()
            .strip_prefix("v1=")
            .unwrap_or_else(|| transmission_sig_header.trim());
        let received_hmac =
            hex_decode_32(sig_hex).ok_or(WebhookVerificationError::InvalidHexSignature)?;

        Self::verify_replay_window(ts, server_now_unix)?;

        let body_fnv = fnv1a_64_const(raw_body);
        let signing_str = format!(
            "{}|{}|{}|{:016x}",
            transmission_id.trim(),
            ts,
            self.secrets.paypal_webhook_id,
            body_fnv
        );
        let expected_hmac =
            hmac_sha256(&self.secrets.paypal_webhook_secret, signing_str.as_bytes());

        if !constant_time_eq_32(&expected_hmac, &received_hmac) {
            return Err(WebhookVerificationError::HmacSignatureMismatch);
        }

        let payload: CheckoutWebhookPayload = serde_json::from_slice(raw_body)
            .map_err(|e| WebhookVerificationError::MalformedJsonPayload(e.to_string()))?;

        if payload.event_type != "PAYMENT.CAPTURE.COMPLETED" {
            return Err(WebhookVerificationError::UnsupportedEventType(
                payload.event_type,
            ));
        }

        self.mint_idempotent_m8_ticket(PaymentGatewayPole::PayPalMinus, payload, server_now_unix)
    }

    #[inline]
    fn verify_replay_window(
        webhook_ts: u64,
        server_now: u64,
    ) -> Result<(), WebhookVerificationError> {
        let skew = webhook_ts.abs_diff(server_now);
        if skew > WEBHOOK_MAX_REPLAY_SKEW_SECS {
            return Err(WebhookVerificationError::ReplayTimestampOutOfBounds {
                webhook_ts,
                server_now,
                max_skew: WEBHOOK_MAX_REPLAY_SKEW_SECS,
            });
        }
        Ok(())
    }

    fn mint_idempotent_m8_ticket(
        &mut self,
        gateway_pole: PaymentGatewayPole,
        payload: CheckoutWebhookPayload,
        server_now_unix: u64,
    ) -> Result<SignedM8DownloadTicket, WebhookVerificationError> {
        if payload.amount_minor_units == 0 {
            return Err(WebhookVerificationError::ZeroPaymentAmount);
        }

        if let Some(existing_ticket) = self.idempotent_ledger.get(&payload.event_id) {
            return Ok(existing_ticket.clone());
        }

        let granted_download_caps: CapEffectMask = CAP_ARENA | CAP_M8;
        verify_charm_18_sovereign_secrecy(true, granted_download_caps)
            .map_err(|_| WebhookVerificationError::Charm18RootSealViolation)?;
        if (granted_download_caps & CAP_ROOT_SEAL) != 0 {
            return Err(WebhookVerificationError::Charm18RootSealViolation);
        }

        let seed_phrase = if payload.persona_seed_sku.is_empty() {
            OMNI_GENESIS_SEED_PHRASE
        } else {
            payload.persona_seed_sku.as_str()
        };
        let m8_container = M8PersonaContainer::generate_from_seed_phrase(seed_phrase)
            .map_err(WebhookVerificationError::MalformedJsonPayload)?;
        let m8_bytes = m8_container.to_m8_bytes();
        let blocks_needed = m8_bytes.len().div_ceil(CAP_BLOCK_BYTES) as u32;

        let slot_id = self.next_slot_cursor;
        if slot_id == 255 {
            return Err(WebhookVerificationError::CapabilityTableExhausted);
        }
        self.next_slot_cursor = self.next_slot_cursor.saturating_add(1);

        let start_block = self.next_arena_block_cursor;
        self.next_arena_block_cursor = self.next_arena_block_cursor.saturating_add(blocks_needed);

        let futhark_tag_r2 = (gateway_pole.futhark_aett_base() + (slot_id % 9)).min(26);

        let _staging_handle = self
            .cap_table
            .mint_capability_slot(
                slot_id,
                start_block,
                blocks_needed,
                granted_download_caps,
                1,
                futhark_tag_r2,
                1,
            )
            .map_err(WebhookVerificationError::CapabilityAccessFault)?;

        for b_idx in 0..(blocks_needed as usize) {
            let mut line = [0u8; CAP_BLOCK_BYTES];
            let start = b_idx * CAP_BLOCK_BYTES;
            let end = (start + CAP_BLOCK_BYTES).min(m8_bytes.len());
            line[..end - start].copy_from_slice(&m8_bytes[start..end]);

            let blk_handle =
                VirtualCapHandle64::encode(1, slot_id, futhark_tag_r2, b_idx as u32, 0);
            self.cap_table
                .invoke_cap(blk_handle, PhoenixCapOp::ArenaWrite64B { data: line })
                .map_err(WebhookVerificationError::CapabilityAccessFault)?;
        }

        let ro_handle = self
            .cap_table
            .mint_capability_slot(
                slot_id,
                start_block,
                blocks_needed,
                granted_download_caps,
                1,
                futhark_tag_r2,
                0,
            )
            .map_err(WebhookVerificationError::CapabilityAccessFault)?;

        self.slot_m8_byte_len.insert(slot_id, m8_bytes.len());

        let well_text = format!(
            "- source: phoenix-commerce\n- kind: m8-container\n- title: {} mantra prayer\n- artist: Peter Jozsef Lodri\n- album: {}\n- year: 2026\n- genre: Ternary Persona\n- url: https://8b.is/m8/{}\n- duration: 00:40\n- track_count: 1\n",
            payload.persona_seed_sku, payload.order_reference, payload.event_id
        );
        let well_table = WellRecord::parse_zero_alloc(&well_text).resolve_signals();
        let well_signal_spqt3 = pack_signal_table_spqt3(&well_table, 2);

        let expires_at_unix = server_now_unix + DOWNLOAD_TICKET_TTL_SECS;
        let canonical = SignedM8DownloadTicket::canonical_signing_bytes(
            gateway_pole,
            &payload.event_id,
            &payload.order_reference,
            ro_handle,
            granted_download_caps,
            m8_container.container_pcc_seal_u64,
            &well_signal_spqt3,
            server_now_unix,
            expires_at_unix,
        );
        let sig_bytes = hmac_sha256(&self.secrets.enclave_root_seal_secret, &canonical);

        let ticket = SignedM8DownloadTicket {
            gateway_pole,
            event_id: payload.event_id.clone(),
            order_reference: payload.order_reference,
            virt_cap_handle: ro_handle,
            granted_download_caps,
            container_pcc_seal_u64: m8_container.container_pcc_seal_u64,
            well_signal_spqt3,
            issued_at_unix: server_now_unix,
            expires_at_unix,
            enclave_hmac_sig_hex: hex_encode_bytes(&sig_bytes),
        };

        self.idempotent_ledger
            .insert(payload.event_id, ticket.clone());
        Ok(ticket)
    }

    pub fn redeem_m8_download_ticket(
        &mut self,
        ticket: &SignedM8DownloadTicket,
        server_now_unix: u64,
        consume_one_time_epoch: bool,
    ) -> Result<Vec<u8>, WebhookVerificationError> {
        if server_now_unix > ticket.expires_at_unix {
            return Err(WebhookVerificationError::TicketInvalidOrExpired);
        }
        if (ticket.granted_download_caps & CAP_ROOT_SEAL) != 0 {
            return Err(WebhookVerificationError::Charm18RootSealViolation);
        }

        let canonical = SignedM8DownloadTicket::canonical_signing_bytes(
            ticket.gateway_pole,
            &ticket.event_id,
            &ticket.order_reference,
            ticket.virt_cap_handle,
            ticket.granted_download_caps,
            ticket.container_pcc_seal_u64,
            &ticket.well_signal_spqt3,
            ticket.issued_at_unix,
            ticket.expires_at_unix,
        );
        let expected_sig = hmac_sha256(&self.secrets.enclave_root_seal_secret, &canonical);
        let provided_sig = hex_decode_32(&ticket.enclave_hmac_sig_hex)
            .ok_or(WebhookVerificationError::TicketInvalidOrExpired)?;

        if !constant_time_eq_32(&expected_sig, &provided_sig) {
            return Err(WebhookVerificationError::TicketInvalidOrExpired);
        }

        let slot_id = ticket.virt_cap_handle.cap_slot_id();
        let byte_len = *self
            .slot_m8_byte_len
            .get(&slot_id)
            .ok_or(WebhookVerificationError::TicketInvalidOrExpired)?;
        let blocks = byte_len.div_ceil(CAP_BLOCK_BYTES) as u32;

        let mut raw_m8 = Vec::with_capacity((blocks as usize) * CAP_BLOCK_BYTES);
        for b_idx in 0..blocks {
            let blk_handle = VirtualCapHandle64::encode(
                ticket.virt_cap_handle.epoch_gen(),
                slot_id,
                ticket.virt_cap_handle.futhark_tag_r2(),
                b_idx,
                0,
            );
            let res = self
                .cap_table
                .invoke_cap(blk_handle, PhoenixCapOp::ArenaRead64B)
                .map_err(WebhookVerificationError::CapabilityAccessFault)?;
            raw_m8.extend_from_slice(&res.read_block_64b.unwrap());
        }
        raw_m8.truncate(byte_len);

        let verified_m8 = M8PersonaContainer::from_m8_bytes_verified(&raw_m8)
            .map_err(WebhookVerificationError::MalformedJsonPayload)?;
        if verified_m8.container_pcc_seal_u64 != ticket.container_pcc_seal_u64 {
            return Err(WebhookVerificationError::TicketInvalidOrExpired);
        }

        if consume_one_time_epoch {
            self.cap_table
                .advance_slot_epoch_for_hotswap(slot_id)
                .map_err(WebhookVerificationError::CapabilityAccessFault)?;
        }

        Ok(raw_m8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        phoenix_cap_abi::{PosixMmapArenaBackend, DEFAULT_ARENA_BLOCKS},
        well_signal_spqt_espnow::unpack_signal_table_spqt3,
    };

    #[test]
    fn test_self_hosted_stripe_revolut_paypal_webhooks_and_m8_capability_redemption() {
        let secrets = M8WebhookSecrets::new(
            "whsec_stripe_secret_2026",
            "whsec_revolut_secret_2026",
            "whsec_paypal_secret_2026",
            "WH-8B-IS-PAYPAL-2026",
            "enclave_charm18_root_seal_master_key_never_exported",
        );

        let mmap_backend =
            PosixMmapArenaBackend::allocate_shared(DEFAULT_ARENA_BLOCKS as u32).unwrap();
        let mut server = M8CommerceWebhookServer::new(secrets, mmap_backend);
        let now_unix: u64 = 1_762_540_000;

        let stripe_event = CheckoutWebhookPayload {
            event_id: "evt_stripe_allfather_001".into(),
            event_type: "checkout.session.completed".into(),
            order_reference: "ORD-STRIPE-01".into(),
            persona_seed_sku: OMNI_GENESIS_SEED_PHRASE.into(),
            amount_minor_units: 4_900,
            currency: "USD".into(),
        };
        let stripe_body = serde_json::to_vec(&stripe_event).unwrap();
        let mut stripe_signing_input = format!("{now_unix}.").into_bytes();
        stripe_signing_input.extend_from_slice(&stripe_body);
        let stripe_hmac = hmac_sha256(b"whsec_stripe_secret_2026", &stripe_signing_input);
        let stripe_header = format!("t={now_unix},v1={}", hex_encode_bytes(&stripe_hmac));

        let stripe_ticket = server
            .handle_stripe_webhook(&stripe_header, &stripe_body, now_unix)
            .expect("Valid Stripe webhook must mint a SignedM8DownloadTicket");

        assert_eq!(stripe_ticket.gateway_pole, PaymentGatewayPole::StripePlus);
        assert!(stripe_ticket.virt_cap_handle.futhark_tag_r2() < 9);

        let stripe_retry_ticket = server
            .handle_stripe_webhook(&stripe_header, &stripe_body, now_unix + 5)
            .unwrap();
        assert_eq!(stripe_ticket, stripe_retry_ticket);
        assert_eq!(server.next_slot_cursor, 2);

        let revolut_event = CheckoutWebhookPayload {
            event_id: "evt_revolut_sun_002".into(),
            event_type: "ORDER_COMPLETED".into(),
            order_reference: "ORD-REV-02".into(),
            persona_seed_sku: OMNI_GENESIS_SEED_PHRASE.into(),
            amount_minor_units: 4_500,
            currency: "EUR".into(),
        };
        let revolut_body = serde_json::to_vec(&revolut_event).unwrap();
        let rev_ts_ms = (now_unix * 1_000).to_string();
        let mut rev_signing_input = format!("v1.{rev_ts_ms}.").into_bytes();
        rev_signing_input.extend_from_slice(&revolut_body);
        let rev_hmac = hmac_sha256(b"whsec_revolut_secret_2026", &rev_signing_input);
        let rev_sig_header = format!("v1={}", hex_encode_bytes(&rev_hmac));

        let revolut_ticket = server
            .handle_revolut_webhook(&rev_ts_ms, &rev_sig_header, &revolut_body, now_unix)
            .expect("Valid Revolut Merchant webhook must mint a SignedM8DownloadTicket");

        assert_eq!(revolut_ticket.gateway_pole, PaymentGatewayPole::RevolutZero);
        assert!((9..18).contains(&revolut_ticket.virt_cap_handle.futhark_tag_r2()));

        let paypal_event = CheckoutWebhookPayload {
            event_id: "evt_paypal_dharma_003".into(),
            event_type: "PAYMENT.CAPTURE.COMPLETED".into(),
            order_reference: "ORD-PP-03".into(),
            persona_seed_sku: OMNI_GENESIS_SEED_PHRASE.into(),
            amount_minor_units: 4_900,
            currency: "USD".into(),
        };
        let paypal_body = serde_json::to_vec(&paypal_event).unwrap();
        let tx_id = "PP-TX-998877";
        let body_fnv = fnv1a_64_const(&paypal_body);
        let pp_signing_str = format!("{tx_id}|{now_unix}|WH-8B-IS-PAYPAL-2026|{body_fnv:016x}");
        let pp_hmac = hmac_sha256(b"whsec_paypal_secret_2026", pp_signing_str.as_bytes());
        let pp_sig_header = format!("v1={}", hex_encode_bytes(&pp_hmac));

        let paypal_ticket = server
            .handle_paypal_webhook(
                tx_id,
                &now_unix.to_string(),
                &pp_sig_header,
                &paypal_body,
                now_unix,
            )
            .expect("Valid PayPal webhook must mint a SignedM8DownloadTicket");

        assert_eq!(paypal_ticket.gateway_pole, PaymentGatewayPole::PayPalMinus);
        assert!((18..27).contains(&paypal_ticket.virt_cap_handle.futhark_tag_r2()));

        assert_eq!(
            (stripe_ticket.gateway_pole as i8)
                + (revolut_ticket.gateway_pole as i8)
                + (paypal_ticket.gateway_pole as i8),
            0
        );

        let well_table = unpack_signal_table_spqt3(&stripe_ticket.well_signal_spqt3, 2)
            .expect("Ticket SPQT3 well signal table must pass Z_3 parity");
        assert_eq!(well_table.total_present_out_of_12(), 12);

        let forged_header = format!("t={now_unix},v1={}", "aa".repeat(32));
        assert_eq!(
            server.handle_stripe_webhook(&forged_header, &stripe_body, now_unix),
            Err(WebhookVerificationError::HmacSignatureMismatch)
        );

        assert!(matches!(
            server.handle_stripe_webhook(&stripe_header, &stripe_body, now_unix + 601),
            Err(WebhookVerificationError::ReplayTimestampOutOfBounds { .. })
        ));

        assert!(matches!(
            server.cap_table.invoke_cap(
                stripe_ticket.virt_cap_handle,
                PhoenixCapOp::ArenaWrite64B {
                    data: [0xFF; CAP_BLOCK_BYTES]
                }
            ),
            Err(CapAbiFault::TernaryPermissionDenied {
                trit_perm: 0,
                attempted_write: true
            })
        ));

        let downloaded_m8_bytes = server
            .redeem_m8_download_ticket(&stripe_ticket, now_unix + 10, true)
            .expect("Valid SignedM8DownloadTicket must stream the verified .m8 container");

        let verified_container = M8PersonaContainer::from_m8_bytes_verified(&downloaded_m8_bytes)
            .expect("Downloaded .m8 bytes must pass PHOENIX PCC verification");
        assert_eq!(
            verified_container.container_pcc_seal_u64,
            stripe_ticket.container_pcc_seal_u64
        );

        assert_eq!(
            server.redeem_m8_download_ticket(&stripe_ticket, now_unix + 15, true),
            Err(WebhookVerificationError::CapabilityAccessFault(
                CapAbiFault::StaleEpochUseAfterFree {
                    handle_epoch: 1,
                    current_epoch: 2
                }
            ))
        );
    }
}

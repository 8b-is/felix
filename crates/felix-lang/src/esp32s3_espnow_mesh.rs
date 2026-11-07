//! `felix/crates/felix-lang/src/esp32s3_espnow_mesh.rs`
//!
//! 216-Byte ESP32-S3 ESP-NOW Capability & `quantTernP/Q` Voice Frame (`EspNowQuantVoiceFrame`).

use crate::{
    agp_cap_iommu_tlb::VirtualCapHandle64,
    felix_phoenix::{encode_e2m1_nibble, pack_spqt_base243_byte, CapEffectMask},
    ljodatal_18_charms::CAP_ROOT_SEAL,
};
use serde::{Deserialize, Serialize};

pub const M8_ESPNOW_MAGIC: [u8; 4] = *b"M8VN";
pub const M8_ESPNOW_FRAME_BYTES: usize = 216;
pub const VOICE_FRAME_TRITS: usize = 640;
pub const VOICE_P_PLANE_BYTES: usize = 128;
pub const VOICE_Q_PLANE_BYTES: usize = 10;
pub const ESPNOW_BROADCAST_MAC: [u8; 6] = [0xFF; 6];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EspNowFrameKind {
    VoiceQuantTernStream = 0,
    M8CrdtCapabilitySync = 1,
    WakeWordCommandTrigger = 2,
    ByzantineHeartbeat = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i8)]
pub enum MeshPoleNode {
    DharmaEnclave = -1,
    SunRelay = 0,
    AllFatherIngress = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EspNowMeshFault {
    InvalidMagicHeader,
    MtuExceeded(usize),
    Charm18RootSealRfLeak,
    ByzantinePccSealMismatch { expected: u64, received: u64 },
    IommuCapabilityFault(u8),
}

pub trait EspNowRadioHal {
    fn send_espnow_frame(
        &mut self,
        dst_mac: &[u8; 6],
        payload: &[u8; M8_ESPNOW_FRAME_BYTES],
    ) -> Result<(), EspNowMeshFault>;
}

/// Computes the 64-bit FNV-1a Proof-Carrying Code seal over the ESP-NOW header + payload slice.
#[inline]
pub fn compute_frame_pcc_seal(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub type QuantVoiceEncodedTuple = (
    i16,
    [u8; VOICE_P_PLANE_BYTES],
    [u8; VOICE_Q_PLANE_BYTES],
    [(i16, i16); 8],
);

/// Encodes 640 16-bit PCM voice samples into 128 SPQT Base-243 P-plane bytes (`640 / 5 = 128`),
/// 10 E2M1 Q-plane scale bytes (`20 x 4-bit`), and an 8-mode Sleipnir Q15 harmonic summary.
pub fn encode_pcm640_to_quant_tern_pq(
    pcm_640: &[i16; VOICE_FRAME_TRITS],
    base_step_q: u16,
    bend_k: u8,
) -> QuantVoiceEncodedTuple {
    let pred_init = pcm_640[0];
    let step = (base_step_q.max(1)) as i32;
    let mut trits = [0i8; VOICE_FRAME_TRITS];
    let mut prev = pred_init as i32;
    let k_step = (bend_k % 3) as usize;

    for i in 0..VOICE_FRAME_TRITS {
        let diff = (pcm_640[i] as i32) - prev;
        let t = if diff > step / 2 {
            prev += step;
            1i8
        } else if diff < -step / 2 {
            prev -= step;
            -1i8
        } else {
            0i8
        };
        let u = (t + 1) as u8;
        let bent_u = (u + ((k_step * i) % 3) as u8) % 3;
        trits[i] = (bent_u as i8) - 1;
    }

    let mut p_plane = [0u8; VOICE_P_PLANE_BYTES];
    for g in 0..VOICE_P_PLANE_BYTES {
        p_plane[g] = pack_spqt_base243_byte(&trits[g * 5..g * 5 + 5]);
    }

    let mut q_plane = [0u8; VOICE_Q_PLANE_BYTES];
    let nib = encode_e2m1_nibble(1.0);
    q_plane.fill((nib << 4) | (nib & 0x0F));

    let mut wave_q15 = [(0i16, 0i16); 8];
    for (m, slot) in wave_q15.iter_mut().enumerate() {
        *slot = (pcm_640[m * 8], pcm_640[m * 8 + 1]);
    }

    (pred_init, p_plane, q_plane, wave_q15)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EspNowQuantVoiceFrame {
    pub magic: [u8; 4],
    pub frame_kind: EspNowFrameKind,
    pub bend_k: u8,
    pub rok_invert_aett: bool,
    pub origin_pole: MeshPoleNode,
    pub seq_num: u16,
    pub virt_cap_handle: VirtualCapHandle64,
    pub declared_caps: CapEffectMask,
    pub predictor_init_i16: i16,
    pub base_step_q: u16,
    pub triadic_vclock: [u32; 3],
    pub sleipnir_wave_q15: [(i16, i16); 8],
    pub p_plane_base243: [u8; VOICE_P_PLANE_BYTES],
    pub q_plane_e2m1: [u8; VOICE_Q_PLANE_BYTES],
    pub futhark_tag_r2: u8,
    pub command_id: u8,
    pub pcc_fnv1a_seal_u64: u64,
}

impl EspNowQuantVoiceFrame {
    pub fn to_wire_bytes(&self) -> [u8; M8_ESPNOW_FRAME_BYTES] {
        let mut buf = [0u8; M8_ESPNOW_FRAME_BYTES];
        buf[0..4].copy_from_slice(&self.magic);
        let flags = ((self.frame_kind as u8) & 0x03)
            | (((self.bend_k % 3) & 0x03) << 2)
            | (if self.rok_invert_aett { 0x10 } else { 0x00 });
        buf[4] = flags;
        buf[5] = self.origin_pole as i8 as u8;
        buf[6..8].copy_from_slice(&self.seq_num.to_le_bytes());
        buf[8..16].copy_from_slice(&self.virt_cap_handle.0.to_le_bytes());
        buf[16..20].copy_from_slice(&self.declared_caps.to_le_bytes());
        buf[20..22].copy_from_slice(&self.predictor_init_i16.to_le_bytes());
        buf[22..24].copy_from_slice(&self.base_step_q.to_le_bytes());
        for i in 0..3 {
            buf[24 + i * 4..28 + i * 4].copy_from_slice(&self.triadic_vclock[i].to_le_bytes());
        }
        for m in 0..8 {
            let base = 36 + m * 4;
            buf[base..base + 2].copy_from_slice(&self.sleipnir_wave_q15[m].0.to_le_bytes());
            buf[base + 2..base + 4].copy_from_slice(&self.sleipnir_wave_q15[m].1.to_le_bytes());
        }
        buf[68..196].copy_from_slice(&self.p_plane_base243);
        buf[196..206].copy_from_slice(&self.q_plane_e2m1);
        buf[206] = self.futhark_tag_r2;
        buf[207] = self.command_id;

        let seal = compute_frame_pcc_seal(&buf[0..208]);
        buf[208..216].copy_from_slice(&seal.to_le_bytes());
        buf
    }

    pub fn from_wire_bytes_verified(
        buf: &[u8; M8_ESPNOW_FRAME_BYTES],
    ) -> Result<Self, EspNowMeshFault> {
        if buf[0..4] != M8_ESPNOW_MAGIC {
            return Err(EspNowMeshFault::InvalidMagicHeader);
        }
        let expected_seal = compute_frame_pcc_seal(&buf[0..208]);
        let wire_seal = u64::from_le_bytes(buf[208..216].try_into().unwrap());
        if wire_seal != expected_seal {
            return Err(EspNowMeshFault::ByzantinePccSealMismatch {
                expected: expected_seal,
                received: wire_seal,
            });
        }
        let declared_caps = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        if (declared_caps & CAP_ROOT_SEAL) != 0 {
            return Err(EspNowMeshFault::Charm18RootSealRfLeak);
        }

        let flags = buf[4];
        let frame_kind = match flags & 0x03 {
            0 => EspNowFrameKind::VoiceQuantTernStream,
            1 => EspNowFrameKind::M8CrdtCapabilitySync,
            2 => EspNowFrameKind::WakeWordCommandTrigger,
            _ => EspNowFrameKind::ByzantineHeartbeat,
        };
        let bend_k = (flags >> 2) & 0x03;
        let rok_invert_aett = (flags & 0x10) != 0;
        let origin_pole = match buf[5] as i8 {
            -1 => MeshPoleNode::DharmaEnclave,
            0 => MeshPoleNode::SunRelay,
            _ => MeshPoleNode::AllFatherIngress,
        };
        let seq_num = u16::from_le_bytes(buf[6..8].try_into().unwrap());
        let virt_cap_handle = VirtualCapHandle64(u64::from_le_bytes(buf[8..16].try_into().unwrap()));
        let predictor_init_i16 = i16::from_le_bytes(buf[20..22].try_into().unwrap());
        let base_step_q = u16::from_le_bytes(buf[22..24].try_into().unwrap());
        let mut triadic_vclock = [0u32; 3];
        for i in 0..3 {
            triadic_vclock[i] =
                u32::from_le_bytes(buf[24 + i * 4..28 + i * 4].try_into().unwrap());
        }
        let mut sleipnir_wave_q15 = [(0i16, 0i16); 8];
        for (m, slot) in sleipnir_wave_q15.iter_mut().enumerate() {
            let base = 36 + m * 4;
            let re = i16::from_le_bytes(buf[base..base + 2].try_into().unwrap());
            let im = i16::from_le_bytes(buf[base + 2..base + 4].try_into().unwrap());
            *slot = (re, im);
        }
        let mut p_plane_base243 = [0u8; VOICE_P_PLANE_BYTES];
        p_plane_base243.copy_from_slice(&buf[68..196]);
        let mut q_plane_e2m1 = [0u8; VOICE_Q_PLANE_BYTES];
        q_plane_e2m1.copy_from_slice(&buf[196..206]);

        Ok(Self {
            magic: M8_ESPNOW_MAGIC,
            frame_kind,
            bend_k,
            rok_invert_aett,
            origin_pole,
            seq_num,
            virt_cap_handle,
            declared_caps,
            predictor_init_i16,
            base_step_q,
            triadic_vclock,
            sleipnir_wave_q15,
            p_plane_base243,
            q_plane_e2m1,
            futhark_tag_r2: buf[206],
            command_id: buf[207],
            pcc_fnv1a_seal_u64: wire_seal,
        })
    }
}

pub struct Esp32S3MeshNodeBridge {
    pub pole: MeshPoleNode,
    pub seq_counter: u16,
    pub vclock: [u32; 3],
}

impl Esp32S3MeshNodeBridge {
    pub fn new(pole: MeshPoleNode) -> Self {
        Self {
            pole,
            seq_counter: 0,
            vclock: [0, 0, 0],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transmit_voice_capability_frame<R: EspNowRadioHal>(
        &mut self,
        radio: &mut R,
        dst_mac: &[u8; 6],
        pcm_640: &[i16; VOICE_FRAME_TRITS],
        virt_cap_handle: VirtualCapHandle64,
        declared_caps: CapEffectMask,
        base_step_q: u16,
        bend_k: u8,
        command_id: u8,
    ) -> Result<[u8; M8_ESPNOW_FRAME_BYTES], EspNowMeshFault> {
        if (declared_caps & CAP_ROOT_SEAL) != 0 {
            return Err(EspNowMeshFault::Charm18RootSealRfLeak);
        }
        self.seq_counter = self.seq_counter.wrapping_add(1);
        let idx = ((self.pole as i8) + 1) as usize;
        self.vclock[idx] = self.vclock[idx].wrapping_add(1);

        let (pred_init, p_plane, q_plane, wave_q15) =
            encode_pcm640_to_quant_tern_pq(pcm_640, base_step_q, bend_k);
        let frame = EspNowQuantVoiceFrame {
            magic: M8_ESPNOW_MAGIC,
            frame_kind: EspNowFrameKind::VoiceQuantTernStream,
            bend_k: bend_k % 3,
            rok_invert_aett: true,
            origin_pole: self.pole,
            seq_num: self.seq_counter,
            virt_cap_handle,
            declared_caps,
            predictor_init_i16: pred_init,
            base_step_q,
            triadic_vclock: self.vclock,
            sleipnir_wave_q15: wave_q15,
            p_plane_base243: p_plane,
            q_plane_e2m1: q_plane,
            futhark_tag_r2: virt_cap_handle.futhark_tag_r2(),
            command_id,
            pcc_fnv1a_seal_u64: 0,
        };
        let wire = frame.to_wire_bytes();
        radio.send_espnow_frame(dst_mac, &wire)?;
        Ok(wire)
    }
}

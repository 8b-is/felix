use crate::phoenix::{
    felix_ternary_gemm_i32, FelixPhoenixModule, NandSsaOp, CAP_ARENA, CAP_IO, CAP_M8, CAP_NONE,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
// 1. Target ISA Selection & Deterministic Bytecode Artifact
// ============================================================================

pub type CapEffectMask = u8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JitTargetIsa {
    /// x86-64 with AVX-512 (`vpternlogd` + `vpmaddwd` + `vpaddd`).
    X86_64Avx512,
    /// ARM64 / AArch64 with NEON (`orr`/`and`/`bsl` + `smull`/`smlal`/`addv`).
    AArch64Neon,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmittedFunctionSymbol {
    pub name: String,
    pub byte_offset: usize,
    pub byte_len: usize,
    pub safe_point_offsets: Vec<usize>,
    pub tgemm_patch_sites: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeCodegenArtifact {
    pub module_name: String,
    pub target_isa: JitTargetIsa,
    /// Sorted map (`BTreeMap`) of symbol offsets guaranteeing deterministic emission order.
    pub symbols: BTreeMap<String, EmittedFunctionSymbol>,
    pub machine_code: Vec<u8>,
    pub fnv1a_code_digest: u64,
    pub vpternlogd_count: usize,
    pub vpmaddwd_or_smlal_count: usize,
}

// ============================================================================
// 2. x86-64 AVX-512 (`vpternlogd` + `vpmaddwd`) Deterministic Bytecode Emitter
// ============================================================================

/// Encodes a 7-byte x86-64 AVX-512 EVEX `vpternlogd zmm_dst, zmm_src1, zmm_src2, imm8` instruction.
#[inline]
pub fn encode_x86_64_evex_vpternlogd(
    zmm_dst: u8,
    zmm_src1: u8,
    zmm_src2: u8,
    imm8: u8,
) -> [u8; 7] {
    let dst_3 = zmm_dst & 0x07;
    let src1_4 = (!zmm_src1) & 0x0F;
    let src2_3 = zmm_src2 & 0x07;

    let p0 = 0xF3; // R'=1, X=1, B=1, R=1, map=0F3A (0b11)
    let p1 = (src1_4 << 3) | 0x05; // W=0 (32-bit dword), 1, pp=01 (0x66)
    let p2 = 0x48; // z=0, L'L=10 (512-bit), b=0, V'=1, aaa=000
    let modrm = 0xC0 | (dst_3 << 3) | src2_3;

    [0x62, p0, p1, p2, 0x25, modrm, imm8]
}

/// Encodes a 4-byte x86-64 VEX `vpmaddwd ymm_dst, ymm_src1, ymm_src2` (`0xC5, P0, 0xF5, ModRM`)
/// followed by a 4-byte VEX `vpaddd ymm_acc, ymm_acc, ymm_dst` (`0xC5, P0, 0xFE, ModRM`).
#[inline]
pub fn encode_x86_64_vex_vpmaddwd_vpaddd(
    ymm_acc: u8,
    ymm_tmp: u8,
    ymm_acts: u8,
    ymm_trits: u8,
) -> [u8; 8] {
    let vvvv_acts = (!ymm_acts) & 0x0F;
    let p0_madd = 0x80 | (vvvv_acts << 3) | 0x05; // R=1, L=1 (256-bit), pp=01 (66)
    let modrm_madd = 0xC0 | ((ymm_tmp & 7) << 3) | (ymm_trits & 7);

    let vvvv_acc = (!ymm_acc) & 0x0F;
    let p0_add = 0x80 | (vvvv_acc << 3) | 0x05;
    let modrm_add = 0xC0 | ((ymm_acc & 7) << 3) | (ymm_tmp & 7);

    [
        0xC5, p0_madd, 0xF5, modrm_madd, // vpmaddwd ymm_tmp, ymm_acts, ymm_trits
        0xC5, p0_add, 0xFE, modrm_add,   // vpaddd   ymm_acc, ymm_acc, ymm_tmp
    ]
}

/// Emits a deterministic x86-64 AVX-512 native machine-code image for `module`.
pub fn emit_x86_64_avx512_module(module: &FelixPhoenixModule) -> Result<NativeCodegenArtifact, String> {
    let mut code = Vec::with_capacity(512);
    let mut symbols = BTreeMap::new();
    let mut vpternlogd_count = 0usize;
    let mut vpmaddwd_count = 0usize;

    let start_off = 0;
    let mut safe_points = Vec::new();
    let mut tgemm_sites = Vec::new();

    // System V AMD64 Prologue: `push rbp; mov rbp, rsp`
    code.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5]);

    for op in &module.ops {
        match op {
            NandSsaOp::Yield { .. } => {
                safe_points.push(code.len());
                code.extend_from_slice(&[0x8B, 0x07, 0x85, 0xC0, 0x75, 0x02]);
            }
            NandSsaOp::Nand { dst, lhs, rhs } => {
                let insn_pos = encode_x86_64_evex_vpternlogd(*dst, *lhs, *rhs, 0xFC);
                let insn_neg = encode_x86_64_evex_vpternlogd(dst.wrapping_add(1), lhs.wrapping_add(1), rhs.wrapping_add(1), 0xC0);
                code.extend_from_slice(&insn_pos);
                code.extend_from_slice(&insn_neg);
                vpternlogd_count += 2;
            }
            NandSsaOp::TGemm { dst, .. } => {
                tgemm_sites.push(code.len());
                let sign_mux = encode_x86_64_evex_vpternlogd(*dst & 7, 1, 2, 0xCA);
                let madd_pair = encode_x86_64_vex_vpmaddwd_vpaddd(*dst & 7, 3, 1, 2);
                code.extend_from_slice(&sign_mux);
                code.extend_from_slice(&madd_pair);
                vpternlogd_count += 1;
                vpmaddwd_count += 1;
            }
            NandSsaOp::Cap { dst, cap_flag, .. } => {
                code.push(0xB8);
                code.extend_from_slice(&(*cap_flag as u32).to_le_bytes());
                code.extend_from_slice(&[0x23, 0x47, 0x04]);
                let disp = ((*dst) & 0x0F) * 4;
                code.extend_from_slice(&[0x89, 0x42, disp]);
            }
            NandSsaOp::Ld { dst, arena_offset } => {
                let disp_a = ((*arena_offset as u8) & 0x1F) * 4;
                let disp_r = ((*dst) & 0x0F) * 4;
                code.extend_from_slice(&[0x8B, 0x46, disp_a, 0x89, 0x42, disp_r]);
            }
            NandSsaOp::St { arena_offset, src } => {
                let disp_r = ((*src) & 0x0F) * 4;
                let disp_a = ((*arena_offset as u8) & 0x1F) * 4;
                code.extend_from_slice(&[0x8B, 0x42, disp_r, 0x89, 0x46, disp_a]);
            }
            NandSsaOp::Ret { src } => {
                let disp_r = ((*src) & 0x0F) * 4;
                code.extend_from_slice(&[0x8B, 0x42, disp_r, 0x5D, 0xC3]);
            }
            _ => {
                code.push(0x90);
            }
        }
    }

    let byte_len = code.len() - start_off;
    symbols.insert(
        module.name.clone(),
        EmittedFunctionSymbol {
            name: module.name.clone(),
            byte_offset: start_off,
            byte_len,
            safe_point_offsets: safe_points,
            tgemm_patch_sites: tgemm_sites,
        },
    );

    let digest = fnv1a_code_hash(&code);
    Ok(NativeCodegenArtifact {
        module_name: module.name.clone(),
        target_isa: JitTargetIsa::X86_64Avx512,
        symbols,
        machine_code: code,
        fnv1a_code_digest: digest,
        vpternlogd_count,
        vpmaddwd_or_smlal_count: vpmaddwd_count,
    })
}

// ============================================================================
// 3. ARM64 (AArch64 NEON) Deterministic Bytecode Emitter
// ============================================================================

#[inline]
pub const fn encode_aarch64_neon_rrr(base_opcode: u32, rd: u8, rn: u8, rm: u8) -> [u8; 4] {
    let word = base_opcode
        | (((rm & 0x1F) as u32) << 16)
        | (((rn & 0x1F) as u32) << 5)
        | ((rd & 0x1F) as u32);
    word.to_le_bytes()
}

/// Emits a deterministic ARM64 (AArch64) NEON machine-code image for `module`.
pub fn emit_aarch64_neon_module(module: &FelixPhoenixModule) -> Result<NativeCodegenArtifact, String> {
    let mut code = Vec::with_capacity(512);
    let mut symbols = BTreeMap::new();
    let mut neon_logic_count = 0usize;
    let mut smlal_count = 0usize;

    let start_off = 0;
    let mut safe_points = Vec::new();
    let mut tgemm_sites = Vec::new();

    for op in &module.ops {
        match op {
            NandSsaOp::Yield { .. } => {
                safe_points.push(code.len());
                code.extend_from_slice(&0xB940_0009u32.to_le_bytes());
            }
            NandSsaOp::Nand { dst, lhs, rhs } => {
                code.extend_from_slice(&encode_aarch64_neon_rrr(0x4EA0_1C00, *dst, *lhs, *rhs));
                code.extend_from_slice(&encode_aarch64_neon_rrr(0x4E20_1C00, dst.wrapping_add(1), lhs.wrapping_add(1), rhs.wrapping_add(1)));
                neon_logic_count += 2;
            }
            NandSsaOp::TGemm { dst, .. } => {
                tgemm_sites.push(code.len());
                let d = (*dst) & 0x1F;
                code.extend_from_slice(&encode_aarch64_neon_rrr(0x0E60_C000, d, 1, 2));
                code.extend_from_slice(&encode_aarch64_neon_rrr(0x0E60_8000, d, 3, 4));
                let addv_word = 0x4EB1_B800u32 | ((d as u32) << 5) | (d as u32);
                code.extend_from_slice(&addv_word.to_le_bytes());
                smlal_count += 2;
            }
            NandSsaOp::Ret { .. } => {
                code.extend_from_slice(&0xD65F_03C0u32.to_le_bytes());
            }
            _ => {
                code.extend_from_slice(&0xD503_201Fu32.to_le_bytes());
            }
        }
    }

    let byte_len = code.len() - start_off;
    symbols.insert(
        module.name.clone(),
        EmittedFunctionSymbol {
            name: module.name.clone(),
            byte_offset: start_off,
            byte_len,
            safe_point_offsets: safe_points,
            tgemm_patch_sites: tgemm_sites,
        },
    );

    let digest = fnv1a_code_hash(&code);
    Ok(NativeCodegenArtifact {
        module_name: module.name.clone(),
        target_isa: JitTargetIsa::AArch64Neon,
        symbols,
        machine_code: code,
        fnv1a_code_digest: digest,
        vpternlogd_count: neon_logic_count,
        vpmaddwd_or_smlal_count: smlal_count,
    })
}

// ============================================================================
// 4. W^X Executable JIT Page (`mmap` PROT_READ|PROT_WRITE -> `mprotect` PROT_READ|PROT_EXEC)
// ============================================================================

#[repr(C)]
pub struct JitRuntimeBridge {
    pub hotswap_pending: i32,
    pub granted_caps: u32,
    pub tgemm_result_slot: i32,
    pub exit_status: i32,
}

pub struct JitExecutablePage {
    ptr: *mut u8,
    map_size: usize,
    code_len: usize,
}

unsafe impl Send for JitExecutablePage {}
unsafe impl Sync for JitExecutablePage {}

impl JitExecutablePage {
    pub fn from_machine_code(machine_code: &[u8]) -> Result<Self, String> {
        if machine_code.is_empty() {
            return Err("Cannot allocate empty JIT page".into());
        }

        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }.max(4096);
        let map_size = ((machine_code.len() + page_size - 1) / page_size) * page_size;

        let ptr = unsafe {
            let mut flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
            #[cfg(target_os = "macos")]
            {
                flags |= libc::MAP_JIT;
            }

            let raw = libc::mmap(
                std::ptr::null_mut(),
                map_size,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1,
                0,
            );
            if raw == libc::MAP_FAILED {
                return Err("libc::mmap failed allocating JIT page".into());
            }

            #[cfg(target_os = "macos")]
            {
                extern "C" {
                    fn pthread_jit_write_protect_np(enabled: libc::c_int);
                }
                pthread_jit_write_protect_np(0);
            }

            std::ptr::copy_nonoverlapping(
                machine_code.as_ptr(),
                raw as *mut u8,
                machine_code.len(),
            );

            #[cfg(target_os = "macos")]
            {
                extern "C" {
                    fn pthread_jit_write_protect_np(enabled: libc::c_int);
                    fn sys_icache_invalidate(start: *const libc::c_void, len: libc::size_t);
                }
                pthread_jit_write_protect_np(1);
                sys_icache_invalidate(raw, machine_code.len());
            }

            if libc::mprotect(raw, map_size, libc::PROT_READ | libc::PROT_EXEC) != 0 {
                let _ = libc::munmap(raw, map_size);
                return Err("libc::mprotect(PROT_READ | PROT_EXEC) failed".into());
            }

            raw as *mut u8
        };

        Ok(Self {
            ptr,
            map_size,
            code_len: machine_code.len(),
        })
    }

    pub fn compile_host_native_trampoline(required_caps: CapEffectMask) -> Result<Self, String> {
        #[cfg(target_arch = "x86_64")]
        {
            let req = (required_caps as u32).to_le_bytes();
            let mut code: Vec<u8> = vec![
                0x8B, 0x07, 0x85, 0xC0, 0x74, 0x0A, 0xC7, 0x47, 0x0C, 0x01, 0x00, 0x00, 0x00,
                0x31, 0xC0, 0xC3, 0x8B, 0x47, 0x04, 0x25,
            ];
            code.extend_from_slice(&req);
            code.push(0x3D);
            code.extend_from_slice(&req);
            code.extend_from_slice(&[
                0x74, 0x0C, 0xC7, 0x47, 0x0C, 0xFF, 0xFF, 0xFF, 0xFF, 0xB8, 0xFF, 0xFF, 0xFF,
                0xFF, 0xC3, 0xC7, 0x47, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x8B, 0x47, 0x08, 0xC3,
            ]);
            return Self::from_machine_code(&code);
        }

        #[cfg(target_arch = "aarch64")]
        {
            let req_imm16 = (required_caps as u32) & 0xFFFF;
            let movz_w10 = 0x5280_000Au32 | (req_imm16 << 5);
            // 0: ldr w9, [x0, #0]
            // 1: cbz w9, +20 bytes (to insn 6: 5 words = imm19 = 5)
            // 2: mov w10, #1
            // 3: str w10, [x0, #12]
            // 4: mov w0, #0
            // 5: ret
            // 6: ldr w9, [x0, #4]
            // 7: movz w10, #(req & 0xFFFF)
            // 8: and w11, w9, w10
            // 9: cmp w11, w10
            // 10: b.eq +20 bytes (to insn 15: 5 words = imm19 = 5)
            // 11: movn w10, #0 (-1)
            // 12: str w10, [x0, #12]
            // 13: movn w0, #0 (-1)
            // 14: ret
            // 15: str wzr, [x0, #12]
            // 16: ldr w0, [x0, #8]
            // 17: ret
            let words: [u32; 18] = [
                0xB940_0009,
                0x3400_00A9,
                0x5280_002A,
                0xB900_0C0A,
                0x5280_0000,
                0xD65F_03C0,
                0xB940_0409,
                movz_w10,
                0x0A0A_012B,
                0x6B0A_017F,
                0x5400_00A0,
                0x1280_000A,
                0xB900_0C0A,
                0x1280_0000,
                0xD65F_03C0,
                0xB900_0C1F,
                0xB940_0800,
                0xD65F_03C0,
            ];
            let mut code = Vec::with_capacity(18 * 4);
            for w in words {
                code.extend_from_slice(&w.to_le_bytes());
            }
            return Self::from_machine_code(&code);
        }

        #[allow(unreachable_code)]
        Err("Unsupported host architecture for live JIT page execution".into())
    }

    pub fn execute_native(
        &self,
        bridge: &mut JitRuntimeBridge,
        acts: &[i16],
        trits: &[i8],
    ) -> i32 {
        let raw_dot = felix_ternary_gemm_i32(acts, trits);
        bridge.tgemm_result_slot = raw_dot;

        type NativeJitFn = unsafe extern "C" fn(*mut JitRuntimeBridge) -> i32;
        unsafe {
            let func: NativeJitFn = std::mem::transmute(self.ptr);
            func(bridge as *mut JitRuntimeBridge)
        }
    }

    pub fn code_len(&self) -> usize {
        self.code_len
    }
}

impl Drop for JitExecutablePage {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.map_size > 0 {
            unsafe {
                let _ = libc::munmap(self.ptr as *mut libc::c_void, self.map_size);
            }
        }
    }
}

#[inline]
pub fn fnv1a_code_hash(bytes: &[u8]) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for &b in bytes {
        h = (h ^ (b as u64)).wrapping_mul(0x0100_0000_01B3);
    }
    h
}

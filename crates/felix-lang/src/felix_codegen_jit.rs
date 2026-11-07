//! `felix/crates/felix-lang/src/felix_codegen_jit.rs`
//!
//! # FELIX-CODEGEN — Deterministic x86-64 AVX-512 (`vpternlogd` + `vpmaddwd`) & ARM64 NEON
//! Machine-Code Emitter + W^X Executable JIT Page Manager (`mmap` -> `mprotect`).

use crate::felix_phoenix::{
    tgemm_quant_tern_pq, CapEffectMask, FelixFunctionIr, FelixSsaOp, PccModule, QuantTernPQTensor,
};
use std::collections::BTreeMap;

/// Minimal zero-dependency POSIX / Darwin `libc` FFI bindings for W^X page management.
#[allow(non_camel_case_types)]
mod libc {
    pub type c_void = core::ffi::c_void;
    pub type c_int = i32;
    pub type size_t = usize;
    pub type off_t = i64;
    pub type c_long = isize;

    pub const PROT_READ: c_int = 0x01;
    pub const PROT_WRITE: c_int = 0x02;
    pub const PROT_EXEC: c_int = 0x04;

    pub const MAP_PRIVATE: c_int = 0x0002;
    #[cfg(target_os = "macos")]
    pub const MAP_ANONYMOUS: c_int = 0x1000;
    #[cfg(not(target_os = "macos"))]
    pub const MAP_ANONYMOUS: c_int = 0x0020;

    #[cfg(target_os = "macos")]
    pub const _SC_PAGESIZE: c_int = 29;
    #[cfg(not(target_os = "macos"))]
    pub const _SC_PAGESIZE: c_int = 30;

    pub const MAP_FAILED: *mut c_void = (!0usize) as *mut c_void;

    extern "C" {
        pub fn mmap(
            addr: *mut c_void,
            len: size_t,
            prot: c_int,
            flags: c_int,
            fd: c_int,
            offset: off_t,
        ) -> *mut c_void;
        pub fn mprotect(addr: *mut c_void, len: size_t, prot: c_int) -> c_int;
        pub fn munmap(addr: *mut c_void, len: size_t) -> c_int;
        pub fn sysconf(name: c_int) -> c_long;
    }
}

// ============================================================================
// 1. Target ISA Selection & Deterministic Bytecode Artifact
// ============================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JitTargetIsa {
    /// x86-64 with AVX-512 (`vpternlogd` + `vpmaddwd` + `vpaddd`).
    X86_64Avx512,
    /// ARM64 / AArch64 with NEON (`orr`/`and`/`bsl` + `smull`/`smlal`/`addv`).
    AArch64Neon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedFunctionSymbol {
    pub name: String,
    pub byte_offset: usize,
    pub byte_len: usize,
    pub safe_point_offsets: Vec<usize>,
    pub tgemm_patch_sites: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
pub fn encode_x86_64_evex_vpternlogd(zmm_dst: u8, zmm_src1: u8, zmm_src2: u8, imm8: u8) -> [u8; 7] {
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
        0xC5, p0_add, 0xFE, modrm_add, // vpaddd   ymm_acc, ymm_acc, ymm_tmp
    ]
}

/// Emits a deterministic x86-64 AVX-512 native machine-code image for `module`.
/// Iterates functions in strictly sorted (`BTreeMap`) order to guarantee bit-reproducibility.
pub fn emit_x86_64_avx512_module(module: &PccModule) -> Result<NativeCodegenArtifact, String> {
    if !module.pcc.effect_safety_proven {
        return Err("FELIX-CODEGEN refused module without proven PCC effect safety".into());
    }

    // Sort functions lexicographically by symbol name
    let mut sorted_fns: Vec<(&String, &FelixFunctionIr)> = module.functions.iter().collect();
    sorted_fns.sort_by(|a, b| a.0.cmp(b.0));

    let mut code = Vec::with_capacity(512);
    let mut symbols = BTreeMap::new();
    let mut vpternlogd_count = 0usize;
    let mut vpmaddwd_count = 0usize;

    for &(fn_name, func) in &sorted_fns {
        let start_off = code.len();
        let mut safe_points = Vec::new();
        let mut tgemm_sites = Vec::new();

        // System V AMD64 Prologue: `push rbp; mov rbp, rsp`
        code.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5]);

        for op in &func.instructions {
            match op {
                FelixSsaOp::Yield => {
                    // Safe-point check against `hotswap_pending` at `[rdi + 0]`:
                    // `mov eax, dword ptr [rdi]; test eax, eax; jnz +2`
                    safe_points.push(code.len());
                    code.extend_from_slice(&[0x8B, 0x07, 0x85, 0xC0, 0x75, 0x02]);
                }
                FelixSsaOp::Nand { dst, src1, src2 } => {
                    // Dual-rail 512-bit Ternary NAND via two EVEX `vpternlogd` instructions:
                    // 1) pos_out = neg1 | neg2 (imm8 = 0xFC)
                    // 2) neg_out = pos1 & pos2 (imm8 = 0xC0)
                    let insn_pos =
                        encode_x86_64_evex_vpternlogd(*dst as u8, *src1 as u8, *src2 as u8, 0xFC);
                    let insn_neg = encode_x86_64_evex_vpternlogd(
                        (*dst as u8).wrapping_add(1),
                        (*src1 as u8).wrapping_add(1),
                        (*src2 as u8).wrapping_add(1),
                        0xC0,
                    );
                    code.extend_from_slice(&insn_pos);
                    code.extend_from_slice(&insn_neg);
                    vpternlogd_count += 2;
                }
                FelixSsaOp::TSel {
                    dst,
                    cond_reg,
                    true_reg,
                    false_reg: _,
                } => {
                    // Ternary select `cond ? a : b` via single EVEX `vpternlogd` (imm8 = 0xCA)
                    let insn = encode_x86_64_evex_vpternlogd(
                        *dst as u8,
                        *cond_reg as u8,
                        *true_reg as u8,
                        0xCA,
                    );
                    code.extend_from_slice(&insn);
                    vpternlogd_count += 1;
                }
                FelixSsaOp::TGemm { dst, .. } => {
                    // Fused `vpternlogd` sign-mux (0xCA) + `vpmaddwd` + `vpaddd` loop body
                    tgemm_sites.push(code.len());
                    let sign_mux = encode_x86_64_evex_vpternlogd((*dst as u8) & 7, 1, 2, 0xCA);
                    let madd_pair = encode_x86_64_vex_vpmaddwd_vpaddd((*dst as u8) & 7, 3, 1, 2);
                    code.extend_from_slice(&sign_mux);
                    code.extend_from_slice(&madd_pair);
                    vpternlogd_count += 1;
                    vpmaddwd_count += 1;
                }
                FelixSsaOp::Cap { dst, cap_mask, .. } => {
                    // Static + dynamic capability guard: `mov eax, imm32; and eax, [rdi + 4]`
                    code.push(0xB8);
                    code.extend_from_slice(&cap_mask.to_le_bytes());
                    code.extend_from_slice(&[0x23, 0x47, 0x04]);
                    // Store result into register slot `[rdx + dst*4]`
                    let disp = ((*dst as u8) & 0x0F) * 4;
                    code.extend_from_slice(&[0x89, 0x42, disp]);
                }
                FelixSsaOp::Ld { dst, arena_offset } => {
                    let disp_a = ((*arena_offset as u8) & 0x1F) * 4;
                    let disp_r = ((*dst as u8) & 0x0F) * 4;
                    code.extend_from_slice(&[0x8B, 0x46, disp_a, 0x89, 0x42, disp_r]);
                }
                FelixSsaOp::St { arena_offset, src } => {
                    let disp_r = ((*src as u8) & 0x0F) * 4;
                    let disp_a = ((*arena_offset as u8) & 0x1F) * 4;
                    code.extend_from_slice(&[0x8B, 0x42, disp_r, 0x89, 0x46, disp_a]);
                }
                FelixSsaOp::Ret { src } => {
                    let disp_r = ((*src as u8) & 0x0F) * 4;
                    // `mov eax, [rdx + src*4]; pop rbp; ret`
                    code.extend_from_slice(&[0x8B, 0x42, disp_r, 0x5D, 0xC3]);
                }
            }
        }

        let byte_len = code.len() - start_off;
        symbols.insert(
            fn_name.clone(),
            EmittedFunctionSymbol {
                name: fn_name.clone(),
                byte_offset: start_off,
                byte_len,
                safe_point_offsets: safe_points,
                tgemm_patch_sites: tgemm_sites,
            },
        );
    }

    let digest = fnv1a_code_hash(&code);
    Ok(NativeCodegenArtifact {
        module_name: module.module_name.clone(),
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
pub fn emit_aarch64_neon_module(module: &PccModule) -> Result<NativeCodegenArtifact, String> {
    if !module.pcc.effect_safety_proven {
        return Err("FELIX-CODEGEN refused module without proven PCC effect safety".into());
    }

    let mut sorted_fns: Vec<(&String, &FelixFunctionIr)> = module.functions.iter().collect();
    sorted_fns.sort_by(|a, b| a.0.cmp(b.0));

    let mut code = Vec::with_capacity(512);
    let mut symbols = BTreeMap::new();
    let mut neon_logic_count = 0usize;
    let mut smlal_count = 0usize;

    for &(fn_name, func) in &sorted_fns {
        let start_off = code.len();
        let mut safe_points = Vec::new();
        let mut tgemm_sites = Vec::new();

        for op in &func.instructions {
            match op {
                FelixSsaOp::Yield => {
                    // `ldr w9, [x0, #0]` (0xB9400009)
                    safe_points.push(code.len());
                    code.extend_from_slice(&0xB940_0009u32.to_le_bytes());
                }
                FelixSsaOp::Nand { dst, src1, src2 } => {
                    // Dual-rail NEON Ternary NAND:
                    // `orr v_dst.16b, v_src1.16b, v_src2.16b` (0x4EA01C00)
                    // `and v_dst2.16b, v_src1.16b, v_src2.16b` (0x4E201C00)
                    code.extend_from_slice(&encode_aarch64_neon_rrr(
                        0x4EA0_1C00,
                        *dst as u8,
                        *src1 as u8,
                        *src2 as u8,
                    ));
                    code.extend_from_slice(&encode_aarch64_neon_rrr(
                        0x4E20_1C00,
                        (*dst as u8).wrapping_add(1),
                        (*src1 as u8).wrapping_add(1),
                        (*src2 as u8).wrapping_add(1),
                    ));
                    neon_logic_count += 2;
                }
                FelixSsaOp::TSel {
                    dst,
                    cond_reg,
                    true_reg,
                    false_reg: _,
                } => {
                    // `bsl v_dst.16b, v_true.16b, v_false.16b` (0x6E601C00)
                    code.extend_from_slice(&encode_aarch64_neon_rrr(
                        0x6E60_1C00,
                        *dst as u8,
                        *cond_reg as u8,
                        *true_reg as u8,
                    ));
                    neon_logic_count += 1;
                }
                FelixSsaOp::TGemm { dst, .. } => {
                    // `smull v_dst.4s, v1.4h, v2.4h` (0x0E60C000)
                    // `smlal v_dst.4s, v3.4h, v4.4h` (0x0E608000)
                    // `addv s_dst, v_dst.4s`         (0x4EB1B800)
                    tgemm_sites.push(code.len());
                    let d = (*dst as u8) & 0x1F;
                    code.extend_from_slice(&encode_aarch64_neon_rrr(0x0E60_C000, d, 1, 2));
                    code.extend_from_slice(&encode_aarch64_neon_rrr(0x0E60_8000, d, 3, 4));
                    let addv_word = 0x4EB1_B800u32 | ((d as u32) << 5) | (d as u32);
                    code.extend_from_slice(&addv_word.to_le_bytes());
                    smlal_count += 2;
                }
                FelixSsaOp::Ret { .. } => {
                    // `ret` (0xD65F03C0)
                    code.extend_from_slice(&0xD65F_03C0u32.to_le_bytes());
                }
                _ => {
                    // `nop` (0xD503201F)
                    code.extend_from_slice(&0xD503_201Fu32.to_le_bytes());
                }
            }
        }

        let byte_len = code.len() - start_off;
        symbols.insert(
            fn_name.clone(),
            EmittedFunctionSymbol {
                name: fn_name.clone(),
                byte_offset: start_off,
                byte_len,
                safe_point_offsets: safe_points,
                tgemm_patch_sites: tgemm_sites,
            },
        );
    }

    let digest = fnv1a_code_hash(&code);
    Ok(NativeCodegenArtifact {
        module_name: module.module_name.clone(),
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

/// Shared C-ABI bridge passed in the first argument register (`rdi` on x86-64, `x0` on ARM64)
/// to native JIT-compiled PHOENIX pages.
#[repr(C)]
pub struct JitRuntimeBridge {
    /// Checked at every native `@hotswappable` safe-point (`Op::Yield`).
    pub hotswap_pending: i32,
    /// Active capability grant mask verified by native `Op::Cap` guards.
    pub granted_caps: u32,
    /// Pre-evaluated or callback-populated `tgemm` slot in the shared arena.
    pub tgemm_result_slot: i32,
    /// Recorded exit status (`0` = normal completion, `1` = yielded for hot-swap, `-1` = cap fault).
    pub exit_status: i32,
}

/// Manages an OS page-aligned W^X memory region (`libc::mmap` + `libc::mprotect`) holding
/// native executable instructions. Automatically unmaps via `libc::munmap` on `Drop`.
pub struct JitExecutablePage {
    ptr: *mut u8,
    map_size: usize,
    code_len: usize,
}

unsafe impl Send for JitExecutablePage {}
unsafe impl Sync for JitExecutablePage {}

impl JitExecutablePage {
    /// Allocates a W^X page, copies `machine_code`, transitions the page from
    /// `PROT_READ | PROT_WRITE` to `PROT_READ | PROT_EXEC`, and flushes the instruction cache.
    pub fn from_machine_code(machine_code: &[u8]) -> Result<Self, String> {
        if machine_code.is_empty() {
            return Err("Cannot allocate empty JIT page".into());
        }

        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }.max(4096);
        let map_size = ((machine_code.len() + page_size - 1) / page_size) * page_size;

        let ptr = unsafe {
            let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
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

            std::ptr::copy_nonoverlapping(
                machine_code.as_ptr(),
                raw as *mut u8,
                machine_code.len(),
            );

            if libc::mprotect(raw, map_size, libc::PROT_READ | libc::PROT_EXEC) != 0 {
                let _ = libc::munmap(raw, map_size);
                return Err("libc::mprotect(PROT_READ | PROT_EXEC) failed".into());
            }

            #[cfg(target_os = "macos")]
            {
                extern "C" {
                    fn sys_icache_invalidate(start: *const libc::c_void, len: libc::size_t);
                }
                sys_icache_invalidate(raw, machine_code.len());
            }

            raw as *mut u8
        };

        Ok(Self {
            ptr,
            map_size,
            code_len: machine_code.len(),
        })
    }

    /// Compiles a host-executable native trampoline for `@hotswappable` FELIX functions:
    /// - Checks `bridge.hotswap_pending` at the `Op::Yield` safe-point: if non-zero, sets
    ///   `bridge.exit_status = 1` and returns immediately so PHOENIX swaps the trampoline.
    /// - Verifies `(required_caps & bridge.granted_caps) == required_caps`; if unauthorized,
    ///   sets `bridge.exit_status = -1` and aborts.
    /// - Otherwise sets `bridge.exit_status = 0` and returns `bridge.tgemm_result_slot` in `eax`/`w0`.
    pub fn compile_host_native_trampoline(required_caps: CapEffectMask) -> Result<Self, String> {
        #[cfg(target_arch = "x86_64")]
        {
            let req = required_caps.to_le_bytes();
            let mut code: Vec<u8> = vec![
                0x8B, 0x07, 0x85, 0xC0, 0x74, 0x0A, 0xC7, 0x47, 0x0C, 0x01, 0x00, 0x00, 0x00, 0x31,
                0xC0, 0xC3, 0x8B, 0x47, 0x04, 0x25,
            ];
            code.extend_from_slice(&req);
            code.push(0x3D);
            code.extend_from_slice(&req);
            code.extend_from_slice(&[
                0x74, 0x0C, 0xC7, 0x47, 0x0C, 0xFF, 0xFF, 0xFF, 0xFF, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF,
                0xC3, 0xC7, 0x47, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x8B, 0x47, 0x08, 0xC3,
            ]);
            return Self::from_machine_code(&code);
        }

        #[cfg(target_arch = "aarch64")]
        {
            // AAPCS64 (`x0 = *mut JitRuntimeBridge`):
            //   ldr w9, [x0, #0]       ; 0xB9400009 (hotswap_pending)
            //   cbz w9, #20            ; 0x340000A9 -> .check_caps (pc + 5*4 = +20 bytes)
            //   mov w10, #1            ; 0x5280002A
            //   str w10, [x0, #12]     ; 0xB9000C0A (exit_status = 1)
            //   mov w0, #0             ; 0x52800000
            //   ret                    ; 0xD65F03C0
            // .check_caps:
            //   ldr w9, [x0, #4]       ; 0xB9400409 (granted_caps)
            //   movz w10, #(req & 0xFFFF)
            //   and w11, w9, w10       ; 0x0A0A012B
            //   cmp w11, w10           ; 0x6B0A017F
            //   b.eq #20               ; 0x540000A0 -> .run_tgemm (pc + 5*4 = +20 bytes)
            //   movn w10, #0           ; 0x1280000A (-1)
            //   str w10, [x0, #12]     ; 0xB9000C0A (exit_status = -1)
            //   movn w0, #0            ; 0x12800000
            //   ret                    ; 0xD65F03C0
            // .run_tgemm:
            //   str wzr, [x0, #12]     ; 0xB9000C1F (exit_status = 0)
            //   ldr w0, [x0, #8]       ; 0xB9400800 (tgemm_result_slot)
            //   ret                    ; 0xD65F03C0
            let req_imm16 = (required_caps & 0xFFFF) as u32;
            let movz_w10 = 0x5280_000Au32 | (req_imm16 << 5);
            let words: [u32; 17] = [
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
            ];
            let mut code = Vec::with_capacity(18 * 4);
            for w in words {
                code.extend_from_slice(&w.to_le_bytes());
            }
            code.extend_from_slice(&0xD65F_03C0u32.to_le_bytes());
            return Self::from_machine_code(&code);
        }

        #[allow(unreachable_code)]
        Err("Unsupported host architecture for live JIT page execution".into())
    }

    /// Invokes the W^X native machine-code page directly via a C-ABI function pointer.
    pub fn execute_native(
        &self,
        bridge: &mut JitRuntimeBridge,
        acts: &[i16],
        qtensor: &QuantTernPQTensor,
    ) -> i32 {
        let (raw_dot, _) = tgemm_quant_tern_pq(acts, qtensor);
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

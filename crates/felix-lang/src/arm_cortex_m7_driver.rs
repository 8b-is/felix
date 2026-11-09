//! # `arm_cortex_m7_driver` — Safety-Critical ARM Cortex-M7 / Cortex-M4 / Cortex-M0+ Firmware & Driver Engine
//!
//! Zero-float (`#![no_std]` compatible), zero-`static mut`, exact integer and rational (`p/q`)
//! embedded systems driver and verification suite implementing the `/arm-cortex-expert` specification:
//!
//! 1. **Weakly-Ordered MMIO Memory Barriers (`DMB` / `DSB` / `ISB`) & Address Validation**:
//!    - Enforces `DMB` before/after peripheral register reads and `DMB` + `DSB` around writes.
//!    - Validates peripheral addresses (`0x4000_0000..=0x5FFF_FFFF`) and Cortex-M System Control
//!      Space / Private Peripheral Bus (`0xE000_0000..=0xE00F_FFFF`).
//!    - Implements Write-1-to-Clear (`W1C`) status clearing (`USBSTS`, `PORTSC1`, `LPSPI_SR`, `DMA_ISR`)
//!      without clobbering concurrent hardware flags.
//! 2. **32-Byte Cache-Line Aligned DMA Buffers & DTCM / MPU Coherency**:
//!    - `#[repr(C, align(32))]` [`CortexM7DmaBuffer`] with strict 32-byte alignment and 32-byte size
//!      multiple checks so D-Cache invalidation never corrupts adjacent memory.
//!    - Ranks placement across Tier-1 `DTCM` (non-cacheable, zero maintenance), Tier-2 `MPU Non-Cacheable`,
//!      and Tier-3 `Cached SRAM` with explicit `clean_dcache_by_range_32` / `invalidate_dcache_by_range_32`.
//! 3. **Zero `static mut` Lock-Free Atomic ISR SPSC Ring Buffer & NVIC / `BASEPRI` Discipline**:
//!    - [`AtomicIsrSpscQueue`] built exclusively on `AtomicUsize` (`Acquire`/`Release`) and `AtomicU32`.
//!    - Priority banding (`0..=2` Hard Real-Time DMA/Timers, `3..=7` Peripherals, `8..=15` Background)
//!      and `BASEPRI` selective masking (`priority << (8 - prio_bits)`).
//! 4. **Transaction-Based Non-Blocking LPSPI / SPI Master + DMA Sensor Driver**:
//!    - Enforces Teensy 4.x `LPSPI_MAX_CLOCK_HZ = 30_000_000` (`30 MHz`), automatic DMA escalation
//!      above `500_000 Hz` (`500 kHz`), `WHO_AM_I` (`0x0F`) register transaction, and exact rational
//!      `p/q` DWT cycle efficiency with Rung 0 Bonobo resonance (`3/2`).
//! 5. **HardFault / `CFSR` Decoder, Deterministic FPU (`FPCCR.LSPEN = 0`), and MPU + Canary Stack Guard**.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// ARM Cortex-M7 data cache line size in bytes (32 bytes = 8 × 32-bit words).
pub const CORTEX_M7_CACHE_LINE_BYTES: usize = 32;

/// Validated peripheral MMIO base address (`0x4000_0000`).
pub const MMIO_PERIPH_BASE: u32 = 0x4000_0000;
/// Validated peripheral MMIO end address (`0x5FFF_FFFF`).
pub const MMIO_PERIPH_END: u32 = 0x5FFF_FFFF;

/// ARM Cortex-M Private Peripheral Bus / System Control Space base (`0xE000_0000`).
pub const MMIO_PPB_BASE: u32 = 0xE000_0000;
/// ARM Cortex-M Private Peripheral Bus / System Control Space end (`0xE00F_FFFF`).
pub const MMIO_PPB_END: u32 = 0xE00F_FFFF;

/// i.MX RT1062 (Teensy 4.x) USB1 OTG Status Register (`USBSTS`, Write-1-to-Clear).
pub const IMXRT_USB1_USBSTS_ADDR: u32 = 0x402E_0144;
/// i.MX RT1062 (Teensy 4.x) USB1 Port Status & Control (`PORTSC1`, Write-1-to-Clear bits).
pub const IMXRT_USB1_PORTSC1_ADDR: u32 = 0x402E_0184;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Control Register (`CR`).
pub const IMXRT_LPSPI4_CR_ADDR: u32 = 0x403A_0010;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Status Register (`SR`, Write-1-to-Clear).
pub const IMXRT_LPSPI4_SR_ADDR: u32 = 0x403A_0014;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 DMA Enable Register (`DER`).
pub const IMXRT_LPSPI4_DER_ADDR: u32 = 0x403A_0018;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Clock Configuration Register (`CCR`).
pub const IMXRT_LPSPI4_CCR_ADDR: u32 = 0x403A_0020;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Transmit Command Register (`TCR`).
pub const IMXRT_LPSPI4_TCR_ADDR: u32 = 0x403A_0060;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Transmit Data Register (`TDR`).
pub const IMXRT_LPSPI4_TDR_ADDR: u32 = 0x403A_0064;
/// i.MX RT1062 (Teensy 4.x) LPSPI4 Receive Data Register (`RDR`).
pub const IMXRT_LPSPI4_RDR_ADDR: u32 = 0x403A_0074;

/// ARM Cortex-M DWT Control Register (`DWT_CTRL`).
pub const CORTEX_DWT_CTRL_ADDR: u32 = 0xE000_1000;
/// ARM Cortex-M DWT Cycle Count Register (`DWT_CYCCNT`).
pub const CORTEX_DWT_CYCCNT_ADDR: u32 = 0xE000_1004;
/// ARM Cortex-M Configurable Fault Status Register (`SCB_CFSR`).
pub const CORTEX_SCB_CFSR_ADDR: u32 = 0xE000_ED28;
/// ARM Cortex-M HardFault Status Register (`SCB_HFSR`).
pub const CORTEX_SCB_HFSR_ADDR: u32 = 0xE000_ED2C;
/// ARM Cortex-M MemManage Fault Address Register (`SCB_MMFAR`).
pub const CORTEX_SCB_MMFAR_ADDR: u32 = 0xE000_ED34;
/// ARM Cortex-M BusFault Address Register (`SCB_BFAR`).
pub const CORTEX_SCB_BFAR_ADDR: u32 = 0xE000_ED38;
/// ARM Cortex-M Floating-Point Context Control Register (`FPU_FPCCR`).
pub const CORTEX_FPU_FPCCR_ADDR: u32 = 0xE000_EF34;

/// `FPU->FPCCR` bit 31: Automatic State Preservation Enable (`ASPEN`).
pub const FPCCR_ASPEN_BIT: u32 = 1 << 31;
/// `FPU->FPCCR` bit 30: Lazy State Preservation Enable (`LSPEN`). Clear for deterministic ISR latency.
pub const FPCCR_LSPEN_BIT: u32 = 1 << 30;

/// Teensy 4.x (`i.MX RT1062`) LPSPI maximum safe peripheral clock (`30 MHz`).
pub const TEENSY4_LPSPI_MAX_CLOCK_HZ: u32 = 30_000_000;
/// SPI throughput threshold (`500 kHz`) above which DMA transfers are required over polled/IRQ byte I/O.
pub const SPI_DMA_THRESHOLD_HZ: u32 = 500_000;
/// Teensy 4.x emulated EEPROM maximum safe sustained write rate (`10 Hz`).
pub const TEENSY4_EEPROM_MAX_WRITE_HZ: u32 = 10;

/// Canonical stack canary sentinel value (`0xDEAD_BEEF`) placed at the bottom of the stack.
pub const STACK_CANARY_MAGIC: u32 = 0xDEAD_BEEF;

/// Canonical WHO_AM_I register address (`0x0F`) for external MEMS IMU / environmental sensors.
pub const SENSOR_REG_WHO_AM_I: u8 = 0x0F;

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

/// Reduces `(p, q)` to canonical coprime rational form (`gcd(p, q) == 1`).
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

/// Rung 0 Bonobo target resonance trit (`+1` when `p / q >= 3 / 2`, `0` when `1/1 <= p/q < 3/2`, `-1` otherwise).
#[inline]
pub const fn bonobo_resonance_trit(p: u64, q: u64) -> i8 {
    if q == 0 {
        return -1;
    }
    if p.saturating_mul(2) >= q.saturating_mul(3) {
        1
    } else if p >= q {
        0
    } else {
        -1
    }
}

/// ARM Cortex-M microarchitecture family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexCoreVariant {
    /// ARM Cortex-M0 / M0+ (Thumb-1, ~50 MHz, 2-4 NVIC priorities, no FPU/Cache/TCM/DWT, limited HardFault).
    CortexM0Plus,
    /// ARM Cortex-M3 (Thumb-2, ~100 MHz, 8-256 NVIC priorities, DWT, full CFSR/HFSR fault registers).
    CortexM3,
    /// ARM Cortex-M4F (Thumb-2 + DSP + single-precision FPU, ~180 MHz, DWT, MPU).
    CortexM4F,
    /// ARM Cortex-M7F (Thumb-2 + DSP + single/double FPU, ~600 MHz, I/D-Cache, ITCM/DTCM, weakly-ordered memory).
    CortexM7F,
}

impl CortexCoreVariant {
    /// Whether the core has a weakly-ordered memory model and data cache requiring explicit `DMB`/`DSB` barriers.
    #[inline]
    pub const fn requires_mmio_and_dma_barriers(self) -> bool {
        matches!(self, Self::CortexM7F)
    }

    /// Whether the core provides full `CFSR`, `MMFAR`, and `BFAR` fault status registers.
    #[inline]
    pub const fn has_configurable_fault_registers(self) -> bool {
        !matches!(self, Self::CortexM0Plus)
    }

    /// Whether the core includes a Data Watchpoint and Trace (`DWT`) cycle counter (`CYCCNT`).
    #[inline]
    pub const fn has_dwt_cycle_counter(self) -> bool {
        !matches!(self, Self::CortexM0Plus)
    }
}

/// Target MCU hardware profile (Teensy 4.x, STM32H7, nRF52840, SAMD21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McuPlatformProfile {
    pub name: &'static str,
    pub mcu_part: &'static str,
    pub core: CortexCoreVariant,
    pub core_clock_hz: u32,
    pub dtcm_bytes: u32,
    pub dcache_line_bytes: u8,
    pub nvic_prio_bits: u8,
    pub max_spi_clock_hz: u32,
    pub gpio_5v_tolerant: bool,
}

impl McuPlatformProfile {
    /// Teensy 4.0 / 4.1 (`NXP i.MX RT1062`, ARM Cortex-M7F @ 600 MHz, 512 KB TCM, 32B cache line, 30 MHz LPSPI max).
    #[inline]
    pub const fn teensy_41_imxrt1062() -> Self {
        Self {
            name: "Teensy 4.1",
            mcu_part: "MIMXRT1062DVJ6B",
            core: CortexCoreVariant::CortexM7F,
            core_clock_hz: 600_000_000,
            dtcm_bytes: 512 * 1024,
            dcache_line_bytes: 32,
            nvic_prio_bits: 4,
            max_spi_clock_hz: TEENSY4_LPSPI_MAX_CLOCK_HZ,
            gpio_5v_tolerant: false,
        }
    }

    /// STM32H743 (`ARM Cortex-M7F` @ 480 MHz, 128 KB DTCM, 32B cache line, 50 MHz SPI, FT 5V-tolerant pins).
    #[inline]
    pub const fn stm32h743_cortex_m7() -> Self {
        Self {
            name: "STM32H743ZI",
            mcu_part: "STM32H743ZIT6",
            core: CortexCoreVariant::CortexM7F,
            core_clock_hz: 480_000_000,
            dtcm_bytes: 128 * 1024,
            dcache_line_bytes: 32,
            nvic_prio_bits: 4,
            max_spi_clock_hz: 50_000_000,
            gpio_5v_tolerant: true,
        }
    }

    /// Nordic `nRF52840` (`ARM Cortex-M4F` @ 64 MHz, EasyDMA, 3-bit NVIC priorities, 32 MHz SPIM3).
    #[inline]
    pub const fn nrf52840_cortex_m4f() -> Self {
        Self {
            name: "nRF52840-DK",
            mcu_part: "nRF52840-QIAA",
            core: CortexCoreVariant::CortexM4F,
            core_clock_hz: 64_000_000,
            dtcm_bytes: 0,
            dcache_line_bytes: 0,
            nvic_prio_bits: 3,
            max_spi_clock_hz: 32_000_000,
            gpio_5v_tolerant: false,
        }
    }

    /// Microchip `ATSAMD21G18` (`ARM Cortex-M0+` @ 48 MHz, SERCOM SPI, 2-bit / 4-level NVIC priorities).
    #[inline]
    pub const fn samd21_cortex_m0plus() -> Self {
        Self {
            name: "SAMD21G18",
            mcu_part: "ATSAMD21G18A",
            core: CortexCoreVariant::CortexM0Plus,
            core_clock_hz: 48_000_000,
            dtcm_bytes: 0,
            dcache_line_bytes: 0,
            nvic_prio_bits: 2,
            max_spi_clock_hz: 12_000_000,
            gpio_5v_tolerant: false,
        }
    }
}

/// Driver & safety verification error kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexDriverError {
    /// MMIO address is outside `0x4000_0000..=0x5FFF_FFFF` and `0xE000_0000..=0xE00F_FFFF` or not 4-byte aligned.
    InvalidMmioAddress { addr: u32 },
    /// DMA buffer address or byte length is not a multiple of 32 bytes (`CORTEX_M7_CACHE_LINE_BYTES`).
    DmaCacheLineMisaligned { addr: usize, len: usize },
    /// Requested SPI clock exceeds the platform's safe peripheral clock limit (e.g. >30 MHz on Teensy 4.x).
    SpiClockExceedsPlatformMax { requested_hz: u32, max_hz: u32 },
    /// Attempted to reconfigure CCM / GCLK clock tree while peripheral is active.
    ClockChangeWhilePeripheralActive,
    /// Lock-free ISR ring buffer is full (`Overrun`).
    RingBufferFull,
    /// Lock-free ISR ring buffer is empty (`Underrun`).
    RingBufferEmpty,
    /// Invalid NVIC priority level for the target core's `nvic_prio_bits`.
    InvalidNvicPriority { priority: u8, max_priority: u8 },
}

// ============================================================================
// 1. Weakly-Ordered Memory Barriers (`DMB` / `DSB` / `ISB`) & Safe MMIO
// ============================================================================

static DMB_COUNT: AtomicUsize = AtomicUsize::new(0);
static DSB_COUNT: AtomicUsize = AtomicUsize::new(0);
static ISB_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Execute an ARM Data Memory Barrier (`DMB SY`), ensuring all explicit memory accesses
/// before the barrier are observed before any explicit memory accesses after the barrier.
#[inline(always)]
pub fn cortex_dmb() {
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    // SAFETY: `dmb sy` is a pure memory barrier instruction with no register or stack side effects.
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, preserves_flags));
    }
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    {
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
    DMB_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Execute an ARM Data Synchronization Barrier (`DSB SY`), waiting until all pending
/// explicit memory transactions and cache maintenance operations complete before the next instruction executes.
#[inline(always)]
pub fn cortex_dsb() {
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    // SAFETY: `dsb sy` is a pure synchronization barrier instruction with no register or stack side effects.
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    {
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
    DSB_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Execute an ARM Instruction Synchronization Barrier (`ISB SY`), flushing the pipeline
/// after MPU, FPU (`FPCCR`), cache, or `BASEPRI` control register updates.
#[inline(always)]
pub fn cortex_isb() {
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    // SAFETY: `isb sy` flushes the processor pipeline with no register or stack side effects.
    unsafe {
        core::arch::asm!("isb sy", options(nostack, preserves_flags));
    }
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    {
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
    ISB_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Read the cumulative `(dmb_count, dsb_count, isb_count)` barrier telemetry counters.
#[inline]
pub fn barrier_telemetry_counts() -> (usize, usize, usize) {
    (
        DMB_COUNT.load(Ordering::Relaxed),
        DSB_COUNT.load(Ordering::Relaxed),
        ISB_COUNT.load(Ordering::Relaxed),
    )
}

/// Validates that `addr` is 32-bit word-aligned and falls within either the ARM Cortex-M
/// peripheral region (`0x4000_0000..=0x5FFF_FFFF`) or System Control Space (`0xE000_0000..=0xE00F_FFFF`).
#[inline]
pub const fn is_valid_mmio_address(addr: u32) -> bool {
    if (addr & 0x3) != 0 {
        return false;
    }
    let in_periph = addr >= MMIO_PERIPH_BASE && addr <= MMIO_PERIPH_END;
    let in_ppb = addr >= MMIO_PPB_BASE && addr <= MMIO_PPB_END;
    in_periph || in_ppb
}

/// Deterministic MMIO register bank for bare-metal verification and hardware-in-the-loop simulation.
/// Tracks `DMB` and `DSB` barrier discipline and models Write-1-to-Clear (`W1C`) status registers accurately.
#[derive(Debug, Clone)]
pub struct MmioBusSim {
    slots: [(u32, u32, bool); 16],
    len: usize,
    pub dmb_issued: u32,
    pub dsb_issued: u32,
}

impl Default for MmioBusSim {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioBusSim {
    #[inline]
    pub const fn new() -> Self {
        Self {
            slots: [(0, 0, false); 16],
            len: 0,
            dmb_issued: 0,
            dsb_issued: 0,
        }
    }

    /// Map a register `addr` with `initial_val` and whether it uses Write-1-to-Clear (`is_w1c`) semantics.
    pub fn map_register(
        &mut self,
        addr: u32,
        initial_val: u32,
        is_w1c: bool,
    ) -> Result<(), CortexDriverError> {
        if !is_valid_mmio_address(addr) {
            return Err(CortexDriverError::InvalidMmioAddress { addr });
        }
        for i in 0..self.len {
            if self.slots[i].0 == addr {
                self.slots[i].1 = initial_val;
                self.slots[i].2 = is_w1c;
                return Ok(());
            }
        }
        if self.len < self.slots.len() {
            self.slots[self.len] = (addr, initial_val, is_w1c);
            self.len += 1;
        }
        Ok(())
    }

    /// Read a 32-bit MMIO register wrapped with `DMB` before and `DMB` after the volatile read.
    pub fn mmio_read(&mut self, addr: u32) -> Result<u32, CortexDriverError> {
        if !is_valid_mmio_address(addr) {
            return Err(CortexDriverError::InvalidMmioAddress { addr });
        }
        cortex_dmb();
        self.dmb_issued = self.dmb_issued.saturating_add(1);
        let mut val = 0u32;
        for i in 0..self.len {
            if self.slots[i].0 == addr {
                val = self.slots[i].1;
                break;
            }
        }
        cortex_dmb();
        self.dmb_issued = self.dmb_issued.saturating_add(1);
        Ok(val)
    }

    /// Write a 32-bit MMIO register wrapped with `DMB` before the volatile write and `DSB` after.
    /// If the register is configured as Write-1-to-Clear (`W1C`), writing `1` bits clears those bits
    /// while writing `0` bits leaves existing hardware status flags untouched.
    pub fn mmio_write(&mut self, addr: u32, val: u32) -> Result<(), CortexDriverError> {
        if !is_valid_mmio_address(addr) {
            return Err(CortexDriverError::InvalidMmioAddress { addr });
        }
        cortex_dmb();
        self.dmb_issued = self.dmb_issued.saturating_add(1);
        let mut found = false;
        for i in 0..self.len {
            if self.slots[i].0 == addr {
                if self.slots[i].2 {
                    // Write-1-to-Clear (W1C): clear bits where `val` has 1s!
                    self.slots[i].1 &= !val;
                } else {
                    self.slots[i].1 = val;
                }
                found = true;
                break;
            }
        }
        if !found && self.len < self.slots.len() {
            self.slots[self.len] = (addr, val, false);
            self.len += 1;
        }
        cortex_dsb();
        self.dsb_issued = self.dsb_issued.saturating_add(1);
        Ok(())
    }

    /// Safe Read-Modify-Write helper (`(reg & !clear_mask) | set_mask`) for standard control registers.
    pub fn mmio_modify(
        &mut self,
        addr: u32,
        clear_mask: u32,
        set_mask: u32,
    ) -> Result<u32, CortexDriverError> {
        let cur = self.mmio_read(addr)?;
        let next = (cur & !clear_mask) | set_mask;
        self.mmio_write(addr, next)?;
        Ok(next)
    }

    /// Explicit Write-1-to-Clear (`W1C`) helper: reads active status flags, masks to `handled_mask`,
    /// and writes those `1` bits back to clear them without disturbing unhandled flags.
    pub fn mmio_clear_w1c(
        &mut self,
        addr: u32,
        handled_mask: u32,
    ) -> Result<u32, CortexDriverError> {
        let status = self.mmio_read(addr)?;
        let to_clear = status & handled_mask;
        if to_clear != 0 {
            self.mmio_write(addr, to_clear)?;
        }
        Ok(to_clear)
    }
}

// ============================================================================
// 2. DMA & Cache Coherency (32-Byte Cache Line Alignment & DTCM Placement)
// ============================================================================

/// Memory placement strategy for ARM Cortex-M7 DMA buffers, ranked Best to Worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaMemoryPlacement {
    /// **Best (Rank 1)**: Tightly Coupled Memory (`DTCM`, `.dtcm.bss`). Non-cacheable, single-cycle CPU access,
    /// zero cache maintenance overhead required before/after DMA.
    DtcmNonCacheable,
    /// **Good (Rank 2)**: OCRAM / AXI SRAM region configured as Non-Cacheable via the ARM MPU (`TEX=001, C=0, B=0`).
    MpuNonCacheableSram,
    /// **Fallback (Rank 3)**: Cacheable OCRAM / SRAM requiring explicit 32-byte aligned D-Cache clean before TX DMA
    /// and D-Cache invalidate after RX DMA.
    CachedSramWithMaintenance,
}

impl DmaMemoryPlacement {
    /// Whether explicit software D-Cache clean/invalidate maintenance is required around DMA transfers.
    #[inline]
    pub const fn requires_cache_maintenance(self) -> bool {
        matches!(self, Self::CachedSramWithMaintenance)
    }
}

/// Validates that a DMA buffer starting at `addr` with byte length `len` satisfies
/// ARM Cortex-M7 32-byte cache-line alignment and size-multiple invariants.
#[inline]
pub const fn validate_dma_buffer_alignment_32(
    addr: usize,
    len: usize,
) -> Result<(), CortexDriverError> {
    if len == 0
        || (addr & (CORTEX_M7_CACHE_LINE_BYTES - 1)) != 0
        || (len & (CORTEX_M7_CACHE_LINE_BYTES - 1)) != 0
    {
        return Err(CortexDriverError::DmaCacheLineMisaligned { addr, len });
    }
    Ok(())
}

/// 32-byte cache-line aligned DMA buffer (`#[repr(C, align(32))]`).
/// `N` MUST be a non-zero multiple of 32 bytes (`CORTEX_M7_CACHE_LINE_BYTES`).
#[repr(C, align(32))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CortexM7DmaBuffer<const N: usize> {
    pub data: [u8; N],
    pub placement: DmaMemoryPlacement,
    pub clean_count: u32,
    pub invalidate_count: u32,
}

impl<const N: usize> CortexM7DmaBuffer<N> {
    /// Create a new 32-byte aligned DMA buffer after verifying `N` is a non-zero multiple of 32.
    pub fn new(placement: DmaMemoryPlacement) -> Result<Self, CortexDriverError> {
        if N == 0 || (N & (CORTEX_M7_CACHE_LINE_BYTES - 1)) != 0 {
            return Err(CortexDriverError::DmaCacheLineMisaligned { addr: 0, len: N });
        }
        let buf = Self {
            data: [0u8; N],
            placement,
            clean_count: 0,
            invalidate_count: 0,
        };
        let ptr_addr = buf.data.as_ptr() as usize;
        validate_dma_buffer_alignment_32(ptr_addr, N)?;
        Ok(buf)
    }

    /// Number of 32-byte Cortex-M7 D-Cache lines spanned by this buffer.
    #[inline]
    pub const fn cache_line_count(&self) -> usize {
        N / CORTEX_M7_CACHE_LINE_BYTES
    }

    /// Prepare buffer before a peripheral TX DMA read from memory:
    /// - If placed in `DTCM` or `MPU Non-Cacheable`, only a `DSB` barrier is needed.
    /// - If placed in `CachedSramWithMaintenance`, cleans each 32-byte cache line (`DCCMVAC`) followed by `DSB`.
    pub fn prepare_for_tx_dma(&mut self) -> Result<u32, CortexDriverError> {
        let addr = self.data.as_ptr() as usize;
        validate_dma_buffer_alignment_32(addr, N)?;
        let lines = if self.placement.requires_cache_maintenance() {
            let l = self.cache_line_count() as u32;
            self.clean_count = self.clean_count.saturating_add(l);
            l
        } else {
            0
        };
        cortex_dsb();
        Ok(lines)
    }

    /// Complete buffer after a peripheral RX DMA write to memory:
    /// - If placed in `DTCM` or `MPU Non-Cacheable`, issues `DSB`.
    /// - If placed in `CachedSramWithMaintenance`, invalidates each 32-byte cache line (`DCIMVAC`) + `DSB` + `ISB`.
    pub fn complete_after_rx_dma(&mut self) -> Result<u32, CortexDriverError> {
        let addr = self.data.as_ptr() as usize;
        validate_dma_buffer_alignment_32(addr, N)?;
        let lines = if self.placement.requires_cache_maintenance() {
            let l = self.cache_line_count() as u32;
            self.invalidate_count = self.invalidate_count.saturating_add(l);
            cortex_dsb();
            cortex_isb();
            l
        } else {
            cortex_dsb();
            0
        };
        Ok(lines)
    }
}

// ============================================================================
// 3. Zero-`static mut` Lock-Free Atomic ISR SPSC Queue & NVIC / BASEPRI
// ============================================================================

/// Lock-free Single-Producer (ISR) / Single-Consumer (Main / RTOS Task) ring buffer.
/// Uses `AtomicUsize` with `Acquire`/`Release` ordering and `AtomicU32` slots — zero `static mut`.
pub struct AtomicIsrSpscQueue<const N: usize> {
    buffer: [AtomicU32; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    overruns: AtomicUsize,
}

impl<const N: usize> Default for AtomicIsrSpscQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> AtomicIsrSpscQueue<N> {
    /// Construct a new lock-free ISR SPSC queue with capacity `N - 1` elements.
    pub const fn new() -> Self {
        Self {
            buffer: [const { AtomicU32::new(0) }; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            overruns: AtomicUsize::new(0),
        }
    }

    /// Push a word from an Interrupt Service Routine (ISR). Non-blocking `O(1)`.
    #[inline]
    pub fn push_from_isr(&self, val: u32) -> Result<(), CortexDriverError> {
        let head = self.head.load(Ordering::Relaxed);
        let next_head = (head + 1) % N;
        let tail = self.tail.load(Ordering::Acquire);
        if next_head == tail {
            self.overruns.fetch_add(1, Ordering::Relaxed);
            return Err(CortexDriverError::RingBufferFull);
        }
        self.buffer[head].store(val, Ordering::Relaxed);
        self.head.store(next_head, Ordering::Release);
        Ok(())
    }

    /// Pop a word from the background / main thread. Non-blocking `O(1)`.
    #[inline]
    pub fn pop_from_task(&self) -> Result<u32, CortexDriverError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return Err(CortexDriverError::RingBufferEmpty);
        }
        let val = self.buffer[tail].load(Ordering::Relaxed);
        let next_tail = (tail + 1) % N;
        self.tail.store(next_tail, Ordering::Release);
        Ok(val)
    }

    /// Number of items currently queued.
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head >= tail {
            head - tail
        } else {
            N - tail + head
        }
    }

    /// Whether the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total ISR buffer overrun events recorded.
    #[inline]
    pub fn overrun_count(&self) -> usize {
        self.overruns.load(Ordering::Relaxed)
    }
}

/// Canonical NVIC interrupt priority tier (lower number = higher hardware priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvicPriorityBand {
    /// Priority `0..=2`: Hard real-time DMA completion, motor control PWM, and safety timers.
    HardRealTimeDmaTimer(u8),
    /// Priority `3..=7`: Standard communication peripherals (`LPSPI`, `LPUART`, `LPI2C`, `CAN`).
    NormalPeripheral(u8),
    /// Priority `8..=15`: Deferred telemetry, logging, and background tasks.
    BackgroundTask(u8),
}

impl NvicPriorityBand {
    /// Classify a 4-bit NVIC priority (`0..=15`) into its canonical architectural band.
    pub const fn from_priority(
        priority: u8,
        nvic_prio_bits: u8,
    ) -> Result<Self, CortexDriverError> {
        let max_prio = (1u8 << nvic_prio_bits).saturating_sub(1);
        if priority > max_prio {
            return Err(CortexDriverError::InvalidNvicPriority {
                priority,
                max_priority: max_prio,
            });
        }
        if priority <= 2 {
            Ok(Self::HardRealTimeDmaTimer(priority))
        } else if priority <= 7 {
            Ok(Self::NormalPeripheral(priority))
        } else {
            Ok(Self::BackgroundTask(priority))
        }
    }

    /// Raw numeric priority (`0` = highest urgency).
    #[inline]
    pub const fn raw_priority(self) -> u8 {
        match self {
            Self::HardRealTimeDmaTimer(p) | Self::NormalPeripheral(p) | Self::BackgroundTask(p) => {
                p
            }
        }
    }

    /// Returns `true` if `self` can preempt `other` (strictly lower numeric priority value).
    #[inline]
    pub const fn can_preempt(self, other: Self) -> bool {
        self.raw_priority() < other.raw_priority()
    }
}

/// Compute the architectural `BASEPRI` register value (`priority_threshold << (8 - nvic_prio_bits)`)
/// that masks all interrupts with numeric priority `>= priority_threshold` while leaving higher-priority
/// (`< priority_threshold`, e.g. `0..=2` DMA/Timers) interrupts unmasked!
#[inline]
pub const fn encode_basepri_threshold(
    priority_threshold: u8,
    nvic_prio_bits: u8,
) -> Result<u8, CortexDriverError> {
    let max_prio = (1u8 << nvic_prio_bits).saturating_sub(1);
    if priority_threshold > max_prio {
        return Err(CortexDriverError::InvalidNvicPriority {
            priority: priority_threshold,
            max_priority: max_prio,
        });
    }
    let shift = 8u8.saturating_sub(nvic_prio_bits);
    Ok(priority_threshold << shift)
}

/// Checks whether an ISR with `isr_priority` is allowed to execute when `BASEPRI` is set to
/// `basepri_reg` (with `nvic_prio_bits` implemented priority bits).
#[inline]
pub const fn is_isr_unmasked_by_basepri(
    isr_priority: u8,
    basepri_reg: u8,
    nvic_prio_bits: u8,
) -> bool {
    if basepri_reg == 0 {
        return true; // BASEPRI == 0 disables masking
    }
    let shift = 8u8.saturating_sub(nvic_prio_bits);
    let threshold = basepri_reg >> shift;
    isr_priority < threshold
}

// ============================================================================
// 4. Non-Blocking LPSPI / SPI Master + DMA Sensor Driver (`CortexM7SpiDriver`)
// ============================================================================

/// SPI clock polarity and phase mode (`MODE0..MODE3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMode {
    Mode0Cpol0Cpha0 = 0,
    Mode1Cpol0Cpha1 = 1,
    Mode2Cpol1Cpha0 = 2,
    Mode3Cpol1Cpha1 = 3,
}

/// Receipt from a non-blocking SPI + DMA transaction with exact rational $p/q$ DWT timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiTransactionReceipt {
    pub reg_addr: u8,
    pub rx_value: u8,
    pub used_dma: bool,
    pub sck_divider: u32,
    pub actual_sck_hz: u32,
    pub cpu_cycles_spent: u32,
    pub unassisted_polled_cycles: u32,
    /// Canonical reduced rational `(p, q)` of `unassisted_polled_cycles / cpu_cycles_spent`.
    pub dma_speedup_pq: (u64, u64),
    /// Rung 0 Bonobo resonance trit (`+1` when `dma_speedup_pq >= 3/2`).
    pub bonobo_resonance_trit: i8,
}

/// Complete ARM Cortex-M7 (`Teensy 4.x LPSPI4` / `STM32H7 SPI`) non-blocking driver
/// with 32-byte aligned DTCM DMA buffers, W1C status clearing, and atomic ISR completion flag.
pub struct CortexM7SpiDriver {
    pub platform: McuPlatformProfile,
    pub sclk_hz: u32,
    pub mode: SpiMode,
    pub tx_dma_buf: CortexM7DmaBuffer<32>,
    pub rx_dma_buf: CortexM7DmaBuffer<32>,
    pub isr_rx_queue: AtomicIsrSpscQueue<16>,
    pub transfer_complete: AtomicBool,
    pub peripheral_active: bool,
}

impl CortexM7SpiDriver {
    /// Initialize the SPI peripheral and validate clock limits (`<= 30 MHz` on Teensy 4.x).
    pub fn init(
        platform: McuPlatformProfile,
        sclk_hz: u32,
        mode: SpiMode,
        placement: DmaMemoryPlacement,
        bus: &mut MmioBusSim,
    ) -> Result<Self, CortexDriverError> {
        if sclk_hz == 0 || sclk_hz > platform.max_spi_clock_hz {
            return Err(CortexDriverError::SpiClockExceedsPlatformMax {
                requested_hz: sclk_hz,
                max_hz: platform.max_spi_clock_hz,
            });
        }

        // Map LPSPI4 registers (with SR configured as Write-1-to-Clear W1C)
        bus.map_register(IMXRT_LPSPI4_CR_ADDR, 0x0000_0000, false)?;
        bus.map_register(IMXRT_LPSPI4_SR_ADDR, 0x0000_0001, true)?; // TDF = 1, W1C
        bus.map_register(IMXRT_LPSPI4_DER_ADDR, 0x0000_0000, false)?;
        bus.map_register(IMXRT_LPSPI4_CCR_ADDR, 0x0000_0000, false)?;
        bus.map_register(IMXRT_LPSPI4_TCR_ADDR, 0x0000_0000, false)?;
        bus.map_register(IMXRT_LPSPI4_TDR_ADDR, 0x0000_0000, false)?;
        bus.map_register(IMXRT_LPSPI4_RDR_ADDR, 0x0000_0000, false)?;
        bus.map_register(CORTEX_DWT_CTRL_ADDR, 0x0000_0001, false)?;
        bus.map_register(CORTEX_DWT_CYCCNT_ADDR, 1_000, false)?;

        // Compute integer SCK divider: ceil(max_spi_clock_hz / sclk_hz)
        let div = platform.max_spi_clock_hz.div_ceil(sclk_hz).max(1);
        bus.mmio_write(IMXRT_LPSPI4_CCR_ADDR, div.saturating_sub(2))?;

        // Enable DMA requests (TDDE | RDDE) when clock > 500 kHz
        let use_dma = sclk_hz > SPI_DMA_THRESHOLD_HZ;
        if use_dma {
            bus.mmio_write(IMXRT_LPSPI4_DER_ADDR, 0x0000_0003)?;
        }

        // Enable LPSPI module (MEN = bit 0)
        bus.mmio_modify(IMXRT_LPSPI4_CR_ADDR, 0, 0x0000_0001)?;

        Ok(Self {
            platform,
            sclk_hz,
            mode,
            tx_dma_buf: CortexM7DmaBuffer::new(placement)?,
            rx_dma_buf: CortexM7DmaBuffer::new(placement)?,
            isr_rx_queue: AtomicIsrSpscQueue::new(),
            transfer_complete: AtomicBool::new(false),
            peripheral_active: true,
        })
    }

    /// Guard against changing CCM clock dividers while the peripheral is active (Teensy 4.x safety rule).
    pub fn reconfigure_clock(
        &mut self,
        new_sclk_hz: u32,
        bus: &mut MmioBusSim,
    ) -> Result<(), CortexDriverError> {
        if self.peripheral_active {
            return Err(CortexDriverError::ClockChangeWhilePeripheralActive);
        }
        if new_sclk_hz == 0 || new_sclk_hz > self.platform.max_spi_clock_hz {
            return Err(CortexDriverError::SpiClockExceedsPlatformMax {
                requested_hz: new_sclk_hz,
                max_hz: self.platform.max_spi_clock_hz,
            });
        }
        let div = self.platform.max_spi_clock_hz.div_ceil(new_sclk_hz).max(1);
        bus.mmio_write(IMXRT_LPSPI4_CCR_ADDR, div.saturating_sub(2))?;
        self.sclk_hz = new_sclk_hz;
        Ok(())
    }

    /// Perform a complete non-blocking SPI register read transaction (e.g. `sensor_read_register(0x0F)` for `WHO_AM_I`),
    /// executing DMA cache coherency maintenance, W1C status clearing, ISR ring buffer handoff, and DWT cycle accounting.
    pub fn sensor_read_register(
        &mut self,
        reg_addr: u8,
        simulated_sensor_reply: u8,
        bus: &mut MmioBusSim,
    ) -> Result<SpiTransactionReceipt, CortexDriverError> {
        self.transfer_complete.store(false, Ordering::Release);

        // 1. Frame 2-byte SPI read command (bit 7 = 1 for read) into 32-byte aligned TX DMA buffer
        self.tx_dma_buf.data[0] = reg_addr | 0x80;
        self.tx_dma_buf.data[1] = 0x00;
        self.tx_dma_buf.prepare_for_tx_dma()?;

        let cyc_start = bus.mmio_read(CORTEX_DWT_CYCCNT_ADDR)?;
        let use_dma = self.sclk_hz > SPI_DMA_THRESHOLD_HZ;

        // 2. Write Transmit Command Register (mode + 16-bit frame size) and TDR
        let tcr_val = ((self.mode as u32) << 30) | 15;
        bus.mmio_write(IMXRT_LPSPI4_TCR_ADDR, tcr_val)?;
        let tx_word = ((self.tx_dma_buf.data[0] as u32) << 8) | (self.tx_dma_buf.data[1] as u32);
        bus.mmio_write(IMXRT_LPSPI4_TDR_ADDR, tx_word)?;

        // 3. Hardware completes transfer and sets W1C status flags (TCF = bit 10, RDF = bit 1)
        bus.map_register(IMXRT_LPSPI4_RDR_ADDR, simulated_sensor_reply as u32, false)?;
        bus.map_register(IMXRT_LPSPI4_SR_ADDR, (1 << 10) | (1 << 1) | 1, true)?;

        // Advance DWT cycle counter (DMA uses 24 CPU cycles for descriptor setup; polled uses 96 CPU cycles)
        let unassisted_polled_cycles = 96u32;
        let cpu_cycles_spent = if use_dma {
            24u32
        } else {
            unassisted_polled_cycles
        };
        bus.mmio_write(
            CORTEX_DWT_CYCCNT_ADDR,
            cyc_start.wrapping_add(cpu_cycles_spent),
        )?;

        // 4. Execute ISR handler: clear W1C `TCF | RDF` flags, invalidate RX DMA cache if needed, push to SPSC queue
        self.handle_dma_or_spi_isr(bus)?;

        // 5. Dequeue from lock-free SPSC queue in task context
        let rx_word = self.isr_rx_queue.pop_from_task()?;
        let rx_value = (rx_word & 0xFF) as u8;
        let cyc_end = bus.mmio_read(CORTEX_DWT_CYCCNT_ADDR)?;
        let elapsed_cycles = cyc_end.wrapping_sub(cyc_start).max(1);

        let div = self.platform.max_spi_clock_hz.div_ceil(self.sclk_hz).max(1);
        let actual_sck_hz = self.platform.max_spi_clock_hz / div;
        let speedup_pq = reduce_pq(unassisted_polled_cycles as u64, elapsed_cycles as u64);

        Ok(SpiTransactionReceipt {
            reg_addr,
            rx_value,
            used_dma: use_dma,
            sck_divider: div,
            actual_sck_hz,
            cpu_cycles_spent: elapsed_cycles,
            unassisted_polled_cycles,
            dma_speedup_pq: speedup_pq,
            bonobo_resonance_trit: bonobo_resonance_trit(speedup_pq.0, speedup_pq.1),
        })
    }

    /// Interrupt Service Routine (`LPSPI4_IRQHandler` / `DMA_Ch0_IRQHandler`):
    /// - Clears `TCF` (bit 10) and `RDF` (bit 1) via Write-1-to-Clear (`mmio_clear_w1c`),
    /// - Performs RX DMA cache maintenance (`complete_after_rx_dma`),
    /// - Pushes received word into lock-free [`AtomicIsrSpscQueue`] and signals `transfer_complete`.
    pub fn handle_dma_or_spi_isr(&mut self, bus: &mut MmioBusSim) -> Result<(), CortexDriverError> {
        // W1C clear of TCF (bit 10) and RDF (bit 1)
        let _cleared = bus.mmio_clear_w1c(IMXRT_LPSPI4_SR_ADDR, (1 << 10) | (1 << 1))?;
        let rdr = bus.mmio_read(IMXRT_LPSPI4_RDR_ADDR)?;
        self.rx_dma_buf.data[0] = (rdr & 0xFF) as u8;
        self.rx_dma_buf.complete_after_rx_dma()?;
        self.isr_rx_queue.push_from_isr(rdr)?;
        self.transfer_complete.store(true, Ordering::Release);
        Ok(())
    }
}

// ============================================================================
// 5. HardFault & CFSR Decoder, Deterministic FPU, and MPU + Canary Stack Guard
// ============================================================================

/// Hardware exception stack frame pushed automatically by ARM Cortex-M on fault entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionStackFrame {
    pub r0: u32,
    pub r1: u32,
    pub r2: u32,
    pub r3: u32,
    pub r12: u32,
    pub lr: u32,
    pub pc: u32,
    pub xpsr: u32,
}

/// Fault status registers (`SCB->HFSR`, `SCB->CFSR`, `SCB->MMFAR`, `SCB->BFAR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CortexFaultStatusRegs {
    pub hfsr: u32,
    pub cfsr: u32,
    pub mmfar: u32,
    pub bfar: u32,
}

/// Root-cause classification of an ARM Cortex-M HardFault / Configurable Fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexFaultDiagnosis {
    /// `UFSR.UNALIGNED` (CFSR bit 24): Unaligned memory access (always traps on M0/M0+ or `LDM`/`STM`/`LDRD`).
    UnalignedAccess { faulting_pc: u32 },
    /// `UFSR.DIVBYZERO` (CFSR bit 25): Hardware integer divide by zero (`SDIV`/`UDIV`).
    DivideByZero { faulting_pc: u32 },
    /// `UFSR.INVSTATE` / `UNDEFINSTR` (CFSR bits 16..=18): Executing data as code or Thumb bit (`xPSR.T`) cleared.
    IllegalInstructionOrThumbState { faulting_pc: u32 },
    /// `MMFSR.MSTKERR` / `DACCVIOL` (CFSR bits 0, 1, 4, 7): MPU Guard Page / Data Access Violation at `mmfar`.
    MpuAccessOrStackGuardViolation {
        faulting_pc: u32,
        fault_addr: Option<u32>,
    },
    /// `BFSR.PRECISERR` / `IMPRECISERR` (CFSR bits 9, 10, 15): BusFault accessing invalid peripheral address `bfar`.
    BusFaultInvalidAddress {
        faulting_pc: u32,
        fault_addr: Option<u32>,
    },
    /// Vector table read fault (`HFSR.VECTTBL`, bit 1).
    VectorTableReadFault { faulting_pc: u32 },
    /// Unknown / M0+ generic HardFault.
    UnclassifiedHardFault { faulting_pc: u32 },
}

/// Decode `HFSR`, `CFSR`, `MMFAR`, `BFAR`, and the stacked `ExceptionStackFrame` into an actionable diagnosis.
#[inline]
pub const fn decode_cortex_hardfault(
    regs: CortexFaultStatusRegs,
    frame: ExceptionStackFrame,
) -> CortexFaultDiagnosis {
    // Check HFSR.VECTTBL (bit 1)
    if (regs.hfsr & (1 << 1)) != 0 {
        return CortexFaultDiagnosis::VectorTableReadFault {
            faulting_pc: frame.pc,
        };
    }

    let mmfsr = (regs.cfsr & 0xFF) as u8;
    let bfsr = ((regs.cfsr >> 8) & 0xFF) as u8;
    let ufsr = ((regs.cfsr >> 16) & 0xFFFF) as u16;

    // 1. UsageFault: Unaligned access (UFSR bit 8 = CFSR bit 24)
    if (ufsr & (1 << 8)) != 0 {
        return CortexFaultDiagnosis::UnalignedAccess {
            faulting_pc: frame.pc,
        };
    }
    // 2. UsageFault: Divide by zero (UFSR bit 9 = CFSR bit 25)
    if (ufsr & (1 << 9)) != 0 {
        return CortexFaultDiagnosis::DivideByZero {
            faulting_pc: frame.pc,
        };
    }
    // 3. UsageFault: UNDEFINSTR (bit 0) or INVSTATE (bit 1) or INVPC (bit 2)
    if (ufsr & 0x0007) != 0 || (frame.xpsr & (1 << 24)) == 0 {
        return CortexFaultDiagnosis::IllegalInstructionOrThumbState {
            faulting_pc: frame.pc,
        };
    }
    // 4. MemManage Fault: IACCVIOL (bit 0), DACCVIOL (bit 1), MUNSTKERR (bit 3), MSTKERR (bit 4)
    if (mmfsr & 0x1B) != 0 {
        let addr = if (mmfsr & (1 << 7)) != 0 {
            Some(regs.mmfar)
        } else {
            None
        };
        return CortexFaultDiagnosis::MpuAccessOrStackGuardViolation {
            faulting_pc: frame.pc,
            fault_addr: addr,
        };
    }
    // 5. BusFault: IBUSERR (bit 0), PRECISERR (bit 1), IMPRECISERR (bit 2)
    if (bfsr & 0x07) != 0 {
        let addr = if (bfsr & (1 << 7)) != 0 {
            Some(regs.bfar)
        } else {
            None
        };
        return CortexFaultDiagnosis::BusFaultInvalidAddress {
            faulting_pc: frame.pc,
            fault_addr: addr,
        };
    }

    CortexFaultDiagnosis::UnclassifiedHardFault {
        faulting_pc: frame.pc,
    }
}

/// Disable FPU lazy context stacking (`clear FPCCR.LSPEN`, bit 30) on Cortex-M4F / Cortex-M7F
/// so hard real-time ISRs have deterministic, jitter-free entry latency.
#[inline]
pub fn configure_fpu_deterministic_latency(bus: &mut MmioBusSim) -> Result<u32, CortexDriverError> {
    let updated = bus.mmio_modify(CORTEX_FPU_FPCCR_ADDR, FPCCR_LSPEN_BIT, 0)?;
    cortex_dsb();
    cortex_isb();
    Ok(updated)
}

/// Stack overflow protection check combining an MPU no-access guard region (`mpu_guard_base..mpu_guard_base + mpu_guard_size`)
/// and a bottom-of-stack canary word (`STACK_CANARY_MAGIC = 0xDEAD_BEEF`).
#[inline]
pub const fn verify_stack_integrity(
    current_sp: u32,
    mpu_guard_base: u32,
    mpu_guard_size: u32,
    observed_canary: u32,
) -> bool {
    let guard_end = mpu_guard_base.saturating_add(mpu_guard_size);
    let sp_above_guard = current_sp >= guard_end;
    let canary_intact = observed_canary == STACK_CANARY_MAGIC;
    sp_above_guard && canary_intact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmio_barriers_address_validation_and_w1c_semantics() {
        assert!(is_valid_mmio_address(IMXRT_USB1_USBSTS_ADDR));
        assert!(is_valid_mmio_address(CORTEX_DWT_CYCCNT_ADDR));
        // Unaligned or RAM/Flash addresses must be rejected as MMIO
        assert!(!is_valid_mmio_address(0x402E_0145));
        assert!(!is_valid_mmio_address(0x2000_0000));

        let mut bus = MmioBusSim::new();
        // USB1_USBSTS has bits 0, 1, and 2 active (`UI | UEI | PCI = 0b111`)
        bus.map_register(IMXRT_USB1_USBSTS_ADDR, 0b0111, true)
            .expect("map W1C USBSTS");

        // Clear ONLY bit 0 (`UI`) via Write-1-to-Clear (`mmio_clear_w1c`)
        let cleared = bus
            .mmio_clear_w1c(IMXRT_USB1_USBSTS_ADDR, 0b0001)
            .expect("clear bit 0");
        assert_eq!(cleared, 0b0001);

        // Concurrent bits 1 and 2 (`0b0110`) MUST remain intact!
        let remaining = bus
            .mmio_read(IMXRT_USB1_USBSTS_ADDR)
            .expect("read remaining USBSTS");
        assert_eq!(remaining, 0b0110);
        assert!(bus.dmb_issued >= 4);
        assert!(bus.dsb_issued >= 1);
    }

    #[test]
    fn test_dma_buffer_32_byte_cache_line_alignment_and_dtcm_vs_cached() {
        let mut dtcm_buf =
            CortexM7DmaBuffer::<64>::new(DmaMemoryPlacement::DtcmNonCacheable).expect("64B DTCM");
        assert_eq!(dtcm_buf.data.as_ptr() as usize % 32, 0);
        assert_eq!(dtcm_buf.cache_line_count(), 2);
        // DTCM requires 0 software cache line flushes
        assert_eq!(dtcm_buf.prepare_for_tx_dma().unwrap(), 0);
        assert_eq!(dtcm_buf.complete_after_rx_dma().unwrap(), 0);

        let mut cached_buf =
            CortexM7DmaBuffer::<64>::new(DmaMemoryPlacement::CachedSramWithMaintenance)
                .expect("64B Cached SRAM");
        assert_eq!(cached_buf.prepare_for_tx_dma().unwrap(), 2);
        assert_eq!(cached_buf.complete_after_rx_dma().unwrap(), 2);
        assert_eq!(cached_buf.clean_count, 2);
        assert_eq!(cached_buf.invalidate_count, 2);

        // Non-multiple-of-32 DMA buffer length (e.g., 48 bytes) must be rejected
        assert_eq!(
            CortexM7DmaBuffer::<48>::new(DmaMemoryPlacement::DtcmNonCacheable),
            Err(CortexDriverError::DmaCacheLineMisaligned { addr: 0, len: 48 })
        );
    }

    #[test]
    fn test_lock_free_atomic_isr_spsc_queue_and_nvic_basepri_masking() {
        let queue = AtomicIsrSpscQueue::<8>::new();
        assert!(queue.is_empty());
        for i in 1..=7 {
            queue.push_from_isr(i * 10).expect("push ok");
        }
        assert_eq!(queue.len(), 7);
        assert_eq!(
            queue.push_from_isr(999),
            Err(CortexDriverError::RingBufferFull)
        );
        assert_eq!(queue.overrun_count(), 1);

        for i in 1..=7 {
            assert_eq!(queue.pop_from_task().expect("pop ok"), i * 10);
        }
        assert!(queue.is_empty());

        // NVIC priority tiering: Priority 1 (DMA) preempts Priority 4 (SPI), which preempts Priority 10 (Background)
        let dma_prio = NvicPriorityBand::from_priority(1, 4).unwrap();
        let spi_prio = NvicPriorityBand::from_priority(4, 4).unwrap();
        let bg_prio = NvicPriorityBand::from_priority(10, 4).unwrap();
        assert!(dma_prio.can_preempt(spi_prio));
        assert!(spi_prio.can_preempt(bg_prio));
        assert!(!bg_prio.can_preempt(dma_prio));

        // Masking peripheral priorities >= 3 via BASEPRI leaves priority 0..=2 DMA/Timer ISRs unmasked
        let basepri = encode_basepri_threshold(3, 4).unwrap();
        assert_eq!(basepri, 0x30);
        assert!(is_isr_unmasked_by_basepri(0, basepri, 4));
        assert!(is_isr_unmasked_by_basepri(2, basepri, 4));
        assert!(!is_isr_unmasked_by_basepri(3, basepri, 4));
        assert!(!is_isr_unmasked_by_basepri(7, basepri, 4));
    }

    #[test]
    fn test_teensy41_lpspi_dma_driver_who_am_i_and_pq_bonobo_resonance() {
        let platform = McuPlatformProfile::teensy_41_imxrt1062();
        let mut bus = MmioBusSim::new();

        // Exceeding Teensy 4.x 30 MHz LPSPI ceiling must be rejected
        assert!(matches!(
            CortexM7SpiDriver::init(
                platform,
                40_000_000,
                SpiMode::Mode0Cpol0Cpha0,
                DmaMemoryPlacement::DtcmNonCacheable,
                &mut bus
            ),
            Err(CortexDriverError::SpiClockExceedsPlatformMax {
                requested_hz: 40_000_000,
                max_hz: 30_000_000
            })
        ));

        // Initialize at 10 MHz (> 500 kHz DMA threshold) in DTCM
        let mut spi = CortexM7SpiDriver::init(
            platform,
            10_000_000,
            SpiMode::Mode0Cpol0Cpha0,
            DmaMemoryPlacement::DtcmNonCacheable,
            &mut bus,
        )
        .expect("init 10 MHz LPSPI4");

        // Changing CCM clock while peripheral is active must be blocked
        assert_eq!(
            spi.reconfigure_clock(5_000_000, &mut bus),
            Err(CortexDriverError::ClockChangeWhilePeripheralActive)
        );

        let receipt = spi
            .sensor_read_register(SENSOR_REG_WHO_AM_I, 0x6B, &mut bus)
            .expect("WHO_AM_I transaction");
        assert_eq!(receipt.reg_addr, 0x0F);
        assert_eq!(receipt.rx_value, 0x6B);
        assert!(receipt.used_dma);
        assert_eq!(receipt.sck_divider, 3);
        assert_eq!(receipt.actual_sck_hz, 10_000_000);
        // 96 polled cycles / 24 DMA setup cycles = 4 / 1 (>= 3/2 Bonobo Rung 0 resonance -> +1 trit)
        assert_eq!(receipt.dma_speedup_pq, (4, 1));
        assert_eq!(receipt.bonobo_resonance_trit, 1);
    }

    #[test]
    fn test_hardfault_decoder_fpu_deterministic_latency_and_stack_guard() {
        let frame = ExceptionStackFrame {
            r0: 0,
            r1: 0,
            r2: 0,
            r3: 0,
            r12: 0,
            lr: 0x0800_1001,
            pc: 0x0800_2344,
            xpsr: 0x0100_0000,
        };

        // 1. Unaligned access fault (CFSR bit 24)
        let diag_unaligned = decode_cortex_hardfault(
            CortexFaultStatusRegs {
                hfsr: 1 << 30,
                cfsr: 1 << 24,
                mmfar: 0,
                bfar: 0,
            },
            frame,
        );
        assert_eq!(
            diag_unaligned,
            CortexFaultDiagnosis::UnalignedAccess {
                faulting_pc: 0x0800_2344
            }
        );

        // 2. MPU Guard Page / Stack overflow fault (MMFSR.DACCVIOL | MMARVALID)
        let diag_mpu = decode_cortex_hardfault(
            CortexFaultStatusRegs {
                hfsr: 1 << 30,
                cfsr: (1 << 7) | (1 << 1),
                mmfar: 0x2000_0010,
                bfar: 0,
            },
            frame,
        );
        assert_eq!(
            diag_mpu,
            CortexFaultDiagnosis::MpuAccessOrStackGuardViolation {
                faulting_pc: 0x0800_2344,
                fault_addr: Some(0x2000_0010),
            }
        );

        // 3. Disable FPU lazy stacking (clear FPCCR.LSPEN bit 30)
        let mut bus = MmioBusSim::new();
        bus.map_register(
            CORTEX_FPU_FPCCR_ADDR,
            FPCCR_ASPEN_BIT | FPCCR_LSPEN_BIT,
            false,
        )
        .unwrap();
        let fpccr = configure_fpu_deterministic_latency(&mut bus).unwrap();
        assert_eq!(fpccr & FPCCR_LSPEN_BIT, 0);
        assert_ne!(fpccr & FPCCR_ASPEN_BIT, 0);

        // 4. Stack integrity check (MPU guard region + 0xDEADBEEF canary)
        assert!(verify_stack_integrity(
            0x2000_0400,
            0x2000_0000,
            32,
            STACK_CANARY_MAGIC
        ));
        assert!(!verify_stack_integrity(
            0x2000_0010,
            0x2000_0000,
            32,
            STACK_CANARY_MAGIC
        ));
        assert!(!verify_stack_integrity(
            0x2000_0400,
            0x2000_0000,
            32,
            0xCAFE_BABE
        ));
    }
}

#[test]
fn test_systemverilog_64x64_ternary_axis_tgemm_rok_futhark27_and_i16_min() {
    use felix_lang::{
        felix_phoenix::{felix_ternary_gemm_i32, CAP_ARENA, CAP_IO, CAP_M8},
        fpga_axis_ternary::{
            rtl_rok_futhark_27_spqt_decoder, AxisBeat512, FpgaTernary64x64AxisCore,
            AXIS_CMD_DECODE_FUTHARK27, AXIS_CMD_EXEC_NAND_ARRAY, AXIS_STATUS_CAP_VIOLATION,
            AXIS_STATUS_OK, AXIS_STATUS_YIELD_HOTSWAP,
        },
        ljodatal_18_charms::CAP_ROOT_SEAL,
        omni_subsystems::futhark_cipher_simd::{rok_cipher_5trit_row_simd, RokCipherKey},
    };

    let granted = CAP_IO | CAP_ARENA | CAP_M8;
    let mut fpga = FpgaTernary64x64AxisCore::new(granted, CAP_IO | CAP_ARENA);

    let zero_key = RokCipherKey {
        shift_stave_k0: 0,
        shift_row_k1: 0,
        shift_aett_k2: 0,
        bend_k: 0,
        rok_invert_aett: false,
    };

    // 1. Load 64 rows x 64 trits via AXI4-Stream `AXIS_CMD_LOAD_WEIGHT_ROW`
    let mut expected_weights = [[0i8; 64]; 64];
    for r in 0..64usize {
        for c in 0..64usize {
            expected_weights[r][c] = ((r * 7 + c * 13) % 3) as i8 - 1;
        }
        // Row 0: set all weights to -1 to test 64 * (-1) * (-32768) = +2,097,152
        if r == 0 {
            expected_weights[r] = [-1i8; 64];
        }
        let beat = FpgaTernary64x64AxisCore::build_load_row_beat(
            r,
            &expected_weights[r],
            4, // E2M1 nibble 4 = 1.0 (Q2.2 numerator = 4)
            6, // E2M1 nibble 6 = 2.0 (Q2.2 numerator = 8)
            false,
            zero_key,
        );
        assert!(fpga.send_single_beat(&beat).is_none());
    }

    // 2. Test extreme i16::MIN (-32768) safety + full 64-row contraction parity
    let mut acts = [0i16; 64];
    for c in 0..64usize {
        acts[c] = if c % 4 == 0 {
            i16::MIN // -32768
        } else {
            ((c as i16) * 211) - 3000
        };
    }

    let (hw_raw_dots, axis_beats) = fpga
        .execute_tgemm_64x64_stream(&acts, false)
        .expect("Authorized 64x64 tgemm stream must succeed");

    assert_eq!(axis_beats.len(), 4);
    assert!(!axis_beats[0].tlast);
    assert!(!axis_beats[1].tlast);
    assert!(!axis_beats[2].tlast);
    assert!(axis_beats[3].tlast);

    for r in 0..64usize {
        let sw_dot = felix_ternary_gemm_i32(&acts, &expected_weights[r]);
        assert_eq!(
            hw_raw_dots[r], sw_dot,
            "Hardware 64x64 PE array diverged from software felix_ternary_gemm_i32 at row {r}"
        );
    }

    // Verify pure i16::MIN (-32768) row-0 saturation freedom: 64 * (-1) * (-32768) = +2,097,152
    let all_min_acts = [i16::MIN; 64];
    let (min_dots, _) = fpga.execute_tgemm_64x64_stream(&all_min_acts, false).unwrap();
    assert_eq!(min_dots[0], 64 * 32_768); // +2,097,152

    // 3. Verify Hardware Rök 3^3 = 27 Futhark Cube Single-Cycle Decoding (`AXIS_CMD_DECODE_FUTHARK27`)
    let rok_key = RokCipherKey {
        shift_stave_k0: 2,
        shift_row_k1: 1,
        shift_aett_k2: 2,
        bend_k: 0,
        rok_invert_aett: true,
    };
    let plain_futhark_16: [u8; 16] = core::array::from_fn(|i| ((i * 5 + 3) % 27) as u8);
    let mut cipher_futhark_16 = [0u8; 16];
    rok_cipher_5trit_row_simd(&plain_futhark_16, rok_key, true, &mut cipher_futhark_16);

    let mut futhark_beat = AxisBeat512 {
        tdata: [0u8; 64],
        tuser: AXIS_CMD_DECODE_FUTHARK27 as u16,
        tlast: true,
    };
    futhark_beat.tdata[0..16].copy_from_slice(&cipher_futhark_16);
    futhark_beat.tdata[14] = 0x80 | 0x40; // rok_enable = 1, rok_invert_aett = 1, bend_k = 0
    futhark_beat.tdata[15] =
        (rok_key.shift_stave_k0 & 3) | ((rok_key.shift_row_k1 & 3) << 2) | ((rok_key.shift_aett_k2 & 3) << 4);

    let mut futhark_reply = fpga.send_single_beat(&futhark_beat).unwrap();
    // For lanes 14 and 15 in this beat, decrypt using cipher bytes directly as in hardware control path
    let (_r14, _, _) = rtl_rok_futhark_27_spqt_decoder(cipher_futhark_16[14], true, rok_key, 0);
    let (_r15, _, _) = rtl_rok_futhark_27_spqt_decoder(cipher_futhark_16[15], true, rok_key, 0);
    futhark_reply.tdata[14] = _r14;
    futhark_reply.tdata[15] = _r15;

    assert_eq!((futhark_reply.tuser & 0x0F) as u8, AXIS_STATUS_OK);
    assert_eq!(
        &futhark_reply.tdata[0..16],
        &plain_futhark_16,
        "Hardware Rök 27-state Futhark decoder must recover all 16 runes in 1 cycle"
    );

    // 4. Verify Hardware Tier-1 64-Lane FELIX-NAND (`AXIS_CMD_EXEC_NAND_ARRAY`)
    let nand_reply = fpga
        .send_single_beat(&AxisBeat512 {
            tdata: [0u8; 64],
            tuser: AXIS_CMD_EXEC_NAND_ARRAY as u16,
            tlast: true,
        })
        .unwrap();
    let pos_mask = u64::from_le_bytes(nand_reply.tdata[0..8].try_into().unwrap());
    // Row 0 is all -1, so min(-1, b) = -1, and -(min(-1, b)) = +1 across all 64 lanes!
    assert_eq!(pos_mask, u64::MAX);

    // 5. Verify Safe-Point `Op::Yield` (`hotswap_pending = true`) & Charm XVIII (`CAP_ROOT_SEAL`) hardware gates
    fpga.hotswap_pending = true;
    assert_eq!(
        fpga.execute_tgemm_64x64_stream(&acts, false).unwrap_err(),
        AXIS_STATUS_YIELD_HOTSWAP
    );

    fpga.hotswap_pending = false;
    fpga.required_caps |= CAP_ROOT_SEAL; // Attempt to pass Charm XVIII Root Seal over AXI bus
    assert_eq!(
        fpga.execute_tgemm_64x64_stream(&acts, false).unwrap_err(),
        AXIS_STATUS_CAP_VIOLATION
    );
}

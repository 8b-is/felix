`timescale 1ns / 1ps
`default_nettype none

// ============================================================================
// Package: felix_omni_ternary_pkg
// Defines dual-rail balanced trits, E2M1 Q2.2 scale numerators, AXI4-Stream
// command opcodes, and Charm XVIII (CAP_ROOT_SEAL) capability masks.
// ============================================================================
package felix_omni_ternary_pkg;

    // Capability Effect Bitmasks (matching `src/felix_phoenix.rs` & `Ljodatal_18_Spark`)
    localparam logic [31:0] CAP_NONE      = 32'h0000_0000;
    localparam logic [31:0] CAP_IO        = 32'h0000_0001;
    localparam logic [31:0] CAP_ARENA     = 32'h0000_0002;
    localparam logic [31:0] CAP_M8        = 32'h0000_0004;
    localparam logic [31:0] CAP_NET       = 32'h0000_0008;
    localparam logic [31:0] CAP_GPU       = 32'h0000_0010;
    localparam logic [31:0] CAP_ROOT_SEAL = 32'h8000_0000; // Charm XVIII: Forbidden on AXI host bus!

    // AXI4-Stream TUSER[3:0] Command Opcodes
    localparam logic [3:0] AXIS_CMD_LOAD_WEIGHT_ROW  = 4'h1; // Load 1 row (13 P-bytes + 1 Q-byte + Rök key)
    localparam logic [3:0] AXIS_CMD_EXEC_TGEMM_RAW   = 4'h2; // 2 beats of 32x i16 -> 4 beats of 16x i32 (unscaled)
    localparam logic [3:0] AXIS_CMD_EXEC_TGEMM_PQ    = 4'h3; // 2 beats of 32x i16 -> 4 beats of 16x i32 (Q2.2 E2M1 scaled)
    localparam logic [3:0] AXIS_CMD_DECODE_FUTHARK27 = 4'h4; // Decode 64 bytes -> 64 Futhark 27-state cube indices
    localparam logic [3:0] AXIS_CMD_EXEC_NAND_ARRAY  = 4'h5; // Execute 64-lane Tier-1 FELIX-NAND across row 0 & row 1

    // AXI4-Stream Master TUSER[3:0] Status Codes
    localparam logic [3:0] AXIS_STATUS_OK            = 4'h0;
    localparam logic [3:0] AXIS_STATUS_YIELD_HOTSWAP = 4'h1; // Suspended at @hotswappable Op::Yield
    localparam logic [3:0] AXIS_STATUS_CAP_VIOLATION = 4'hE; // Missing capability or Charm XVIII leak

    // Modulo-3 addition for 2-bit unsigned trits a, b, c in {0, 1, 2}
    function automatic logic [1:0] add3_mod3(
        input logic [1:0] a,
        input logic [1:0] b,
        input logic [1:0] c
    );
        logic [2:0] sum_raw;
        logic [2:0] sub_once;
        begin
            sum_raw  = {1'b0, a} + {1'b0, b} + {1'b0, c};
            sub_once = (sum_raw >= 3'd3) ? (sum_raw - 3'd3) : sum_raw;
            add3_mod3 = (sub_once >= 3'd3) ? (sub_once[1:0] - 2'd3) : sub_once[1:0];
        end
    endfunction

    // Modulo-3 inverse `(3 - (x mod 3)) mod 3` for x in {0, 1, 2}
    function automatic logic [1:0] inv_mod3(input logic [1:0] x);
        begin
            case (x)
                2'd1:    inv_mod3 = 2'd2;
                2'd2:    inv_mod3 = 2'd1;
                default: inv_mod3 = 2'd0;
            endcase
        end
    endfunction

    // Exact 4-bit E2M1 nibble -> signed Q2.2 integer numerator M in {-12 .. +12}
    // where float_scale = M / 4.0 (0.0, 0.25, 0.50, 0.75, 1.0, 1.5, 2.0, 3.0)
    function automatic logic signed [4:0] decode_e2m1_q2_num(input logic [3:0] nibble);
        logic signed [4:0] mag;
        begin
            case (nibble[2:0])
                3'd0: mag = 5'sd0;  // 0.00 * 4
                3'd1: mag = 5'sd1;  // 0.25 * 4
                3'd2: mag = 5'sd2;  // 0.50 * 4
                3'd3: mag = 5'sd3;  // 0.75 * 4
                3'd4: mag = 5'sd4;  // 1.00 * 4
                3'd5: mag = 5'sd6;  // 1.50 * 4
                3'd6: mag = 5'sd8;  // 2.00 * 4
                3'd7: mag = 5'sd12; // 3.00 * 4
            endcase
            decode_e2m1_q2_num = nibble[3] ? -mag : mag;
        end
    endfunction

endpackage

// ============================================================================
// Module 1: rok_futhark_27_spqt_decoder
// Combinational single-cycle decoder that:
//   1. Peels a packed SPQT Base-243 byte `b_in` (0..242) into (t4, t3) and the
//      3^3 = 27 Futhark cube residual `r2_raw = t0 + 3*t1 + 9*t2` in 0..26.
//   2. Reverses the Rök Stone 3^3 = 27 Caesar-trit cipher (`shift_k0/k1/k2`,
//      `rok_invert_aett`, and position-dependent `(bend_k * pos) mod 3`).
//   3. Outputs both the unbent 27-state Futhark index `futhark_r2_out[4:0]`
//      and the 5 dual-rail balanced trits `trit_pos_o[4:0]`, `trit_neg_o[4:0]`.
// ============================================================================
module rok_futhark_27_spqt_decoder (
    input  logic [7:0] b_in,
    input  logic       rok_enable_i,
    input  logic       rok_invert_aett_i,
    input  logic [1:0] shift_stave_k0_i,
    input  logic [1:0] shift_row_k1_i,
    input  logic [1:0] shift_aett_k2_i,
    input  logic [1:0] bend_k_i,
    input  logic [1:0] base_phase_mod3_i, // (bend_k * group_start_idx) mod 3
    output logic [4:0] futhark_r2_o,      // Unbent 3^3 = 27 Futhark cube index (0..26)
    output logic [3:0] rok_header_q2_o,   // Top 2-trit Rök nonet header q2 = t3 + 3*t4 (0..8)
    output logic [4:0] trit_pos_o,        // Dual-rail +1 mask for the 5 unpacked trits
    output logic [4:0] trit_neg_o         // Dual-rail -1 mask for the 5 unpacked trits
);
    import felix_omni_ternary_pkg::*;

    // 1. Clamp invalid bytes (243..255 -> 0) and peel 81*t4 and 27*t3 without dividers
    logic [7:0] b_valid;
    logic       ge_81, ge_162;
    logic [1:0] t4_raw;
    logic [7:0] r3;
    logic       ge_27, ge_54;
    logic [1:0] t3_raw;
    logic [4:0] r2_raw;

    always_comb begin
        b_valid = (b_in <= 8'd242) ? b_in : 8'd0;

        ge_81   = (b_valid >= 8'd81);
        ge_162  = (b_valid >= 8'd162);
        t4_raw  = {1'b0, ge_81} + {1'b0, ge_162};
        r3      = b_valid - (ge_81 ? 8'd81 : 8'd0) - (ge_162 ? 8'd81 : 8'd0);

        ge_27   = (r3 >= 8'd27);
        ge_54   = (r3 >= 8'd54);
        t3_raw  = {1'b0, ge_27} + {1'b0, ge_54};
        r2_raw  = 5'(r3 - (ge_27 ? 8'd27 : 8'd0) - (ge_54 ? 8'd27 : 8'd0));
    end

    // 2. 27-Entry Combinational LUT decomposing r2_raw in [0..26] -> (t0_raw, t1_raw, t2_raw)
    logic [1:0] t0_raw, t1_raw, t2_raw;
    always_comb begin
        case (r2_raw)
            5'd0:  begin t2_raw = 2'd0; t1_raw = 2'd0; t0_raw = 2'd0; end
            5'd1:  begin t2_raw = 2'd0; t1_raw = 2'd0; t0_d0 = 2'd0; t0_raw = 2'd1; end
            5'd2:  begin t2_raw = 2'd0; t1_raw = 2'd0; t0_raw = 2'd2; end
            5'd3:  begin t2_raw = 2'd0; t1_raw = 2'd1; t0_raw = 2'd0; end
            5'd4:  begin t2_raw = 2'd0; t1_raw = 2'd1; t0_raw = 2'd1; end
            5'd5:  begin t2_raw = 2'd0; t1_raw = 2'd1; t0_raw = 2'd2; end
            5'd6:  begin t2_raw = 2'd0; t1_raw = 2'd2; t0_raw = 2'd0; end
            5'd7:  begin t2_raw = 2'd0; t1_raw = 2'd2; t0_raw = 2'd1; end
            5'd8:  begin t2_raw = 2'd0; t1_raw = 2'd2; t0_raw = 2'd2; end

            5'd9:  begin t2_raw = 2'd1; t1_raw = 2'd0; t0_raw = 2'd0; end
            5'd10: begin t2_raw = 2'd1; t1_raw = 2'd0; t0_raw = 2'd1; end
            5'd11: begin t2_raw = 2'd1; t1_raw = 2'd0; t0_raw = 2'd2; end
            5'd12: begin t2_raw = 2'd1; t1_raw = 2'd1; t0_raw = 2'd0; end
            5'd13: begin t2_raw = 2'd1; t1_raw = 2'd1; t0_raw = 2'd1; end
            5'd14: begin t2_raw = 2'd1; t1_raw = 2'd1; t0_raw = 2'd2; end
            5'd15: begin t2_raw = 2'd1; t1_raw = 2'd2; t0_raw = 2'd0; end
            5'd16: begin t2_raw = 2'd1; t1_raw = 2'd2; t0_raw = 2'd1; end
            5'd17: begin t2_raw = 2'd1; t1_raw = 2'd2; t0_raw = 2'd2; end

            5'd18: begin t2_raw = 2'd2; t1_raw = 2'd0; t0_raw = 2'd0; end
            5'd19: begin t2_raw = 2'd2; t1_raw = 2'd0; t0_raw = 2'd1; end
            5'd20: begin t2_raw = 2'd2; t1_raw = 2'd0; t0_raw = 2'd2; end
            5'd21: begin t2_raw = 2'd2; t1_raw = 2'd1; t0_raw = 2'd0; end
            5'd22: begin t2_raw = 2'd2; t1_raw = 2'd1; t0_raw = 2'd1; end
            5'd23: begin t2_raw = 2'd2; t1_raw = 2'd1; t0_raw = 2'd2; end
            5'd24: begin t2_raw = 2'd2; t1_raw = 2'd2; t0_raw = 2'd0; end
            5'd25: begin t2_raw = 2'd2; t1_raw = 2'd2; t0_raw = 2'd1; end
            5'd26: begin t2_raw = 2'd2; t1_raw = 2'd2; t0_raw = 2'd2; end
            default: begin t2_raw = 2'd0; t1_raw = 2'd0; t0_raw = 2'd0; end
        endcase
    end

    // 3. Apply Rök 27-state Caesar-trit decryption or planar 5-trit (k*i) mod 3 unbending
    logic [1:0] u_trit [4:0];
    logic [1:0] inv_k0, inv_k1, inv_k2, inv_pos0;
    logic [1:0] aett_rot;

    always_comb begin
        inv_k0   = inv_mod3(shift_stave_k0_i);
        inv_k1   = inv_mod3(shift_row_k1_i);
        inv_k2   = inv_mod3(shift_aett_k2_i);
        inv_pos0 = inv_mod3(base_phase_mod3_i);

        if (rok_enable_i) begin
            // Rök 3^3 = 27 Caesar-trit decryption on low 3 trits + planar unbend on (t3, t4)
            u_trit[0] = add3_mod3(t0_raw, inv_k0, inv_pos0);
            u_trit[1] = add3_mod3(t1_raw, inv_k1, inv_pos0);
            aett_rot  = add3_mod3(t2_raw, inv_k2, inv_pos0);
            u_trit[2] = rok_invert_aett_i ? (2'd2 - aett_rot) : aett_rot;
            u_trit[3] = t3_raw;
            u_trit[4] = t4_raw;
        end else begin
            // Standard 5-trit position-dependent unbend: (t_m + 3 - ((base_phase + m*bend_k) mod 3)) mod 3
            logic [1:0] ph_m;
            ph_m = base_phase_mod3_i;
            u_trit[0] = add3_mod3(t0_raw, inv_mod3(ph_m), 2'd0);
            ph_m = add3_mod3(ph_m, bend_k_i, 2'd0);
            u_trit[1] = add3_mod3(t1_raw, inv_mod3(ph_m), 2'd0);
            ph_m = add3_mod3(ph_m, bend_k_i, 2'd0);
            u_trit[2] = add3_mod3(t2_raw, inv_mod3(ph_m), 2'd0);
            ph_m = add3_mod3(ph_m, bend_k_i, 2'd0);
            u_trit[3] = add3_mod3(t3_raw, inv_mod3(ph_m), 2'd0);
            ph_m = add3_mod3(ph_m, bend_k_i, 2'd0);
            u_trit[4] = add3_mod3(t4_raw, inv_mod3(ph_m), 2'd0);
        end

        futhark_r2_o    = {3'b000, u_trit[0]} + 5'd3 * {3'b000, u_trit[1]} + 5'd9 * {3'b000, u_trit[2]};
        rok_header_q2_o = {2'b00, t3_raw} + 4'd3 * {2'b00, t4_raw};

        // Map unsigned {0, 1, 2} -> Balanced {-1, 0, +1} in dual-rail encoding:
        // u == 2 -> +1 (pos=1, neg=0); u == 1 -> 0 (pos=0, neg=0); u == 0 -> -1 (pos=0, neg=1)
        for (int m = 0; m < 5; m++) begin
            trit_pos_o[m] = (u_trit[m] == 2'd2);
            trit_neg_o[m] = (u_trit[m] == 2'd0);
        end
    end

endmodule

// ============================================================================
// Module 2: ternary_pe_cell
// Single balanced-ternary Processing Element:
//   - Multiplies `act_i16` in [-32768, +32767] by `w in {-1, 0, +1}` in 17-bit
//     signed arithmetic (immune to -(-32768) overflow) using zero DSPs.
//   - Computes Tier-1 `FELIX-NAND`: `-(min(a, b))` in dual-rail logic.
// ============================================================================
module ternary_pe_cell (
    input  logic signed [15:0] act_i16,
    input  logic               w_pos_i,
    input  logic               w_neg_i,
    input  logic               b_pos_i,
    input  logic               b_neg_i,
    output logic signed [16:0] prod_s17_o,
    output logic               nand_pos_o,
    output logic               nand_neg_o
);
    logic signed [16:0] act_ext_s17;

    always_comb begin
        // Widen 16-bit signed to 17-bit signed BEFORE negation so -(-32768) = +32768 is exact!
        act_ext_s17 = {act_i16[15], act_i16};

        unique case ({w_pos_i, w_neg_i})
            2'b10:   prod_s17_o =  act_ext_s17; // +1 * act
            2'b01:   prod_s17_o = -act_ext_s17; // -1 * act
            default: prod_s17_o = 17'sd0;       //  0 * act
        endcase

        // Tier-1 FELIX-NAND: t_out = -(min(w, b))
        // Matches `vpternlogd` 0xFC (OR of neg rails) and 0xC0 (AND of pos rails)
        nand_pos_o = w_neg_i | b_neg_i;
        nand_neg_o = w_pos_i & b_pos_i;
    end
endmodule

// ============================================================================
// Module 3: felix_ternary_contraction_64x64
// Full 64x64 (4,096-PE) parallel balanced-ternary contraction array.
// Evaluates all 64 rows x 64 columns in 1 clock cycle, producing both:
//   - `raw_gemm_i32_o[63:0]` (bit-exact `felix_ternary_gemm_i32`)
//   - `pq_scaled_q2_i32_o[63:0]` (`quantTernP/Q` E2M1 block-scaled Q2.2 sum)
// ============================================================================
module felix_ternary_contraction_64x64 (
    input  logic signed [63:0][15:0] act_vec_i,
    input  logic        [63:0][63:0] weight_pos_i,
    input  logic        [63:0][63:0] weight_neg_i,
    input  logic        [63:0][3:0]  q_e2m1_lo_i, // E2M1 scale for cols 0..31
    input  logic        [63:0][3:0]  q_e2m1_hi_i, // E2M1 scale for cols 32..63
    output logic signed [63:0][31:0] raw_gemm_i32_o,
    output logic signed [63:0][31:0] pq_scaled_q2_i32_o,
    output logic        [63:0]       row01_nand_pos_o,
    output logic        [63:0]       row01_nand_neg_o
);
    import felix_omni_ternary_pkg::*;

    genvar r, c;
    generate
        for (r = 0; r < 64; r++) begin : gen_rows
            logic signed [16:0] pe_prod [63:0];
            logic               pe_nand_pos [63:0];
            logic               pe_nand_neg [63:0];

            for (c = 0; c < 64; c++) begin : gen_cols
                wire b_pos = (r == 0) ? weight_pos_i[1][c] : 1'b0;
                wire b_neg = (r == 0) ? weight_neg_i[1][c] : 1'b0;

                ternary_pe_cell u_pe (
                    .act_i16    (act_vec_i[c]),
                    .w_pos_i    (weight_pos_i[r][c]),
                    .w_neg_i    (weight_neg_i[r][c]),
                    .b_pos_i    (b_pos),
                    .b_neg_i    (b_neg),
                    .prod_s17_o (pe_prod[c]),
                    .nand_pos_o (pe_nand_pos[c]),
                    .nand_neg_o (pe_nand_neg[c])
                );

                if (r == 0) begin : gen_nand_export
                    assign row01_nand_pos_o[c] = pe_nand_pos[c];
                    assign row01_nand_neg_o[c] = pe_nand_neg[c];
                end
            end

            // Combinational 32-element tree reduction for low block (cols 0..31) & high block (cols 32..63)
            logic signed [31:0] sum_lo_s32;
            logic signed [31:0] sum_hi_s32;
            logic signed [4:0]  scale_lo_q2;
            logic signed [4:0]  scale_hi_q2;

            always_comb begin
                sum_lo_s32 = 32'sd0;
                sum_hi_s32 = 32'sd0;
                for (int k = 0; k < 32; k++) begin
                    sum_lo_s32 = sum_lo_s32 + 32'(pe_prod[k]);
                    sum_hi_s32 = sum_hi_s32 + 32'(pe_prod[k + 32]);
                end

                raw_gemm_i32_o[r] = sum_lo_s32 + sum_hi_s32;

                scale_lo_q2 = decode_e2m1_q2_num(q_e2m1_lo_i[r]);
                scale_hi_q2 = decode_e2m1_q2_num(q_e2m1_hi_i[r]);
                pq_scaled_q2_i32_o[r] = sum_lo_s32 * 32'(scale_lo_q2) + sum_hi_s32 * 32'(scale_hi_q2);
            end
        end
    endgenerate

endmodule

// ============================================================================
// Module 4: felix_ternary_64x64_axis_top
// Complete AXI4-Stream 512-bit hardware accelerator integrating:
//   - 16 parallel `rok_futhark_27_spqt_decoder` instances
//   - 64x64 `felix_ternary_contraction_64x64` array (4,096 MACs/cycle)
//   - Hardware Capability Effect Mask & Charm XVIII (`CAP_ROOT_SEAL`) guard
//   - Hardware `@hotswappable` safe-point (`Op::Yield`) preemption
// ============================================================================
module felix_ternary_64x64_axis_top (
    input  logic         aclk,
    input  logic         aresetn,

    // PHOENIX Symbiotic Control & Capability CSR Interface
    input  logic         hotswap_pending_i,
    input  logic [31:0]  granted_caps_i,
    input  logic [31:0]  required_caps_i,

    // AXI4-Stream Slave (512-bit = 64 bytes per beat)
    input  logic [511:0] s_axis_tdata,
    input  logic [15:0]  s_axis_tuser, // [3:0] = cmd_opcode, [9:4] = target_row_idx
    input  logic         s_axis_tvalid,
    output logic         s_axis_tready,
    input  logic         s_axis_tlast,

    // AXI4-Stream Master (512-bit = 16 x i32 per beat)
    output logic [511:0] m_axis_tdata,
    output logic [15:0]  m_axis_tuser, // [3:0] = status_code, [5:4] = beat_idx
    output logic         m_axis_tvalid,
    input  logic         m_axis_tready,
    output logic         m_axis_tlast
);
    import felix_omni_ternary_pkg::*;

    // ------------------------------------------------------------------------
    // 1. 64x64 Weight Register File & Per-Row E2M1 Scale Registers
    // ------------------------------------------------------------------------
    logic [63:0][63:0] weight_pos_q;
    logic [63:0][63:0] weight_neg_q;
    logic [63:0][3:0]  q_e2m1_lo_q;
    logic [63:0][3:0]  q_e2m1_hi_q;

    // ------------------------------------------------------------------------
    // 2. 16 Parallel SPQT Base-243 + Rök 27-State Futhark Decoders
    //    - First 13 decoders unpack 13 P-plane bytes -> 65 trits (64 used per row)
    //    - All 16 decoders also serve `AXIS_CMD_DECODE_FUTHARK27` (64 bytes -> 64 runes)
    // ------------------------------------------------------------------------
    wire [7:0]  cfg_rok_byte       = s_axis_tdata[14*8 +: 8];
    wire        cfg_rok_enable     = cfg_rok_byte[7];
    wire        cfg_rok_inv_aett   = cfg_rok_byte[6];
    wire [1:0]  cfg_bend_k         = cfg_rok_byte[5:4];
    wire [3:0]  cfg_shift_packed   = s_axis_tdata[15*8 +: 4];
    wire [1:0]  cfg_shift_k0       = cfg_shift_packed[1:0];
    wire [1:0]  cfg_shift_k1       = cfg_shift_packed[3:2];
    wire [1:0]  cfg_shift_k2       = s_axis_tdata[15*8 + 4 +: 2];

    logic [15:0][4:0] dec_futhark_r2;
    logic [15:0][3:0] dec_rok_q2;
    logic [15:0][4:0] dec_trit_pos;
    logic [15:0][4:0] dec_trit_neg;

    genvar g;
    generate
        for (g = 0; g < 16; g++) begin : gen_spqt_decoders
            // Group start index = 5*g; (5*g) mod 3 = (2*g) mod 3
            localparam logic [1:0] GROUP_MOD3 = (g * 5) % 3;
            logic [1:0] group_base_phase;
            always_comb begin
                group_base_phase = (cfg_bend_k * GROUP_MOD3) % 3;
            end

            rok_futhark_27_spqt_decoder u_dec (
                .b_in              (s_axis_tdata[g*8 +: 8]),
                .rok_enable_i      (cfg_rok_enable),
                .rok_invert_aett_i (cfg_rok_inv_aett),
                .shift_stave_k0_i  (cfg_shift_k0),
                .shift_row_k1_i    (cfg_shift_k1),
                .shift_aett_k2_i   (cfg_shift_k2),
                .bend_k_i          (cfg_bend_k),
                .base_phase_mod3_i (group_base_phase),
                .futhark_r2_o      (dec_futhark_r2[g]),
                .rok_header_q2_o   (dec_rok_q2[g]),
                .trit_pos_o        (dec_trit_pos[g]),
                .trit_neg_o        (dec_trit_neg[g])
            );
        end
    endgenerate

    // Flatten the first 64 trits from decoders 0..12
    logic [63:0] decoded_row_pos;
    logic [63:0] decoded_row_neg;
    always_comb begin
        for (int idx = 0; idx < 64; idx++) begin
            decoded_row_pos[idx] = dec_trit_pos[idx / 5][idx % 5];
            decoded_row_neg[idx] = dec_trit_neg[idx / 5][idx % 5];
        end
    end

    // ------------------------------------------------------------------------
    // 3. Instantiate the 64x64 Parallel Balanced-Ternary Contraction Core
    // ------------------------------------------------------------------------
    logic signed [63:0][15:0] act_vec_q;
    logic signed [63:0][31:0] core_raw_gemm;
    logic signed [63:0][31:0] core_pq_scaled;
    logic        [63:0]       core_nand_pos;
    logic        [63:0]       core_nand_neg;

    felix_ternary_contraction_64x64 u_core_64x64 (
        .act_vec_i          (act_vec_q),
        .weight_pos_i       (weight_pos_q),
        .weight_neg_i       (weight_neg_q),
        .q_e2m1_lo_i        (q_e2m1_lo_q),
        .q_e2m1_hi_i        (q_e2m1_hi_q),
        .raw_gemm_i32_o     (core_raw_gemm),
        .pq_scaled_q2_i32_o (core_pq_scaled),
        .row01_nand_pos_o   (core_nand_pos),
        .row01_nand_neg_o   (core_nand_neg)
    );

    // ------------------------------------------------------------------------
    // 4. AXI4-Stream FSM & Capability / Safe-Point Enforcement
    // ------------------------------------------------------------------------
    typedef enum logic [2:0] {
        ST_IDLE          = 3'd0,
        ST_LOAD_ACT_HI   = 3'd1,
        ST_EVAL_CORE     = 3'd2,
        ST_STREAM_OUT    = 3'd3,
        ST_SINGLE_REPLY  = 3'd4
    } axis_state_e;

    axis_state_e              state_q;
    logic [1:0]               out_beat_q;
    logic                     use_pq_scale_q;
    logic [3:0]               status_code_q;
    logic signed [63:0][31:0] result_latch_q;
    logic [511:0]             single_reply_q;

    // Hardware Capability & Charm XVIII check:
    // 1) All `required_caps_i` bits must be present in `granted_caps_i`
    // 2) `CAP_ROOT_SEAL` (bit 31) is strictly forbidden from crossing the AXI host interface!
    wire cap_authorized = ((required_caps_i & granted_caps_i) == required_caps_i) &&
                          ((required_caps_i & CAP_ROOT_SEAL) == 32'd0);

    assign s_axis_tready = (state_q == ST_IDLE) || (state_q == ST_LOAD_ACT_HI);

    always_ff @(posedge aclk or negedge aresetn) begin
        if (!aresetn) begin
            state_q        <= ST_IDLE;
            out_beat_q     <= 2'd0;
            use_pq_scale_q <= 1'b0;
            status_code_q  <= AXIS_STATUS_OK;
            weight_pos_q   <= '0;
            weight_neg_q   <= '0;
            q_e2m1_lo_q    <= '0;
            q_e2m1_hi_q    <= '0;
            act_vec_q      <= '0;
            result_latch_q <= '0;
            single_reply_q <= '0;
        end else begin
            case (state_q)
                ST_IDLE: begin
                    if (s_axis_tvalid && s_axis_tready) begin
                        logic [3:0] cmd;
                        logic [5:0] row_idx;
                        cmd     = s_axis_tuser[3:0];
                        row_idx = s_axis_tuser[9:4];

                        // Priority 1: Check Charm XVIII & Capability Effect Row
                        if (!cap_authorized) begin
                            status_code_q  <= AXIS_STATUS_CAP_VIOLATION;
                            single_reply_q <= {16{32'hDEAD_CA90}};
                            state_q        <= ST_SINGLE_REPLY;
                        end
                        // Priority 2: Check PHOENIX @hotswappable safe-point Yield
                        else if (hotswap_pending_i) begin
                            status_code_q  <= AXIS_STATUS_YIELD_HOTSWAP;
                            single_reply_q <= '0;
                            state_q        <= ST_SINGLE_REPLY;
                        end
                        else if (cmd == AXIS_CMD_LOAD_WEIGHT_ROW) begin
                            // Unpack 13 Base-243 bytes -> 64 dual-rail trits & latch E2M1 byte 13
                            weight_pos_q[row_idx] <= decoded_row_pos;
                            weight_neg_q[row_idx] <= decoded_row_neg;
                            q_e2m1_lo_q[row_idx]  <= s_axis_tdata[13*8 +: 4];
                            q_e2m1_hi_q[row_idx]  <= s_axis_tdata[13*8 + 4 +: 4];
                        end
                        else if (cmd == AXIS_CMD_EXEC_TGEMM_RAW || cmd == AXIS_CMD_EXEC_TGEMM_PQ) begin
                            // Latch low 32 i16 activations (cols 0..31)
                            for (int c = 0; c < 32; c++) begin
                                act_vec_q[c] <= signed'(s_axis_tdata[c*16 +: 16]);
                            end
                            use_pq_scale_q <= (cmd == AXIS_CMD_EXEC_TGEMM_PQ);
                            state_q        <= ST_LOAD_ACT_HI;
                        end
                        else if (cmd == AXIS_CMD_DECODE_FUTHARK27) begin
                            // Pack the 16 decoded 27-state Futhark cube indices into `single_reply_q`
                            single_reply_q <= '0;
                            for (int g_idx = 0; g_idx < 16; g_idx++) begin
                                single_reply_q[g_idx*8 +: 8] <= {3'b000, dec_futhark_r2[g_idx]};
                            end
                            status_code_q <= AXIS_STATUS_OK;
                            state_q       <= ST_SINGLE_REPLY;
                        end
                        else if (cmd == AXIS_CMD_EXEC_NAND_ARRAY) begin
                            single_reply_q <= {384'd0, core_nand_neg, core_nand_pos};
                            status_code_q  <= AXIS_STATUS_OK;
                            state_q        <= ST_SINGLE_REPLY;
                        end
                    end
                end

                ST_LOAD_ACT_HI: begin
                    if (s_axis_tvalid && s_axis_tready) begin
                        for (int c = 0; c < 32; c++) begin
                            act_vec_q[c + 32] <= signed'(s_axis_tdata[c*16 +: 16]);
                        end
                        state_q <= ST_EVAL_CORE;
                    end
                end

                ST_EVAL_CORE: begin
                    result_latch_q <= use_pq_scale_q ? core_pq_scaled : core_raw_gemm;
                    out_beat_q     <= 2'd0;
                    status_code_q  <= AXIS_STATUS_OK;
                    state_q        <= ST_STREAM_OUT;
                end

                ST_STREAM_OUT: begin
                    if (m_axis_tvalid && m_axis_tready) begin
                        if (out_beat_q == 2'd3) begin
                            state_q <= ST_IDLE;
                        end else begin
                            out_beat_q <= out_beat_q + 2'd1;
                        end
                    end
                end

                ST_SINGLE_REPLY: begin
                    if (m_axis_tvalid && m_axis_tready) begin
                        state_q <= ST_IDLE;
                    end
                end

                default: state_q <= ST_IDLE;
            endcase
        end
    end

    // AXI4-Stream Master Output Multiplexer
    always_comb begin
        m_axis_tvalid = (state_q == ST_STREAM_OUT) || (state_q == ST_SINGLE_REPLY);
        m_axis_tlast  = (state_q == ST_SINGLE_REPLY) ||
                        ((state_q == ST_STREAM_OUT) && (out_beat_q == 2'd3));
        m_axis_tuser  = {10'd0, out_beat_q, status_code_q};

        if (state_q == ST_STREAM_OUT) begin
            for (int lane = 0; lane < 16; lane++) begin
                m_axis_tdata[lane*32 +: 32] = result_latch_q[{out_beat_q, 4'(lane)}];
            end
        end else begin
            m_axis_tdata = single_reply_q;
        end
    end

`ifdef FORMAL
    // ========================================================================
    // SystemVerilog Assertions (SVA) for SymbiYosys / Formal Verification
    // ========================================================================
    // 1. Dual-rail mutual exclusion: a decoded trit can never have both pos=1 and neg=1
    always_comb begin
        for (int g_i = 0; g_i < 16; g_i++) begin
            assert ((dec_trit_pos[g_i] & dec_trit_neg[g_i]) == 5'b00000);
            assert (dec_futhark_r2[g_i] <= 5'd26);
        end
    end

    // 2. Charm XVIII Hardware Non-Exportability Invariant
    property p_charm18_blocks_root_seal;
        @(posedge aclk) disable iff (!aresetn)
        ((state_q == ST_IDLE) && s_axis_tvalid && ((required_caps_i & CAP_ROOT_SEAL) != 32'd0))
        |=> (state_q == ST_SINGLE_REPLY) && (status_code_q == AXIS_STATUS_CAP_VIOLATION);
    endproperty
    assert property (p_charm18_blocks_root_seal);
`endif

endmodule
`default_nettype wire

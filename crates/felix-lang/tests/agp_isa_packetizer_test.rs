#[test]
fn test_agp_isa_64bit_bytecode_packetizer_and_streaming_execution() {
    use felix_lang::{
        agp_isa_packetizer::{
            AgpCapabilityGraph, AgpCapabilityNode, AgpInstruction, AgpStreamPacketizer,
            AGP_FLAG_STANDARD, AGP_FLAG_TERMINAL, AGP_OP_CAP_ASSERT, AGP_OP_LD_WEIGHT,
            AGP_OP_SYNC_BARRIER, AGP_OP_TGEMM_PQ, AGP_OP_TGEMM_RAW,
        },
        felix_phoenix::{felix_ternary_gemm_i32, CAP_ARENA, CAP_IO, CAP_M8},
        fpga_axis_ternary::FpgaTernary64x64AxisCore,
        ljodatal_18_charms::CAP_ROOT_SEAL,
    };

    // 1. Verify 64-bit instruction bitfield layout
    let insn = AgpInstruction::encode(
        AGP_OP_TGEMM_PQ,
        AGP_FLAG_TERMINAL,
        true,  // rok_invert
        true,  // bend_enable
        12,    // dst
        5,     // src1
        7,     // src2
        0x5A5A,// imm20
        0x1234,// cap_token
    );

    assert_eq!(insn.opcode(), AGP_OP_TGEMM_PQ);
    assert_eq!(insn.flag(), AGP_FLAG_TERMINAL);
    assert!(insn.rok_invert());
    assert!(insn.bend_enable());
    assert_eq!(insn.dst(), 12);
    assert_eq!(insn.src1(), 5);
    assert_eq!(insn.src2(), 7);
    assert_eq!(insn.imm20(), 0x5A5A);
    assert_eq!(insn.cap_token(), 0x1234);

    // 2. Build a DAG representation of a FELIX capability compute wavefront
    let node0_init = AgpCapabilityNode {
        node_id: 0,
        name: "node0_load_and_cap_gate".into(),
        cap_mask: CAP_IO | CAP_ARENA,
        dependencies: vec![],
        instructions: vec![
            AgpInstruction::encode(AGP_OP_CAP_ASSERT, AGP_FLAG_STANDARD, false, false, 0, 0, 0, 0, (CAP_IO | CAP_ARENA) as u16),
            // Load 64 weight rows
            AgpInstruction::encode(AGP_OP_LD_WEIGHT, AGP_FLAG_STANDARD, false, true, 0, 1, 2, 0, 0),
        ],
    };

    let node1_compute = AgpCapabilityNode {
        node_id: 1,
        name: "node1_tgemm_contraction".into(),
        cap_mask: CAP_ARENA | CAP_M8,
        dependencies: vec![0],
        instructions: vec![
            AgpInstruction::encode(AGP_OP_SYNC_BARRIER, AGP_FLAG_STANDARD, false, false, 0, 0, 0, 1, 0), // Depends on node 0
            AgpInstruction::encode(AGP_OP_TGEMM_RAW, AGP_FLAG_TERMINAL, false, false, 0, 0, 0, 0, 0),
        ],
    };

    let graph = AgpCapabilityGraph {
        graph_id: 0xCAFE_BABE,
        nodes: vec![node0_init, node1_compute],
    };

    let granted = CAP_IO | CAP_ARENA | CAP_M8;
    let mut packetizer = AgpStreamPacketizer::new(graph, granted);

    let mut weights = [[0i8; 64]; 64];
    for r in 0..64 {
        for c in 0..64 {
            weights[r][c] = ((r * 11 + c * 17) % 3) as i8 - 1;
        }
    }
    let mut acts = [0i16; 64];
    for c in 0..64 {
        acts[c] = ((c as i16) * 123) - 500;
    }
    let futhark_sample = [0x12u8; 64];

    // 3. Packetize and stream execution into the hardware core
    let mut core = FpgaTernary64x64AxisCore::new(granted, CAP_IO | CAP_ARENA);
    let results = packetizer
        .execute_graph_pipeline(&mut core, &weights, &acts, &futhark_sample)
        .expect("AGP graph pipeline streaming should succeed");

    assert_eq!(results.len(), 1);
    let raw_gemm = &results[0];

    // Compare with software truth
    let sw_expected_row0 = felix_ternary_gemm_i32(&acts, &weights[0]);
    assert_eq!(raw_gemm[0], sw_expected_row0);

    // 4. Test Security Rejection: DAG node asserting Charm XVIII (CAP_ROOT_SEAL)
    let rogue_node = AgpCapabilityNode {
        node_id: 2,
        name: "node2_rogue_root_seal".into(),
        cap_mask: CAP_ROOT_SEAL,
        dependencies: vec![],
        instructions: vec![
            AgpInstruction::encode(AGP_OP_TGEMM_RAW, AGP_FLAG_TERMINAL, false, false, 0, 0, 0, 0, 0),
        ],
    };

    let rogue_graph = AgpCapabilityGraph {
        graph_id: 0xDEAD,
        nodes: vec![rogue_node],
    };

    let mut rogue_packetizer = AgpStreamPacketizer::new(rogue_graph, granted | CAP_ROOT_SEAL);
    let err = rogue_packetizer.packetize_node(2, &Some(weights), &Some(acts), &Some(futhark_sample));
    assert!(err.is_err(), "Packetizer must reject nodes specifying CAP_ROOT_SEAL");
}

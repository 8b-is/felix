#!/bin/bash
# Benchmark runner wrapper using felix binary

set -euo pipefail

FELIX_BIN="./obj/felix"
ITERATIONS="${1:-100000}"
WARMUP="${2:-10000}"

if [ ! -x "$FELIX_BIN" ]; then
    echo "Error: FELIX binary not found at $FELIX_BIN"
    echo "Build first: gprbuild -P felix.gpr"
    exit 1
fi

echo "=== FELIX Benchmark Suite ==="
echo "Iterations: $ITERATIONS"
echo "Warmup: $WARMUP"
echo

# Run benchmarks via felix binary
"$FELIX_BIN" -e "
    let config : Benchmark_Config := (Iterations => $ITERATIONS, Warmup_Runs => $WARMUP);
    let results : Benchmark_Results := Run_All_Benchmarks(config);
    Print_Results(results);
    Print_Results_MD(results);
"

echo
echo "=== Benchmark Suite Complete ==="
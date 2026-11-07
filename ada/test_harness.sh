# FELIX Test Harness
#
# Run all examples and verify outputs match goldens

#!/bin/bash
set -euo pipefail

FELIX_BIN="${1:-./obj/felix}"

if [ ! -x "$FELIX_BIN" ]; then
    echo "Error: FELIX binary not found at $FELIX_BIN"
    echo "Build first: gprbuild -P felix.gpr"
    exit 1
fi

echo "=== FELIX Test Suite ==="
echo "Binary: $FELIX_BIN"
echo

run_test() {
    local name="$1"
    local input="$2"
    local expected_output="$3"
    local expected_exit="${4:-0}"

    echo "Testing: $name"
    local output
    set +e
    output="$("$FELIX_BIN" -e "$input" 2>&1)"
    local exit_code=$?
    set -e

    if [ "$exit_code" -ne "$expected_exit" ]; then
        echo "  FAIL: exit code $exit_code (expected $expected_exit)"
        echo "  Output: $output"
        return 1
    fi

    if [ "$output" != "$expected_output" ]; then
        echo "  FAIL: output mismatch"
        echo "  Expected: $expected_output"
        echo "  Got:      $output"
        return 1
    fi

    echo "  PASS"
    return 0
}

# Test 1: Basic arithmetic
run_test "arithmetic" \
    'print(1 + 2 * 3);' \
    "7" \
    0

# Test 2: Exact p/q
run_test "exact_ratio" \
    'let r = 22/7; print(r);' \
    "22/7" \
    0

# Test 3: Linear let + consume
run_test "linear_consume" \
    'linear let x = 42; consume(x); print("ok");' \
    "ok" \
    0

# Test 3: Ternary NAND truth table
run_test "ternary_nand" \
    'print(-min(1, 1)); print(-min(1, 0)); print(-min(0, -1));' \
    "-1
0
1" \
    0

echo
echo "=== All tests completed ==="
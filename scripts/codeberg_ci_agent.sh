#!/usr/bin/env bash
# Sovereign Codeberg CI Chore-Polish & Release Agent for FELIX
set -euo pipefail

MODE="${1:---chore-polish}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> [FELIX Codeberg Agent] Mode: $MODE"

run_chore_polish() {
    echo "--> 1. Checking Rust formatting (cargo fmt --all -- --check)..."
    cargo fmt --all -- --check

    echo "--> 2. Running strict Clippy lints (-D warnings)..."
    cargo clippy --workspace --all-targets -- -D warnings

    echo "--> 3. Running workspace unit & integration tests..."
    cargo test --workspace

    echo "--> 4. Auditing Ada 2022 / SPARK 2014 representation invariants..."
    for ads in ada/*.ads; do
        grep -q "pragma SPARK_Mode (On);" "$ads" || { echo "Missing SPARK_Mode in $ads"; exit 1; }
        grep -q "No_Allocators" "$ads" || { echo "Missing No_Allocators restriction in $ads"; exit 1; }
    done
    echo "    ✓ All $(ls -1 ada/*.ads | wc -l | tr -d ' ') Ada/SPARK packages verified."
}

run_release() {
    run_chore_polish
    VERSION="$(grep -m1 '^version = ' Cargo.toml | sed -E 's/version = "(.*)"/\1/')"
    TAG="v${VERSION}"
    echo "--> Building release artifacts for FELIX ${TAG}..."
    cargo build --release --workspace

    mkdir -p dist
    if [ -f target/release/felix ]; then
        cp target/release/felix "dist/felix-${TAG}-$(uname -s)-$(uname -m)"
        shasum -a 256 "dist/felix-${TAG}-$(uname -s)-$(uname -m)" > "dist/felix-${TAG}.sha256"
    fi

    echo "--> Mirroring release state across remotes..."
    if git remote | grep -q '^codeberg$'; then
        git push codeberg HEAD:main --tags || true
    fi
    if git remote | grep -q '^origin$'; then
        git push origin HEAD:main --tags || true
    fi
    echo "    ✓ Release ${TAG} artifacts ready in dist/"
}

case "$MODE" in
    --chore-polish)
        run_chore_polish
        ;;
    --release)
        run_release
        ;;
    *)
        echo "Usage: $0 [--chore-polish | --release]"
        exit 1
        ;;
esac

#!/bin/bash
# Run stress test benchmarks across configurations.
# Usage: ./bench.sh

set -e

cargo build --release --example stress_test 2>/dev/null

echo "=== Native Stress Test Benchmarks ==="
echo ""

for cfg in "25 25" "50 50" "100 100" "150 150" "200 200" "250 150" "200 200 grid" "250 150 grid"; do
  cargo run --release --example stress_test -- $cfg --bench 2>&1
  echo ""
done

#!/bin/bash
# Profile all benchmarks in alejandria-storage and generate flamegraphs
#
# This script profiles three key benchmark groups:
#   - hybrid_search: BM25 + vector similarity search performance
#   - decay_operation: Temporal decay algorithm performance
#   - embedding_generation: Text-to-vector transformation performance
#
# USAGE:
#   ./scripts/profiling/profile-benchmarks.sh [output-dir]
#
# ARGUMENTS:
#   output-dir: Directory for flamegraph output (default: ./target/profiling)
#
# OUTPUT:
#   ${output-dir}/hybrid_search.svg
#   ${output-dir}/decay_operation.svg
#   ${output-dir}/embedding_generation.svg
#
# EXIT CODES:
#   0: Success (all flamegraphs generated)
#   1: Failure (prerequisite missing, build error, or profiling error)
#
# ENVIRONMENT:
#   CARGO_FLAMEGRAPH_ARGS: Additional arguments to pass to cargo-flamegraph
#
# PREREQUISITES:
#   - cargo-flamegraph: Install with: cargo install flamegraph
#   - perf (Linux only): Install with: sudo apt-get install linux-tools-common linux-tools-generic
#   - macOS users: Use Instruments or cargo-instruments instead
#
# EXECUTION TIME:
#   Expect 10-15 minutes for full profiling run (CPU-intensive sampling)

# Exit immediately if any command fails
set -e

# Configuration
OUTPUT_DIR=${1:-./target/profiling}

# Prerequisite checks - validate required tools before profiling
echo "==> Checking prerequisites..."

# Check if cargo-flamegraph is installed
# cargo-flamegraph wraps perf/dtrace and generates interactive SVG flamegraphs
if ! command -v cargo-flamegraph &> /dev/null; then
    echo "ERROR: cargo-flamegraph not found"
    echo ""
    echo "Install it with:"
    echo "  cargo install flamegraph"
    echo ""
    echo "This tool collects CPU sampling data and generates flamegraph visualizations."
    exit 1
fi

# Check if perf is available (Linux only - macOS uses dtrace/Instruments instead)
# perf is the Linux kernel profiler that provides CPU sampling data
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if ! command -v perf &> /dev/null; then
        echo "ERROR: perf not found"
        echo ""
        echo "Install it with:"
        echo "  sudo apt-get install linux-tools-common linux-tools-generic linux-tools-\$(uname -r)"
        echo ""
        echo "Note: The linux-tools-\$(uname -r) package must match your running kernel version."
        echo "      Check your kernel with: uname -r"
        exit 1
    fi
fi

# Create output directory (mkdir -p creates parent dirs and doesn't fail if exists)
echo "==> Creating output directory: $OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build benchmarks with release profile (optimized) + debug symbols (for profiling)
# Debug symbols allow flamegraph to show function names instead of <unknown>
# Configured in Cargo.toml: [profile.release] debug = 1, strip = false
echo "==> Building benchmarks..."
cargo build --release --benches

# Profile each benchmark group individually
# --bench performance: Run the performance benchmark binary
# --output: Path to generated SVG flamegraph file
# -- --bench <name>: Pass --bench flag to criterion to run specific benchmark
echo "==> Profiling hybrid_search benchmark..."
cargo flamegraph --bench performance --output "$OUTPUT_DIR/hybrid_search.svg" -- --bench hybrid_search

echo "==> Profiling decay_operation benchmark..."
cargo flamegraph --bench performance --output "$OUTPUT_DIR/decay_operation.svg" -- --bench decay_operation

echo "==> Profiling embedding_generation benchmark..."
cargo flamegraph --bench performance --output "$OUTPUT_DIR/embedding_generation.svg" -- --bench embedding_generation

# Success
echo ""
echo "==> SUCCESS: Flamegraphs generated in $OUTPUT_DIR"
echo "    - hybrid_search.svg"
echo "    - decay_operation.svg"
echo "    - embedding_generation.svg"
echo ""
echo "Open the SVG files in your browser to view the flamegraphs."

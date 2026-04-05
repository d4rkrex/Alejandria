#!/bin/bash
# Profile a single benchmark with optional custom arguments
#
# This script provides fine-grained profiling control, allowing you to:
#   - Profile a specific benchmark in isolation
#   - Pass custom criterion arguments (e.g., --sample-size, --measurement-time)
#   - Generate a single flamegraph without running all benchmarks
#
# USAGE:
#   ./scripts/profiling/profile-single.sh <benchmark-name> [args...]
#
# ARGUMENTS:
#   benchmark-name: Name of benchmark to profile (hybrid_search, decay_operation, embedding_generation)
#   args: Additional arguments passed directly to criterion benchmark
#
# OUTPUT:
#   ./target/profiling/${benchmark-name}.svg
#
# EXIT CODES:
#   0: Success
#   1: Failure (missing benchmark name, prerequisite missing, or profiling error)
#
# EXAMPLES:
#   # Profile hybrid_search with default criterion settings
#   ./profile-single.sh hybrid_search
#
#   # Profile with reduced sample size (faster, less precise)
#   ./profile-single.sh decay_operation --sample-size 5
#
#   # Profile with increased measurement time (slower, more precise)
#   ./profile-single.sh embedding_generation --measurement-time 60

# Exit immediately if any command fails
set -e

# Configuration
BENCHMARK=$1
OUTPUT_DIR=./target/profiling

# Validate required argument
if [ -z "$BENCHMARK" ]; then
    echo "ERROR: Benchmark name is required"
    echo ""
    echo "USAGE: ./scripts/profiling/profile-single.sh <benchmark-name> [args...]"
    echo ""
    echo "Available benchmarks:"
    echo "  - hybrid_search        : BM25 + vector similarity search"
    echo "  - decay_operation      : Temporal decay algorithm"
    echo "  - embedding_generation : Text-to-vector transformation"
    echo ""
    echo "Examples:"
    echo "  ./scripts/profiling/profile-single.sh hybrid_search"
    echo "  ./scripts/profiling/profile-single.sh decay_operation --sample-size 5"
    exit 1
fi

# Shift off the benchmark name to get remaining args for criterion
# After this, $@ contains only the additional arguments
shift

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

# Profile the specified benchmark
# $@ expands to all remaining arguments after shift (criterion flags like --sample-size)
echo "==> Profiling $BENCHMARK benchmark with args: $@"
cargo flamegraph --bench performance --output "$OUTPUT_DIR/${BENCHMARK}.svg" -- --bench "$BENCHMARK" "$@"

# Success
echo ""
echo "==> SUCCESS: Flamegraph generated at $OUTPUT_DIR/${BENCHMARK}.svg"
echo ""
echo "Open the SVG file in your browser to view the flamegraph."

#!/bin/bash
# Compare profiling results between two git refs
#
# This script automates performance regression detection by:
#   - Checking out two different git refs (branches, commits, tags)
#   - Profiling all benchmarks at each ref
#   - Generating side-by-side flamegraphs for visual comparison
#
# SAFETY FEATURES:
#   - Warns if uncommitted changes exist (data loss risk)
#   - Stores original git ref before any checkouts
#   - Uses EXIT trap to restore original ref even on errors/interrupts
#   - Prompts for confirmation before proceeding
#
# USAGE:
#   ./scripts/profiling/compare-profiles.sh [base-ref] [compare-ref]
#
# ARGUMENTS:
#   base-ref: Git ref for baseline profiling (default: main)
#   compare-ref: Git ref for comparison profiling (default: HEAD)
#
# OUTPUT:
#   ./target/profiling/base/hybrid_search.svg (and other benchmarks)
#   ./target/profiling/compare/hybrid_search.svg (and other benchmarks)
#
# EXIT CODES:
#   0: Success (both refs profiled)
#   1: Failure (git error, uncommitted changes abort, or profiling error)
#
# EXAMPLES:
#   # Compare current branch to main
#   ./compare-profiles.sh main HEAD
#
#   # Compare two feature branches
#   ./compare-profiles.sh feature-a feature-b
#
#   # Compare commit to previous commit
#   ./compare-profiles.sh HEAD~1 HEAD
#
# EXECUTION TIME:
#   Expect 20-30 minutes (profiles twice: base + compare)

# Exit immediately if any command fails
# Trap handler (below) ensures git ref is restored even on error
set -e

# Configuration
BASE_REF=${1:-main}
COMPARE_REF=${2:-HEAD}

# Store current git ref for restoration after profiling
# --abbrev-ref returns branch name instead of commit SHA (more user-friendly)
ORIGINAL_REF=$(git rev-parse --abbrev-ref HEAD)

# Trap handler ensures original git ref is restored on exit
# Fires on: normal exit (exit 0), errors (set -e), signals (Ctrl+C, kill)
# This prevents leaving the repository in a detached HEAD or wrong branch state
restore_ref() {
    echo ""
    echo "==> Restoring original ref: $ORIGINAL_REF"
    # Redirect output to avoid noise when everything works correctly
    # Errors are still shown if checkout fails
    git checkout "$ORIGINAL_REF" > /dev/null 2>&1
}

# Register the trap handler for EXIT signal
# EXIT is triggered by: explicit exit, script end, errors (with set -e), or interrupts
trap restore_ref EXIT

# Safety check: warn about uncommitted changes
# git diff-index checks working tree against HEAD for changes
# --quiet exits with code 1 if changes exist (inverted by !)
if ! git diff-index --quiet HEAD --; then
    echo "WARNING: You have uncommitted changes in your working tree."
    echo "         These changes may be lost if the checkout fails."
    echo ""
    echo "Recommendation: Commit or stash your changes before profiling comparison."
    echo ""
    # Prompt for confirmation (single character input)
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    # Check if response is Y or y (case insensitive via regex)
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted by user."
        exit 1
    fi
fi

echo "==> Profiling comparison: $BASE_REF vs $COMPARE_REF"
echo ""

# Profile base ref (baseline for comparison)
echo "==> Profiling base ref: $BASE_REF"
git checkout "$BASE_REF"
# Call profile-benchmarks.sh with base/ subdirectory for organization
./scripts/profiling/profile-benchmarks.sh ./target/profiling/base

echo ""

# Profile compare ref (the version being tested)
echo "==> Profiling compare ref: $COMPARE_REF"
git checkout "$COMPARE_REF"
# Call profile-benchmarks.sh with compare/ subdirectory for organization
./scripts/profiling/profile-benchmarks.sh ./target/profiling/compare

echo ""
echo "==> SUCCESS: Comparison complete"
echo ""
echo "Baseline flamegraphs:   ./target/profiling/base/"
echo "Comparison flamegraphs: ./target/profiling/compare/"
echo ""
echo "Compare the flamegraphs side-by-side in your browser:"
echo "  - Base:    file://$(pwd)/target/profiling/base/hybrid_search.svg"
echo "  - Compare: file://$(pwd)/target/profiling/compare/hybrid_search.svg"
echo ""
echo "Look for differences in:"
echo "  - Stack depth (deeper = more function calls)"
echo "  - Bar width (wider = more time spent)"
echo "  - Hot paths (red/orange sections = CPU-intensive code)"

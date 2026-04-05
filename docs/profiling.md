# Performance Profiling Guide

Comprehensive guide to profiling Alejandria's performance using cargo-flamegraph and interpreting the results.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Profiling Scripts](#profiling-scripts)
- [Reading Flamegraphs](#reading-flamegraphs)
- [CI Integration](#ci-integration)
- [Common Workflows](#common-workflows)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)
- [References](#references)

## Overview

Profiling is the practice of measuring where your program spends its execution time. While benchmarks tell you *how long* code takes to run, profiling tells you *where* that time is spent—identifying bottlenecks and optimization opportunities.

### What is Profiling Useful For?

- **Performance Investigation**: Identify slow operations in hybrid search, decay algorithms, or embedding generation
- **Optimization Validation**: Verify that code changes actually improve performance where expected
- **Understanding Code Behavior**: Visualize call stacks and execution flow through complex operations
- **Regression Detection**: Compare performance characteristics across git commits

### Tools We Use

Alejandria's profiling infrastructure uses:

- **cargo-flamegraph**: Rust tooling for generating flamegraph visualizations
- **perf**: Linux performance profiler that collects CPU sampling data
- **Criterion**: Existing benchmark suite that profiling tools wrap around

## Prerequisites

### Installing cargo-flamegraph

```bash
cargo install flamegraph
```

This installs the `cargo-flamegraph` subcommand globally. You only need to do this once per development machine.

### Platform-Specific Requirements

#### Linux (Recommended)

Install `perf`, the Linux performance profiler:

```bash
# Ubuntu/Debian
sudo apt-get install linux-tools-common linux-tools-generic linux-tools-$(uname -r)

# Fedora/RHEL
sudo dnf install perf

# Arch Linux
sudo pacman -S perf
```

**Note**: The `linux-tools-$(uname -r)` package is critical—it ensures you get the perf binary matching your running kernel version.

#### macOS (Limited Support)

macOS does not support `perf`. Instead, use native profiling tools:

- **Instruments**: Apple's profiling tool (comes with Xcode)
- **cargo-instruments**: Rust wrapper for Instruments (`cargo install cargo-instruments`)

The scripts in `scripts/profiling/` are designed for Linux and will not work on macOS without modification.

#### Windows (Not Supported)

Windows profiling requires different tooling (ETW, Windows Performance Analyzer). The scripts in this repository target Linux environments. Consider using WSL2 with a Linux distribution for profiling on Windows machines.

## Quick Start

**Time to first flamegraph: 5 minutes**

### 1. Install Prerequisites

```bash
# Install cargo-flamegraph (one-time setup)
cargo install flamegraph

# Install perf on Linux (one-time setup)
sudo apt-get install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
```

### 2. Run Profiling

```bash
# Profile all benchmarks
./scripts/profiling/profile-benchmarks.sh
```

This generates three flamegraph SVG files in `target/profiling/`:
- `hybrid_search.svg`
- `decay_operation.svg`
- `embedding_generation.svg`

### 3. View Results

Open any SVG file in your browser:

```bash
# Linux
xdg-open target/profiling/hybrid_search.svg

# macOS
open target/profiling/hybrid_search.svg

# Or just drag the file into your browser
```

The flamegraph is interactive—click on any stack frame to zoom in.

## Profiling Scripts

Alejandria provides three scripts for common profiling workflows.

### profile-benchmarks.sh

**Purpose**: Profile all criterion benchmarks and generate flamegraphs for each.

**Usage**:
```bash
./scripts/profiling/profile-benchmarks.sh [output-dir]
```

**Arguments**:
- `output-dir` (optional): Directory for flamegraph output. Default: `./target/profiling`

**Output**:
- `${output-dir}/hybrid_search.svg` - Flamegraph for hybrid search operations
- `${output-dir}/decay_operation.svg` - Flamegraph for memory decay operations
- `${output-dir}/embedding_generation.svg` - Flamegraph for embedding generation

**Example**:
```bash
# Profile to default location (./target/profiling)
./scripts/profiling/profile-benchmarks.sh

# Profile to custom directory
./scripts/profiling/profile-benchmarks.sh ./analysis/baseline-profile
```

**Environment Variables**:
- `CARGO_FLAMEGRAPH_ARGS`: Additional arguments to pass to cargo-flamegraph

**When to Use**: 
- Comprehensive profiling of all major operations
- Establishing performance baselines
- Continuous integration (CI automatically uses this script)

### profile-single.sh

**Purpose**: Profile a single benchmark with custom arguments for focused investigation.

**Usage**:
```bash
./scripts/profiling/profile-single.sh <benchmark-name> [args...]
```

**Arguments**:
- `benchmark-name` (required): Name of benchmark to profile
  - `hybrid_search` - Hybrid vector + FTS5 search operations
  - `decay_operation` - Memory decay algorithm execution
  - `embedding_generation` - Text-to-vector embedding creation
- `args...` (optional): Additional arguments passed to the benchmark

**Output**:
- `./target/profiling/${benchmark-name}.svg`

**Examples**:
```bash
# Profile hybrid search with default settings
./scripts/profiling/profile-single.sh hybrid_search

# Profile decay operation with custom sample size
./scripts/profiling/profile-single.sh decay_operation --sample-size 5

# Profile embedding generation with specific parameters
./scripts/profiling/profile-single.sh embedding_generation --measurement-time 30
```

**When to Use**:
- Iterative optimization of a specific operation
- Testing parameter variations without profiling everything
- Debugging performance issues in isolated benchmarks

### compare-profiles.sh

**Purpose**: Compare profiling results between two git refs (branches, commits, tags) to identify performance changes.

**Usage**:
```bash
./scripts/profiling/compare-profiles.sh [base-ref] [compare-ref]
```

**Arguments**:
- `base-ref` (optional): Git ref for baseline profiling. Default: `main`
- `compare-ref` (optional): Git ref for comparison profiling. Default: `HEAD`

**Output**:
- `./target/profiling/base/` - Flamegraphs for base-ref
- `./target/profiling/compare/` - Flamegraphs for compare-ref

**Examples**:
```bash
# Compare current changes against main branch
./scripts/profiling/compare-profiles.sh main HEAD

# Compare two specific branches
./scripts/profiling/compare-profiles.sh feature/old-optimization feature/new-optimization

# Compare against a specific commit
./scripts/profiling/compare-profiles.sh abc123def main
```

**Important Notes**:
- ⚠️ **This script performs git checkouts**. Commit or stash your changes first!
- The script will prompt for confirmation if you have uncommitted changes
- Original branch/ref is automatically restored after completion (via trap handler)
- If script is interrupted (Ctrl+C), the trap handler still restores your original ref

**When to Use**:
- Before/after optimization work to validate improvements
- Investigating performance regressions between commits
- Comparing optimization strategies on different branches
- Reviewing performance impact during code review

## Reading Flamegraphs

Flamegraphs are interactive SVG visualizations that show where CPU time is spent in your program.

### Anatomy of a Flamegraph

```
┌─────────────────────────────────────────┐ ← Top of stack (leaf functions)
│     SQLite FTS5 Query Processing        │   Widest boxes = most time spent
├─────────────────────────────────────────┤
│           search_hybrid                  │ ← Middle of stack (caller functions)
├─────────────────────────────────────────┤
│              main                        │ ← Bottom of stack (entry point)
└─────────────────────────────────────────┘
```

### Key Concepts

**X-Axis (Width)**:
- Represents the proportion of CPU time consumed
- **Width = time spent in that function and all functions it calls**
- Boxes are ordered alphabetically (NOT left-to-right by time)
- A box taking 50% of the width means 50% of total CPU time went through that function

**Y-Axis (Height)**:
- Represents call stack depth
- Bottom = entry point (e.g., `main`, `benchmark_runner`)
- Top = leaf functions (actual work being done)
- Each level = one function call deeper in the stack

**Color**:
- Typically random (differentiates adjacent boxes)
- No semantic meaning in default cargo-flamegraph output
- Some color schemes use warm colors (red/orange) for hot functions, cool colors (blue/green) for cold functions

### Interactive Features

**Click to Zoom**:
- Click any box to zoom into that subtree
- The clicked function becomes the new "root" of the view
- "Reset zoom" appears at the top—click to return to full view

**Hover for Details**:
- Hover over any box to see:
  - Full function name (including crate and module path)
  - Sample count (number of times function was seen in profiler samples)
  - Percentage of total time

**Search**:
- Press Ctrl+F (or Cmd+F) in browser
- Enter function name or keyword
- Matching boxes highlight in purple

### Finding Bottlenecks

**Wide Rectangles at Top of Stack** = Hotspots:
- Look for wide boxes near the top of the flamegraph
- These are leaf functions doing substantial work
- Prime candidates for optimization

**Example Hotspots in Alejandria**:
```
Wide box: "sqlite3_fts5_query_process" (30% width)
↳ Indicates SQLite FTS5 query processing is a bottleneck
↳ Optimization: Reduce query complexity or cache results

Wide box: "cosine_similarity_batch" (20% width)
↳ Vector similarity calculation consuming significant time
↳ Optimization: Use SIMD instructions or optimize algorithm

Wide box: "serde_json::from_str" (15% width)
↳ JSON deserialization overhead
↳ Optimization: Use faster serialization format or reduce parsing
```

**Narrow, Tall Towers** = Deep Call Stacks:
- Many layers of function calls with little actual work
- May indicate excessive abstraction or recursion
- Not necessarily a problem, but worth investigating if towers are wide

### Common Patterns

**Flat Profile** (wide, short boxes):
- Time distributed across many functions
- No single obvious bottleneck
- May indicate well-optimized code or need for higher-level algorithm improvements

**Tall Spike** (narrow, tall tower):
- Deep function call hierarchy with little work
- Often abstraction overhead or setup code
- Usually not a performance concern unless the spike is wide

**Single Dominant Function** (one very wide box):
- Most time spent in one place
- Clear optimization target
- Common in I/O-bound operations (database queries, network requests)

### What to Look For in Alejandria Profiles

**Hybrid Search (`hybrid_search.svg`)**:
- **Expected hotspots**:
  - `sqlite3_*` functions (FTS5 full-text search)
  - `cosine_similarity` or `dot_product` (vector similarity)
  - `merge_results` (combining FTS5 + vector results)
- **Red flags**:
  - Excessive time in JSON serialization/deserialization
  - Repeated database connection/query overhead
  - Inefficient vector operations (non-SIMD)

**Decay Operations (`decay_operation.svg`)**:
- **Expected hotspots**:
  - `sqlite3_step` (iterating through memories)
  - Decay algorithm implementation (exponential, linear, etc.)
  - Batch update operations
- **Red flags**:
  - Individual row updates instead of batch operations
  - Unnecessary computations per memory
  - Lock contention in concurrent scenarios

**Embedding Generation (`embedding_generation.svg`)**:
- **Expected hotspots**:
  - Tokenization (breaking text into tokens)
  - Model inference (if using local embeddings)
  - Network I/O (if using remote embedding API)
- **Red flags**:
  - Redundant encoding operations
  - Inefficient string allocations
  - Synchronous API calls that could be batched

## CI Integration

Alejandria's continuous integration automatically generates flamegraphs for every commit and pull request, providing continuous visibility into performance characteristics.

### How It Works

The `profiling` job in `.github/workflows/ci.yml`:

1. **Triggers**: Runs on every push to `main`/`develop` branches and every pull request
2. **Platform**: Runs on `ubuntu-latest` (Linux-only, perf requirement)
3. **Process**:
   - Installs cargo-flamegraph and perf
   - Executes `./scripts/profiling/profile-benchmarks.sh`
   - Generates flamegraphs for all three benchmarks
4. **Artifacts**: Uploads flamegraphs with 30-day retention
5. **Failure Policy**: Job fails if profiling execution fails (script errors, build failures), but does NOT fail if performance degrades (performance changes are informational, not blocking)

### Accessing Flamegraphs from CI

**Step 1: Navigate to Actions**
- Go to your repository on GitHub
- Click the "Actions" tab
- Select the workflow run you're interested in (click on the commit message or PR title)

**Step 2: Download Artifacts**
- Scroll to the "Artifacts" section at the bottom of the workflow run page
- Click on `profiling-flamegraphs-{run_id}` to download a ZIP file
- The artifact name includes the GitHub run ID for traceability

**Step 3: Extract and View**
- Unzip the downloaded file
- Open any `.svg` file in your browser
- Compare flamegraphs across different commits/PRs by downloading multiple artifacts

**Artifact Details**:
- **Retention Period**: 30 days from workflow run
- **Size**: Typically 1-5 MB per artifact (3 SVG files)
- **Always Uploaded**: Even if profiling fails, artifacts are uploaded (via `if: always()`) to aid debugging

### Use Cases

**Performance Regression Detection**:
```
1. Download flamegraphs from main branch (before your changes)
2. Download flamegraphs from PR branch (after your changes)
3. Compare side-by-side in browser tabs
4. Look for new/wider hotspots that didn't exist before
```

**Before/After Optimization**:
```
1. Commit baseline code, wait for CI to run
2. Download baseline flamegraphs
3. Commit optimization, wait for CI to run
4. Download optimized flamegraphs
5. Verify that target hotspot is reduced in the optimized version
```

**Continuous Monitoring**:
```
1. Download flamegraphs from periodic main branch commits
2. Track growth of specific functions over time
3. Identify gradual performance degradation
```

### Limitations

- **Absolute Timing Variability**: CI runners are shared infrastructure—absolute execution times vary between runs due to CPU contention
- **Non-Deterministic Noise**: Flamegraphs may show slight variations even for identical code
- **Focus on Relative Time**: Use flamegraphs to identify which functions consume the most time *within a single run*, not to compare absolute timings across runs
- **No Automated Regression Detection**: Current setup provides flamegraphs for manual review—automated performance regression detection (criterion baselines, threshold checks) is not yet implemented

## Common Workflows

### Workflow 1: Profiling Before/After Optimization

**Scenario**: You've identified a slow operation and want to verify your optimization improves it.

**Steps**:
```bash
# 1. Profile baseline (before changes)
./scripts/profiling/profile-benchmarks.sh ./target/profiling/before

# 2. Open baseline flamegraph to identify hotspot
xdg-open target/profiling/before/hybrid_search.svg
# (Let's say you identify cosine_similarity as taking 30% of time)

# 3. Make your optimization changes
# (e.g., optimize vector similarity calculation)

# 4. Profile after changes
./scripts/profiling/profile-benchmarks.sh ./target/profiling/after

# 5. Compare before/after
xdg-open target/profiling/after/hybrid_search.svg
# (Check if cosine_similarity is now narrower/lower percentage)
```

**What to Look For**:
- Target function should be narrower in "after" flamegraph
- Surrounding functions may become wider (they now take proportionally more time)
- Total stack width remains 100%—optimization shifts proportions

### Workflow 2: Identifying Performance Regressions

**Scenario**: CI benchmarks show performance degradation, but you don't know why.

**Steps**:
```bash
# 1. Compare your branch against main
./scripts/profiling/compare-profiles.sh main your-feature-branch

# 2. Open both flamegraphs side-by-side
xdg-open target/profiling/base/hybrid_search.svg &
xdg-open target/profiling/compare/hybrid_search.svg &

# 3. Look for NEW wide boxes that weren't present in base
# 4. Look for WIDER existing boxes (regression in known hotspots)
```

**Common Regression Patterns**:
- New JSON serialization/deserialization calls
- Added debug logging in hot paths
- Inefficient data structure conversions
- Accidental synchronous I/O in tight loops

### Workflow 3: Understanding Query Performance

**Scenario**: Users report slow search queries, and you need to understand why.

**Steps**:
```bash
# 1. Profile hybrid search (includes query execution)
./scripts/profiling/profile-single.sh hybrid_search

# 2. Open flamegraph
xdg-open target/profiling/hybrid_search.svg

# 3. Search for SQLite-related functions
# (Press Ctrl+F, type "sqlite3", matching boxes highlight)

# 4. Identify which SQLite operations dominate
# Common hotspots:
#   - sqlite3_fts5_* (full-text search)
#   - sqlite3_step (row iteration)
#   - sqlite3_prepare (query compilation)

# 5. Search for vector similarity functions
# (Press Ctrl+F, type "cosine" or "similarity")

# 6. Compare relative time: FTS5 vs vector similarity
```

**Optimization Decision Tree**:
- If `sqlite3_fts5_*` dominates → Optimize FTS5 queries (simpler queries, indexing)
- If `cosine_similarity` dominates → Optimize vector operations (SIMD, caching)
- If `sqlite3_prepare` is wide → Query is recompiled too often (use prepared statements)

### Workflow 4: Profiling Embedding Generation

**Scenario**: Embedding generation feels slow—profile to find out why.

**Steps**:
```bash
# 1. Profile embedding generation
./scripts/profiling/profile-single.sh embedding_generation

# 2. Open flamegraph
xdg-open target/profiling/embedding_generation.svg

# 3. Look for tokenization overhead
# (Search for "tokenize", "split", "unicode")

# 4. Look for model inference time
# (Search for "infer", "forward", "predict")

# 5. Look for string/memory allocation overhead
# (Search for "alloc", "clone", "to_string")
```

**Optimization Targets**:
- **Tokenization**: Use zero-copy tokenization or reuse tokenizers
- **Model inference**: Batch multiple embeddings in one inference call
- **Allocations**: Reuse buffers, use string views instead of copies

### Workflow 5: Profiling Decay Operations

**Scenario**: Memory decay is taking longer than expected—profile to understand why.

**Steps**:
```bash
# 1. Profile decay operation
./scripts/profiling/profile-single.sh decay_operation

# 2. Open flamegraph
xdg-open target/profiling/decay_operation.svg

# 3. Look for database iteration overhead
# (Search for "sqlite3_step", "fetch", "query")

# 4. Look for decay computation time
# (Search for "decay", "exponential", "calculate")

# 5. Look for update batch efficiency
# (Search for "update", "execute", "commit")
```

**Optimization Strategies**:
- If `sqlite3_step` dominates → Too many small queries (batch operations)
- If decay computation is wide → Simplify algorithm or precompute values
- If individual updates are visible → Use batch UPDATE statements

## Troubleshooting

### Error: "cargo-flamegraph not found"

**Symptom**:
```
ERROR: cargo-flamegraph not found
Install with: cargo install flamegraph
```

**Solution**:
```bash
cargo install flamegraph
```

**Note**: This installs `cargo-flamegraph` globally. You only need to do this once per development machine.

---

### Error: "perf not found"

**Symptom** (Linux):
```
ERROR: perf not found
Install with: sudo apt-get install linux-tools-common linux-tools-generic
```

**Solution**:
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
```

**Critical**: The `linux-tools-$(uname -r)` package is essential—it provides the perf binary for your exact kernel version. Without it, perf may not work even if the other packages are installed.

**Verification**:
```bash
perf --version
# Should output: perf version 5.x.x
```

---

### Error: "Permission denied" or "perf_event_paranoid"

**Symptom** (Linux):
```
Error: Access to performance monitoring and observability operations is limited.
```

**Cause**: Linux kernel restricts perf access for security reasons. The `/proc/sys/kernel/perf_event_paranoid` setting controls access levels.

**Solution (Temporary - Current Session)**:
```bash
# Allow perf access without root (until reboot)
sudo sysctl kernel.perf_event_paranoid=-1
```

**Solution (Permanent)**:
```bash
# Edit sysctl config
sudo nano /etc/sysctl.conf

# Add this line:
kernel.perf_event_paranoid=-1

# Apply changes
sudo sysctl -p
```

**Security Note**: `perf_event_paranoid=-1` allows unprivileged perf access. Default is `2` (restricted). For development machines, `-1` is convenient. For production/shared systems, consider `1` (less permissive) or leave at default and use `sudo` with perf.

---

### Issue: Flamegraphs Show `<unknown>` Symbols

**Symptom**: Flamegraph has boxes labeled `<unknown>` or hex addresses like `0x7fff12345678` instead of function names.

**Cause**: Debug symbols are missing or stripped from the binary.

**Solution 1: Verify Cargo.toml Release Profile**:
Ensure your root `Cargo.toml` has:
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = false  # ← MUST be false
debug = 1      # ← MUST be set
```

**Solution 2: Rebuild with Debug Symbols**:
```bash
# Clean build to ensure symbols are included
cargo clean
cargo build --release --benches

# Re-run profiling
./scripts/profiling/profile-benchmarks.sh
```

**Solution 3: Check for Stripped Dependencies**:
Some crates may ship pre-compiled libraries without symbols. If specific dependencies show `<unknown>`, you may need to compile them from source:
```bash
# Force recompile of specific dependency with symbols
cargo clean -p problematic-crate
cargo build --release
```

---

### Issue: Empty or Incomplete Flamegraphs

**Symptom**: Flamegraph SVG is blank, very small, or shows only `main` function.

**Cause 1: Benchmark Runtime Too Short**:
Profiling requires sufficient samples. If a benchmark runs for < 1 second, the profiler may not collect enough data.

**Solution**:
```bash
# Increase criterion measurement time (default: 5s per benchmark)
./scripts/profiling/profile-single.sh hybrid_search --measurement-time 30
```

**Cause 2: Profiler Not Running**:
Check if perf actually collected data:
```bash
# Run cargo-flamegraph manually to see errors
cargo flamegraph --bench performance -- --bench hybrid_search
# Look for perf errors in output
```

**Cause 3: Benchmark Panics or Exits Early**:
If the benchmark crashes, the profiler may not generate output:
```bash
# Run benchmark without profiling to check for errors
cargo bench --bench performance -- hybrid_search
```

---

### Issue: macOS - "perf not found" or "dtrace not supported"

**Symptom**: Scripts fail on macOS because perf is Linux-only.

**Solution**: Use macOS-native profiling tools:

**Option 1: Instruments (Recommended)**:
1. Install Xcode from the Mac App Store
2. Open Instruments (Xcode → Open Developer Tool → Instruments)
3. Select "Time Profiler" template
4. Run your benchmark binary directly (build with `cargo build --release --benches`)
5. Instruments will show call stacks similar to flamegraphs

**Option 2: cargo-instruments**:
```bash
# Install cargo-instruments
cargo install cargo-instruments

# Profile with Instruments via cargo
cargo instruments --bench performance --template time
```

**Note**: The shell scripts in `scripts/profiling/` are Linux-centric. Consider creating macOS-specific variants if team members develop on macOS.

---

### Issue: Windows - Scripts Don't Work

**Symptom**: Bash scripts fail on Windows (even with Git Bash or WSL1).

**Solution**: Use WSL2 with a Linux distribution:
```powershell
# Install WSL2 (Windows 10/11)
wsl --install

# Inside WSL2, follow Linux setup instructions
sudo apt-get install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
cargo install flamegraph

# Clone repo inside WSL2 and run profiling there
cd /mnt/c/Projects/alejandria  # or clone to WSL filesystem for better performance
./scripts/profiling/profile-benchmarks.sh
```

**Alternative**: Windows Performance Analyzer (WPA) with Event Tracing for Windows (ETW)—advanced setup, not covered here.

---

### CI Job Fails: "No such file or directory" for perf

**Symptom**: CI profiling job fails with perf binary not found even though installation step succeeded.

**Cause**: GitHub Actions runner kernel version mismatch.

**Solution**: Verify `.github/workflows/ci.yml` includes kernel-specific tools:
```yaml
- name: Install perf
  run: sudo apt-get update && sudo apt-get install -y linux-tools-common linux-tools-generic linux-tools-$(uname -r)
```

The `linux-tools-$(uname -r)` package is critical—it provides perf for the exact running kernel.

---

### CI Artifacts Not Uploading

**Symptom**: Profiling job succeeds, but no artifacts appear in GitHub Actions.

**Cause**: Artifact upload step is conditional or has incorrect path.

**Solution**: Verify `.github/workflows/ci.yml`:
```yaml
- name: Upload profiling artifacts
  if: always()  # ← Ensure this is present (uploads even if profiling fails)
  uses: actions/upload-artifact@v4
  with:
    name: profiling-flamegraphs-${{ github.run_id }}
    path: target/profiling/  # ← Ensure path matches script output
    retention-days: 30
```

**Debug**:
1. Check workflow logs—look for "Upload profiling artifacts" step
2. Verify `target/profiling/` contains SVG files after profiling step
3. Check GitHub Actions storage quota (free tier: 500 MB)

## Advanced Usage

### Custom CARGO_FLAMEGRAPH_ARGS

You can pass additional arguments to cargo-flamegraph via environment variables:

```bash
# Increase perf sampling frequency (default: 99 Hz)
CARGO_FLAMEGRAPH_ARGS="--freq 999" ./scripts/profiling/profile-benchmarks.sh

# Use different perf event (e.g., cache misses instead of CPU cycles)
CARGO_FLAMEGRAPH_ARGS="--event cache-misses" ./scripts/profiling/profile-benchmarks.sh

# Combine multiple custom arguments
CARGO_FLAMEGRAPH_ARGS="--freq 499 --no-inline" ./scripts/profiling/profile-benchmarks.sh
```

**Common Options**:
- `--freq <HZ>`: Sampling frequency (higher = more detail, more overhead)
- `--no-inline`: Show inlined functions separately (useful for highly optimized code)
- `--event <EVENT>`: Profile different perf events (cpu-cycles, cache-misses, branch-misses)

### Profiling Non-Benchmark Code

To profile arbitrary binaries or tests (not just benchmarks):

```bash
# Profile the CLI binary
cargo flamegraph --bin alejandria -- recall "test query"

# Profile a specific test
cargo flamegraph --test integration_tests -- --test-threads=1

# Profile an example
cargo flamegraph --example custom_example
```

### Using perf Directly for More Control

For advanced use cases, bypass cargo-flamegraph and use perf + FlameGraph scripts directly:

```bash
# 1. Build release binary with symbols
cargo build --release --bin alejandria

# 2. Record perf data
perf record -F 99 -g --call-graph dwarf -- ./target/release/alejandria recall "query"

# 3. Generate perf script output
perf script > out.perf

# 4. Generate flamegraph (requires flamegraph.pl from FlameGraph repo)
git clone https://github.com/brendangregg/FlameGraph
./FlameGraph/flamegraph.pl out.perf > custom.svg
```

**When to Use Direct perf**:
- Profiling long-running services (not one-shot benchmarks)
- Custom perf events or sampling strategies
- Integration with other perf analysis tools (perf report, perf annotate)

### Frequency and Sampling Tuning

**Trade-off**: Higher sampling frequency = more accurate profiling, but more overhead.

**Default**: 99 Hz (99 samples/second per CPU core)
- Low overhead (~1-2%)
- Sufficient for identifying major hotspots
- May miss very short-lived functions

**Higher Frequency**: 999 Hz or 9999 Hz
```bash
CARGO_FLAMEGRAPH_ARGS="--freq 999" ./scripts/profiling/profile-benchmarks.sh
```
- More overhead (~5-10% at 999 Hz)
- Better visibility into short functions
- Use when profiling very optimized code with many small functions

**Lower Frequency**: 49 Hz
```bash
CARGO_FLAMEGRAPH_ARGS="--freq 49" ./scripts/profiling/profile-benchmarks.sh
```
- Minimal overhead (<1%)
- May miss smaller hotspots
- Use for profiling in near-production conditions

### Profiling with Different Decay Strategies

To profile specific decay strategies without running all benchmarks:

```bash
# Example: Profile only exponential decay
cargo flamegraph --bench performance --output target/profiling/exponential_decay.svg -- \
  --bench decay_operation --measurement-time 30
```

Criterion allows filtering benchmarks by name—check benchmark output for available filter patterns.

## References

### Official Documentation

- **cargo-flamegraph**: [github.com/flamegraph-rs/flamegraph](https://github.com/flamegraph-rs/flamegraph)
  - Installation instructions
  - Command-line options
  - Troubleshooting common issues

- **Brendan Gregg's Flamegraph Guide**: [www.brendangregg.com/flamegraphs.html](https://www.brendangregg.com/flamegraphs.html)
  - Comprehensive explanation of flamegraph concepts
  - Interpretation techniques
  - Real-world case studies

- **Linux perf Documentation**: [perf.wiki.kernel.org](https://perf.wiki.kernel.org)
  - In-depth perf usage
  - Event types and sampling strategies
  - Advanced profiling techniques

### Rust-Specific Resources

- **Rust Performance Book**: [nnethercote.github.io/perf-book](https://nnethercote.github.io/perf-book/)
  - Rust optimization strategies
  - Profiling best practices
  - Benchmark interpretation

- **Criterion.rs**: [bheisler.github.io/criterion.rs](https://bheisler.github.io/criterion.rs/)
  - Benchmarking library used by Alejandria
  - Statistical analysis of performance
  - Benchmark design patterns

### Community Examples

- **Flamegraph Examples**: [www.brendangregg.com/FlameGraphs/cpuflamegraphs.html](https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html)
  - Real-world flamegraphs from various projects
  - Annotated examples with explanations
  - Common patterns and anti-patterns

- **Rust Profiling Case Studies**: Search for "Rust profiling" on the Rust blog ([blog.rust-lang.org](https://blog.rust-lang.org/))
  - How the Rust compiler team uses profiling
  - Optimization success stories from the community

---

**Next Steps**:

1. **Profile Your First Benchmark**: Run `./scripts/profiling/profile-benchmarks.sh` and explore the flamegraphs
2. **Compare Optimization**: Make a small optimization and use `compare-profiles.sh` to visualize the impact
3. **Review CI Flamegraphs**: Check GitHub Actions artifacts from recent commits
4. **Experiment**: Try different sampling frequencies and custom arguments to see how they affect results

For questions or improvements to this guide, please open an issue or PR!

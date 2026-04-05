# Contributing to Alejandria

Thank you for your interest in contributing to Alejandria! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

## Development Setup

### Prerequisites

- **Rust 1.70+**: Install via [rustup](https://rustup.rs/)
- **Git**: For version control
- **A code editor**: VS Code with rust-analyzer is recommended

### Getting Started

1. **Fork and clone the repository**

```bash
git clone https://github.com/yourusername/alejandria.git
cd alejandria
```

2. **Build the project**

```bash
# Build all crates
cargo build --all-features

# Run tests
cargo test --all-features

# Run clippy for linting
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

3. **Run the CLI locally**

```bash
# Without installing
cargo run -p alejandria-cli -- --help

# Store a test memory
cargo run -p alejandria-cli -- store "Test memory" --topic development

# Search memories
cargo run -p alejandria-cli -- recall "test" --limit 5
```

## Project Structure

```
alejandria/
├── crates/
│   ├── alejandria-core/      # Core types, traits, and error handling
│   ├── alejandria-storage/   # SQLite storage implementation
│   ├── alejandria-mcp/       # MCP server implementation
│   └── alejandria-cli/       # Command-line interface
├── docs/                     # Documentation
├── .github/                  # CI/CD workflows
└── README.md
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

### 2. Make Changes

- Write clean, idiomatic Rust code
- Follow existing code style and patterns
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test --all-features

# Run specific crate tests
cargo test -p alejandria-storage

# Run tests with logging
RUST_LOG=debug cargo test --all-features -- --nocapture

# Run benchmarks (if applicable)
cargo bench --all-features
```

## Performance Profiling

Profile benchmarks to identify performance bottlenecks and optimize hot paths. See the [Performance Profiling Guide](docs/profiling.md) for comprehensive instructions.

```bash
# Profile all benchmarks (generates flamegraphs in target/profiling/)
./scripts/profiling/profile-benchmarks.sh

# Profile a single benchmark with custom arguments
./scripts/profiling/profile-single.sh hybrid_search

# Compare performance between branches
./scripts/profiling/compare-profiles.sh main your-feature-branch
```

**Requirements**: You need `cargo-flamegraph` and `perf` installed:
```bash
# Install cargo-flamegraph
cargo install flamegraph

# Install perf (Linux)
sudo apt-get install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
```

**When to Profile**:
- Before optimizing (identify actual bottlenecks, not assumed ones)
- After optimizing (verify improvements worked as expected)
- When investigating performance regressions
- To understand execution flow through complex operations

For detailed usage, interpreting flamegraphs, and troubleshooting, see [docs/profiling.md](docs/profiling.md).

## Test Coverage

Alejandria maintains a minimum code coverage threshold to ensure quality and maintainability. Understanding and working with coverage reports is an essential part of the development workflow.

### Generating Coverage Locally

Generate HTML coverage reports on your local machine to see which code is covered by tests:

```bash
# Generate coverage and open report in browser
make coverage

# Generate coverage without opening browser
make coverage-no-open

# Clean up coverage artifacts
make coverage-clean
```

**Requirements**: You need `cargo-tarpaulin` installed:
```bash
cargo install cargo-tarpaulin
```

### Viewing Coverage Reports

After running `make coverage`, an HTML report will open in your browser showing:

- **Coverage summary**: Overall line/branch/function coverage percentages
- **File-level breakdown**: Navigate to specific files to see coverage details
- **Line highlighting**: 
  - ✅ Green lines are covered by tests
  - ❌ Red lines are not covered
  - Gray lines are not executable (comments, declarations)

**Manual navigation**: If the browser doesn't open automatically, find the report at:
```
file:///path/to/alejandria/target/coverage/html/index.html
```

### Coverage in CI

The CI pipeline enforces a **minimum coverage threshold of 70%** (line coverage). This means:

- ✅ **PR builds pass** when coverage is ≥70%
- ❌ **PR builds fail** when coverage drops below 70%

**When a build fails due to coverage:**

1. Download the coverage report artifact from the GitHub Actions run:
   - Go to the failed workflow run page
   - Scroll to "Artifacts" section
   - Download `coverage-report-{run_id}-{run_number}-failed`
   
2. Extract the ZIP and open `index.html` in your browser

3. Identify uncovered code:
   - Look for red-highlighted lines in the report
   - Focus on critical paths and new functionality
   - Add tests to cover those lines

4. Run `make coverage` locally to verify improvements before pushing

**Threshold configuration**: The 70% threshold is defined in:
- Primary: `Cargo.toml` → `[workspace.metadata.coverage]` → `minimum_coverage = 70`
- Override: `.github/workflows/ci.yml` → `env.COVERAGE_THRESHOLD` (for CI-specific adjustments)

### Troubleshooting Coverage

#### "cargo-tarpaulin not found"

**Solution**: Install cargo-tarpaulin:
```bash
cargo install cargo-tarpaulin
```

#### "Coverage below threshold"

**Solution**: Identify uncovered code and add tests:

1. Run `make coverage` locally
2. Open the HTML report and navigate to files with low coverage
3. Look for red-highlighted lines (uncovered code)
4. Write tests that execute those code paths
5. Re-run `make coverage` to verify improvement
6. Aim for incremental progress: even 1-2% improvement helps

**Tip**: Focus on covering:
- Error handling paths (test failure scenarios)
- Edge cases (empty inputs, boundary values)
- Conditional branches (if/else, match arms)

#### "HTML report not opening"

**Solution**: Open manually using the file path:

```bash
# Linux
xdg-open target/coverage/html/index.html

# macOS
open target/coverage/html/index.html

# Or copy the path and open in browser:
file:///path/to/alejandria/target/coverage/html/index.html
```

#### "LCOV not generated"

**Solution**: Verify tarpaulin ran with correct flags:

```bash
# Should generate all three formats
cargo tarpaulin --all-features --out Html,Lcov,Xml --output-dir target/coverage

# Verify LCOV file exists
ls -lh target/coverage/lcov.info
```

**For IDE integration (VSCode Coverage Gutters)**:
1. Install the "Coverage Gutters" extension
2. Run `make coverage` to generate `lcov.info`
3. VSCode will automatically detect and display line coverage in the editor gutter

### 4. Format and Lint

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### 5. Commit Changes

```bash
git add .
git commit -m "feat: add new feature"
# or
git commit -m "fix: resolve issue with X"
```

**Commit message format:**
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `test:` for test additions/changes
- `refactor:` for code refactoring
- `perf:` for performance improvements
- `chore:` for maintenance tasks

### 6. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub with:
- Clear description of changes
- Reference to related issues (if any)
- Screenshots/examples (if applicable)

## Code Style Guidelines

### Rust Best Practices

- **Use `rustfmt`**: All code must be formatted with `cargo fmt`
- **Pass `clippy`**: Fix all clippy warnings before submitting
- **Write tests**: Aim for >80% code coverage
- **Document public APIs**: Use `///` doc comments for public items
- **Handle errors properly**: Use `Result<T, E>` and proper error types
- **Avoid `unwrap()`**: Prefer `?` operator or proper error handling

### Example

```rust
/// Store a new memory or update existing via topic_key upsert.
///
/// # Arguments
///
/// * `memory` - The memory to store
///
/// # Returns
///
/// The ULID of the stored memory
///
/// # Errors
///
/// Returns `IcmError::DatabaseError` if storage fails
pub fn store(&self, memory: Memory) -> IcmResult<String> {
    // Implementation
}
```

## Testing Guidelines

### Unit Tests

- Place tests in the same file as the code using `#[cfg(test)] mod tests`
- Test edge cases and error conditions
- Use descriptive test names: `test_feature_should_behavior`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = Memory::new("topic".to_string(), "summary".to_string());
        assert_eq!(memory.topic, "topic");
        assert_eq!(memory.weight, 1.0);
    }
}
```

### Integration Tests

- Place in `tests/` directory
- Test complete workflows and interactions between components
- Clean up resources (temp files, databases) after tests

```rust
#[test]
fn test_store_and_retrieve() {
    let temp_dir = TempDir::new().unwrap();
    let store = SqliteStore::open(temp_dir.path().join("test.db")).unwrap();
    
    // Test logic
    
    // Cleanup happens automatically when temp_dir drops
}
```

## Documentation

- **Update README.md** if adding new features
- **Update API docs** for public interfaces
- **Add examples** in doc comments
- **Update ARCHITECTURE.md** for architectural changes

## Pull Request Process

1. **Ensure CI passes**: All tests, clippy, and formatting checks must pass
2. **Update documentation**: Include relevant documentation updates
3. **Add tests**: New features must include tests
4. **Keep PRs focused**: One feature/fix per PR
5. **Respond to feedback**: Address review comments promptly

## Release Process

(Maintainers only)

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create a git tag: `git tag -a v0.x.0 -m "Release v0.x.0"`
4. Push tag: `git push origin v0.x.0`
5. CI will build and create GitHub release

## Getting Help

- **Issues**: Open a GitHub issue for bugs or feature requests
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: Check `docs/` directory and README.md

## License

By contributing to Alejandria, you agree that your contributions will be licensed under the same dual MIT/Apache-2.0 license as the project.

---

Thank you for contributing to Alejandria! 🎉

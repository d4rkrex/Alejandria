---
name: alejandria-commit-hygiene
description: >
  Commit and branch naming standards for Alejandría contributors.
  Trigger: Any commit creation, branch creation, or PR review.
license: MIT
metadata:
  author: appsec-team
  version: "1.0"
  project: alejandria
  adapted_from: engram-commit-hygiene
---

## When to Use

Use this skill when:
- Creating commits
- Creating or naming branches
- Reviewing commit history in a merge request
- Cleaning up staged changes before commit
- Preparing code for review

---

## Critical Rules

1. **Commit messages MUST follow Conventional Commits**
2. **Branch names SHOULD follow `type/description` format**
3. Keep one logical change per commit
4. Message should explain **why**, not only what
5. **NEVER** commit secrets, credentials, or `.env` files
6. **NEVER** commit generated files (binaries, `target/`, `tarpaulin-report.html`)
7. Test before committing (`cargo test` must pass)

---

## Commit Message Format

### Standard Format
```
<type>(<optional-scope>): <description>

[optional body]

[optional footer(s)]
```

### With Breaking Change
```
<type>(<optional-scope>)!: <description>

BREAKING CHANGE: explanation
```

---

## Allowed Types

| Type | Purpose | Example |
|------|---------|---------|
| `feat` | New feature | `feat(tui): add theme switcher with Ctrl+T` |
| `fix` | Bug fix | `fix(tui): resolve arrow key navigation in Help tab` |
| `docs` | Documentation only | `docs: update TUI keybindings in README` |
| `refactor` | Code refactoring (no behavior change) | `refactor(tui): extract scroll logic to ScrollState` |
| `chore` | Maintenance, dependencies | `chore(deps): bump ratatui to 0.26` |
| `style` | Formatting, whitespace | `style(tui): fix alignment in status bar` |
| `perf` | Performance improvement | `perf(storage): optimize FTS5 query with index` |
| `test` | Adding or fixing tests | `test(tui): add unit tests for scroll handlers` |
| `build` | Build system changes | `build: add cargo-tarpaulin to CI pipeline` |
| `ci` | CI/CD pipeline changes | `ci: add coverage reporting to GitLab CI` |
| `revert` | Revert a previous commit | `revert: revert commit abc123` |
| `security` | Security fixes (use for CVEs) | `security: sanitize FTS5 queries to prevent SQL injection` |

---

## Scope Guidelines

Scope is optional but recommended for clarity.

### Common Scopes
- `tui` - Terminal user interface
- `mcp` - MCP server
- `storage` - Database/storage layer
- `cli` - Command-line interface
- `api` - HTTP API
- `core` - Core business logic
- `deps` - Dependencies
- `tests` - Testing infrastructure

### Scope Rules
- Lowercase only
- Short and specific
- Allows `a-z`, `0-9`, `.`, `_`, `-`
- No spaces

---

## Description Rules

1. **Imperative mood**: "add" not "added" or "adds"
2. **Lowercase** (except proper nouns)
3. **No period at end**
4. **Max 72 characters**
5. **Be specific**: "fix Help tab scroll" not "fix bug"

### Good Examples
```
feat(tui): add 4 color themes with Ctrl+T toggle
fix(tui): prevent scroll overflow in Help tab
docs(skills): create tui-quality skill with UX rules
refactor(tui): extract ScrollState struct for smart scrolling
test(tui): add unit tests for navigation handlers
chore(deps): update ratatui to 0.26.1
security(storage): sanitize user input in FTS5 queries
```

### Bad Examples
```
Fix bug                                    ← no type, vague
feat: Add theme support                    ← description not lowercase
FEAT(tui): add themes                      ← type must be lowercase
feat (tui): add themes                     ← space before scope
feat(TUI): add themes                      ← scope must be lowercase
update readme                              ← no conventional format
Fixed the arrow keys in help tab.         ← not imperative, period at end
```

---

## Body (Optional)

Use body for:
- Additional context
- Explaining **why** the change was made
- Describing side effects
- Linking to issues

### Format
- Leave blank line after description
- Wrap at 72 characters
- Use bullet points for lists

### Example
```
feat(tui): add smart scroll with auto-adjust

Implements Engram-style smart scrolling that automatically
adjusts scroll offset when cursor moves out of visible area.
This prevents users from scrolling beyond content bounds.

Changes:
- Calculate visible_lines based on terminal height
- Auto-scroll when cursor >= offset + visible_lines
- Clamp scroll to max_scroll bounds

Closes #42
```

---

## Footer (Optional)

Use footer for:
- Breaking changes
- Issue references
- Co-authors (rare)

### Breaking Changes
```
feat(api)!: change memory ID format from UUID to ULID

BREAKING CHANGE: Memory IDs are now ULIDs instead of UUIDs.
Existing databases need migration script (see docs/MIGRATION.md).
```

### Issue References
```
Closes #123
Fixes #456
Refs #789
```

---

## Branch Naming Format

### Standard Format
```
<type>/<description>
```

### Examples
```
feat/theme-system
fix/help-scroll-arrows
docs/skill-creation-guide
refactor/scroll-state-struct
test/tui-navigation-suite
chore/bump-dependencies
security/sanitize-fts5-queries
```

### Rules
- Type MUST be one of the commit types
- Description MUST be lowercase
- Use hyphens, not underscores or spaces
- Be descriptive but concise
- Only `a-z`, `0-9`, `.`, `_`, `-` allowed

### Good Branch Names
```
feat/smart-scroll-auto-adjust
fix/arrow-keys-help-tab
docs/tui-quality-skill
refactor/extract-scroll-state
test/add-tui-unit-tests
```

### Bad Branch Names
```
feature/add-themes                 ← "feature" not allowed, use "feat"
fix/Fix-Bug                        ← uppercase not allowed
my-branch                          ← no type prefix
fix_something                      ← underscore not standard
feat/add themes                    ← space not allowed
```

---

## Pre-Commit Checklist

Before running `git commit`:

- [ ] Changes match commit scope (no unrelated changes)
- [ ] Commit message follows Conventional Commits
- [ ] Branch name follows `type/description`
- [ ] No secrets, credentials, or `.env` files
- [ ] No binaries (`target/`, `*.so`, `*.exe`)
- [ ] No coverage outputs (`tarpaulin-report.html`, `cobertura.xml`)
- [ ] No IDE configs (`.vscode/`, `.idea/`)
- [ ] Tests pass: `cargo test`
- [ ] Lint passes: `cargo clippy`
- [ ] Format checked: `cargo fmt --check`

---

## Commit Workflow

### 1. Stage Changes
```bash
git add <files>
git status  # Verify what's staged
```

### 2. Write Commit Message
```bash
# Use editor for multi-line messages
git commit

# Or inline for simple commits
git commit -m "feat(tui): add version display in title bar"
```

### 3. Verify Before Push
```bash
# Check last commit
git log -1

# Amend if needed (ONLY if not pushed yet)
git commit --amend

# Push
git push origin <branch>
```

---

## Fixing Bad Commits

### Amend Last Commit (Not Pushed)
```bash
# Fix commit message
git commit --amend -m "correct message"

# Add forgotten files
git add forgotten_file.rs
git commit --amend --no-edit
```

### Interactive Rebase (Not Pushed)
```bash
# Rewrite last 3 commits
git rebase -i HEAD~3

# Options: reword, squash, fixup, drop
```

### After Pushing
```bash
# DO NOT force push to main/master
# Create fixup commit instead
git commit -m "fixup: correct typo in previous commit"
```

---

## Review Guidelines

When reviewing commits:

- ✅ Type is correct and appropriate
- ✅ Scope matches changed files
- ✅ Description is clear and imperative
- ✅ No secrets or binaries committed
- ✅ Commit is atomic (one logical change)
- ✅ Breaking changes have `!` and BREAKING CHANGE footer
- ✅ Tests included for behavior changes

If commit doesn't meet standards:
1. Comment on specific issue
2. Request rebase/amend
3. Provide correct example

---

## CI Integration (Future)

```yaml
# .gitlab-ci.yml
commit-lint:
  stage: validate
  script:
    - git log -1 --pretty=%B > commit-msg.txt
    - |
      if ! grep -qE '^(feat|fix|docs|refactor|chore|style|perf|test|build|ci|revert|security)(\(.+\))?!?: .+' commit-msg.txt; then
        echo "ERROR: Commit message doesn't follow Conventional Commits"
        echo "Format: type(scope): description"
        exit 1
      fi
  only:
    - merge_requests
```

---

## Examples from Alejandría History

### Good Commits
```
feat(tui): add theme system, improved navigation, and enhanced visuals (v1.9.0)
feat(tui): add 2-line contextual status bar with keybindings
chore: update binary with 2-line status bar (38MB)
test: add comprehensive TUI v2 test script
docs(tui): update help text and fix warnings
```

### Could Be Improved
```
update binary                              → chore(build): update binary for v1.9.0
fix bug in help                            → fix(tui): resolve scroll issue in Help tab
add tests                                  → test(tui): add navigation handler tests
```

---

## Special Cases

### Security Fixes
```
security(storage): sanitize FTS5 queries to prevent SQL injection

Replaces string interpolation with parameterized queries
to prevent SQL injection attacks via user-controlled search.

CVE: None (preventive measure)
CVSS: N/A
```

### Reverts
```
revert: revert "feat(tui): add experimental feature"

This reverts commit abc123def456.
Reason: Feature caused crash on macOS terminals.
```

### Breaking Changes
```
feat(api)!: change session ID format

BREAKING CHANGE: Session IDs now use ULID instead of UUID.
Migration: Run `alejandria migrate sessions` before upgrading.
```

---

## Enforcement

- Commit messages SHOULD be validated in CI (future)
- PRs with poor commit hygiene SHOULD be sent back for cleanup
- Maintainers MAY squash-merge if commit history is messy
- Security commits MUST be clearly marked
- Breaking changes MUST have `!` and footer

**Target: 100% Conventional Commits compliance**

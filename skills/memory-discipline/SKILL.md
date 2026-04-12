---
name: alejandria-memory-discipline
description: >
  What to save to Alejandría memory system and how to structure it.
  Trigger: After completing tasks, making decisions, fixing bugs, or discovering non-obvious insights.
license: MIT
metadata:
  author: appsec-team
  version: "1.0"
  project: alejandria
---

## When to Use

Use this skill:
- After completing any substantial task (bug fix, feature, refactor)
- When making architecture or design decisions
- After discovering non-obvious patterns or gotchas
- When implementing workarounds or solutions to tricky problems
- At the end of a session (even if incomplete)

---

## Critical Rules (MANDATORY)

1. **Save IMMEDIATELY after completion** - Don't wait until session end
2. **Include WHY, not just WHAT** - Context is more valuable than code
3. **Save to appropriate topic** - Use existing topics when possible
4. **Structure matters** - Follow the memory format below
5. **Include learnings** - "What surprised you?" is the gold

---

## What to Save (ALWAYS)

### 1. Bug Fixes
**When**: Any time you fix a bug, even minor ones

**What to include**:
- Symptoms (what user reported)
- Root cause (why it happened)
- Solution (how you fixed it)
- Prevention (how to avoid it next time)

**Example**:
```
## Bug: Topic Navigation Broken (v1.9.2)

### Symptoms
Arrow keys in Memories tab didn't navigate topics - data inaccessible.

### Root Cause
Handler called next_memory() instead of next_topic() - methods existed but not wired.

### Solution
Changed Tab::Memories handler:
- Line 745: next_memory() → next_topic()
- Line 765: prev_memory() → prev_topic()

### Learned
Context-aware navigation: Each tab needs custom j/k/arrow handling.
Dual-panel UX: Only one panel can respond to arrows at a time.
```

---

### 2. Architecture Decisions
**When**: Choosing between design alternatives

**What to include**:
- Decision made (what you chose)
- Alternatives considered (what you rejected)
- Trade-offs (pros/cons of each)
- Context (constraints that influenced decision)

**Example**:
```
## Decision: AppState Refactor to Sub-Structs (v1.9.1)

### Chose
Group related fields into FilterState, MemoriesState, ScrollStates.

### Alternatives
1. Keep flat structure (rejected: hard to understand ownership)
2. Use modules instead of structs (rejected: doesn't reduce AppState size)

### Trade-offs
✅ Pro: Clear organization, easier to extend
✅ Pro: Each group has Default impl
❌ Con: More boilerplate (150+ reference updates)

### Context
AppState had 20+ flat fields - cognitive overload when reading.
```

---

### 3. Non-Obvious Discoveries
**When**: You learn something surprising or counterintuitive

**What to include**:
- What you expected
- What actually happened
- Why it works that way
- Impact on future work

**Example**:
```
## Discovery: Ratatui Scroll Requires Manual Bounds

### Expected
Scroll widget would auto-clamp to content bounds.

### Reality
No built-in clamping - scroll offset can exceed content length.

### Why
Ratatui is low-level - delegates bounds checking to app logic.

### Impact
Must manually calculate max_offset = total_lines - visible_lines.
Affects all scrollable tabs (Help, Stats, ActivityLog).
```

---

### 4. Implementation Patterns
**When**: You establish a reusable pattern or convention

**What to include**:
- Pattern name
- When to use it
- Code structure
- Benefits

**Example**:
```
## Pattern: Empty State Messages

### When
Any UI component that can be empty (lists, searches, filters).

### Structure
[Icon] [Bold Title]

[Brief explanation]

[Actionable instructions]:
  • [Primary action with keybinding]
  • [Alternative actions]

### Benefits
- Users know what to do (not confused)
- Reduces support questions
- Improves discoverability
```

---

### 5. Workarounds & Gotchas
**When**: You solve a tricky problem or find unexpected behavior

**What to include**:
- Problem encountered
- Why normal approach failed
- Workaround used
- Future ideal solution

**Example**:
```
## Gotcha: Sed Misses Multiline Continuations

### Problem
Used sed to bulk-replace app.field → app.group.field.
Missed continuations like:
  let x = app
      .field  // ← This line wasn't matched

### Why
Regex matched app\.field\> but continuation had no 'app.' prefix.

### Workaround
Manual fix for continuation lines after bulk sed replacement.

### Future
Use rust-analyzer refactoring instead of sed for large renames.
```

---

## What NOT to Save

❌ **Code that can be read** - Don't copy-paste entire functions  
❌ **Obvious facts** - "Changed variable name" without context  
❌ **Incomplete thoughts** - If you're unsure, note it as "TODO: verify X"  
❌ **Duplicates** - Search memory first to avoid redundant saves  
❌ **Generated boilerplate** - Save the pattern, not every instance  

---

## Memory Structure (Template)

```markdown
## [Type]: [Short Title] ([Version/Context])

### What
[1-2 sentences: What was done]

### Why
[Context that motivated this - user request, bug, performance, etc.]

### Where
[Files/locations affected - be specific with line numbers when relevant]

### How (optional)
[Key implementation details - only if non-obvious]

### Learned
[Insights, gotchas, things that surprised you - MOST VALUABLE SECTION]

### Impact
[How this affects future work, what's now possible/easier]
```

---

## Topic Naming Conventions

Use existing topics when possible. Create new topics only when needed.

### Existing Topics (from alejandria topics --json)
- **Project-specific**: VT-CodeWars, Argos, Veredict, VTstrike-Lite, etc.
- **Component-specific**: Alejandria, veriscan, cloud-scan, etc.
- **General categories**: general, security-review, testing-tui
- **Concepts**: rust-concepts, typescript-concepts, mcp-protocol

### When to Create New Topic
- New project starts (e.g., "ProjectName")
- New major component (e.g., "alejandria-plugin-system")
- Specialized area (e.g., "performance-optimization")

### Topic Naming Pattern
- **Projects**: CamelCase or kebab-case (e.g., "VT-Spec", "cloud-scan")
- **Components**: lowercase-with-dashes (e.g., "alejandria-mcp")
- **Concepts**: noun-concepts (e.g., "rust-concepts", "security-patterns")
- **Tasks**: verb-noun (e.g., "testing-tui", "security-review")

---

## Save Command Format

```bash
alejandria store \
  --topic "Alejandria" \
  --importance high \
  --content "$(cat <<'EOF'
## Bug Fix: Topic Navigation + Memory Loading (v1.9.3)

### What
Fixed two critical bugs preventing topic access in TUI.

### Why
User reported: "no puedo navegar por los topics" - blocking access to 1,588 memories.

### Where
crates/alejandria-cli/src/commands/tui.rs:
- Lines 745, 765: Changed next_memory() → next_topic()
- Lines 746-751, 766-771: Added memory reload after topic change

### Learned
1. Context-aware navigation: Each tab needs different j/k behavior
2. Data loading != navigation - must reload on selection change
3. Empty app.memories.memories_list despite full DB → stale data
4. User feedback is gold: Version discipline + clear bug reports

### Impact
All 43 topics (1,588 memories) now accessible via keyboard navigation.
EOF
)"
```

---

## Examples from This Session

### Good Memory: Bug Fix with Context
```markdown
## Bug Fix: Topic Navigation Broken (v1.9.2→v1.9.3)

### What
Arrow keys navigated topics but didn't reload memories for newly selected topic.

### Why
User saw empty state for all topics except first (loaded on tab switch).
Data existed (1,588 memories across 43 topics) but wasn't displayed.

### Where
- Tab switch (line 1057): Loaded only first topic's memories
- Navigation (lines 745, 765): Changed topic index but didn't reload

### Learned
Navigation state ≠ Data loading.
Must explicitly reload when selection changes.
Always verify data flow: UI state → data fetch → render.

### Impact
User can now access all their topics and memories via keyboard.
Pattern applies to any list-detail dual-panel UI.
```

### Bad Memory: Too Generic
```markdown
## Changed Navigation

Fixed arrows in Memories tab.
```
↑ No context, no learning, not useful for future reference.

---

## Pre-Commit Checklist

Before committing code, ask:

- [ ] Did I make a non-obvious decision? → Save it
- [ ] Did I fix a bug? → Save symptoms + root cause
- [ ] Did I discover a gotcha? → Save the surprise
- [ ] Did I establish a pattern? → Save the template
- [ ] Did I implement a workaround? → Save why + future fix
- [ ] Is this knowledge that would help me 6 months from now? → Save it

---

## Session End Protocol

At end of ANY session (even if incomplete):

1. **Summarize goals**: What we intended to do
2. **List accomplishments**: What actually got done
3. **Capture learnings**: Surprises, gotchas, discoveries
4. **Note next steps**: What remains (for future me)
5. **List affected files**: With brief "what changed" notes

**Save to topic**: Use session-specific or "general" topic.

---

## Integration with Other Skills

This skill COMPLEMENTS other skills:

| Skill | When to Save Memory |
|-------|---------------------|
| `tui-quality` | After UX improvements, document patterns |
| `testing` | After achieving coverage milestone, note test strategy |
| `commit-hygiene` | After complex commit, save rationale |
| `threat-model` | After security review, save findings + mitigations |

---

## Anti-Patterns to Avoid

### ❌ Saving Too Late
```
Bad: Wait until session end, forget 80% of details
Good: Save immediately after each major accomplishment
```

### ❌ Saving Too Much Code
```
Bad: Copy 200 lines of implementation
Good: Describe the pattern + link to commit SHA
```

### ❌ Saving Without Context
```
Bad: "Fixed bug in line 745"
Good: "Fixed bug where navigation called wrong method due to copy-paste"
```

### ❌ Forgetting the "Why"
```
Bad: "Refactored AppState to use sub-structs"
Good: "Refactored AppState because 20+ flat fields caused cognitive overload"
```

### ❌ Not Searching First
```
Bad: Save duplicate memory about same topic
Good: Search first: alejandria recall "topic navigation"
```

---

## Memory Search Before Save

Always search before saving to avoid duplicates:

```bash
# Search by keywords
alejandria recall "topic navigation TUI"

# Search by topic
alejandria recall --topic Alejandria "navigation bug"

# If similar memory exists, UPDATE instead of duplicate
```

---

## Enforcement

This skill is RECOMMENDED for all development sessions. Benefits:

- ✅ **Knowledge retention**: Remember decisions 6 months later
- ✅ **Onboarding**: New team members learn from past decisions
- ✅ **Debugging**: "We solved this before, how did we do it?"
- ✅ **Pattern recognition**: See recurring issues and fix root cause
- ✅ **Continuous learning**: Build personal knowledge base

**Target**: 1-3 memories saved per session (quality > quantity)

---

## Quick Reference Card

```
┌─────────────────────────────────────────────┐
│  WHEN TO SAVE TO ALEJANDRÍA                │
├─────────────────────────────────────────────┤
│  ✓ Bug fixed → Save symptoms + root cause  │
│  ✓ Decision made → Save alternatives + why  │
│  ✓ Discovery → Save surprise + impact       │
│  ✓ Pattern → Save template + when to use   │
│  ✓ Workaround → Save problem + future fix  │
│  ✓ Session end → Save summary + learnings  │
│                                             │
│  ✗ Obvious code → Skip                      │
│  ✗ Generated boilerplate → Skip            │
│  ✗ Incomplete thoughts → Mark as TODO      │
└─────────────────────────────────────────────┘
```

---

## Success Metrics

Good memory discipline when:
- Future you can understand past decisions without reading code
- New contributors can ramp up faster
- Bugs don't repeat because root causes are documented
- Patterns emerge from accumulated memories
- Team knowledge is searchable and accessible

**Remember**: The best time to document is RIGHT NOW, while it's fresh. The second best time is never.

---
name: using-alejandria
description: >
  How to use Alejandría persistent memory system effectively with any codebase.
  Learn when to save, search, and structure knowledge for long-term recall.
license: MIT
metadata:
  author: appsec-team
  version: "1.0"
  scope: universal
---

## When to Use

Use Alejandría memory system whenever you work on ANY codebase where you need to:
- Preserve decisions, bugs, and discoveries across sessions
- Recall past work without re-reading code
- Build cumulative knowledge that survives context resets
- Onboard future agents or humans to past decisions

**Critical**: Alejandría persists memories OUTSIDE of context windows. This means knowledge survives:
- Session ends
- Context compaction
- Long breaks between work
- Switching between agents

---

## MCP Tools Available

When using Alejandría via MCP (Model Context Protocol), you have access to these tools:

### Core Operations
- **`mem_save`** — Save important observations (decisions, bugs, discoveries)
- **`mem_search`** — Full-text search across all memories
- **`mem_context`** — Get recent session history (cheap, fast)
- **`mem_get_observation`** — Retrieve full content of a specific memory by ID
- **`mem_update`** — Modify an existing memory by ID

### Topic Management
- **`mem_suggest_topic_key`** — Get stable topic key for evolving observations
- **`mem_save`** with `topic_key` — Use topic keys to update evolving topics

### Session Management
- **`mem_session_start`** — Register session start
- **`mem_session_end`** — Mark session complete
- **`mem_session_summary`** — Save comprehensive end-of-session summary

### Passive Capture
- **`mem_capture_passive`** — Extract learnings from structured text output

---

## Critical Rules (MANDATORY)

### 1. Save IMMEDIATELY After Completion
Don't wait until session end. Save right after:
- Fixing a bug
- Making an architecture decision
- Discovering something non-obvious
- Establishing a pattern or convention
- Implementing a workaround

### 2. Search BEFORE Starting Work
When the user mentions a project, feature, or problem:
1. Call `mem_search` with keywords from their message
2. If results look relevant, call `mem_get_observation` for full content
3. Use past context to inform current work

### 3. Always Include WHY, Not Just WHAT
Context is more valuable than code:
- **Bad**: "Changed variable name from x to user_id"
- **Good**: "Renamed x → user_id because unclear naming caused confusion during review"

### 4. End EVERY Session with Summary
Before saying "done" or ending a session, call `mem_session_summary` (see Session End Protocol below).

---

## Save Protocol

### When to Save

Call `mem_save` immediately after ANY of these:

| Trigger | What to Save |
|---------|-------------|
| **Bug fixed** | Symptoms, root cause, solution, prevention |
| **Decision made** | What you chose, alternatives, trade-offs, context |
| **Discovery** | What surprised you, why it matters, impact |
| **Pattern established** | When to use, structure, benefits |
| **Workaround implemented** | Problem, why normal approach failed, solution |
| **Configuration change** | What changed, why, environment details |
| **User preference learned** | Constraint or preference, context, scope |

### Save Format

```typescript
mem_save({
  title: "Short, searchable title",  // e.g., "Fixed N+1 query in UserList"
  type: "bugfix | decision | architecture | discovery | pattern | config | preference",
  scope: "project",  // or "personal" for user-level learnings
  project: "project-name",  // Always include project name
  topic_key: "optional/stable/key",  // For evolving topics (see Topic Keys section)
  content: `
**What**: [One sentence — what was done]

**Why**: [The motivation — user request, bug, performance issue, etc.]

**Where**: [Files/paths affected with line numbers when relevant]

**Learned**: [Gotchas, edge cases, surprises — MOST VALUABLE SECTION]
  `
})
```

### Topic Keys (Optional but Recommended)

For **evolving topics** (architecture decisions that change, ongoing discoveries), use `topic_key`:

```typescript
// First save
mem_save({
  title: "API authentication design",
  type: "architecture",
  project: "my-app",
  topic_key: "architecture/auth-model",  // Stable key
  content: "**What**: Using JWT for stateless auth..."
})

// Later update (reuses same topic_key)
mem_save({
  title: "API authentication design",
  type: "architecture", 
  project: "my-app",
  topic_key: "architecture/auth-model",  // Same key → updates existing
  content: "**What**: Switched from JWT to sessions..."
})
```

If unsure about the key format, call `mem_suggest_topic_key` first.

**Important**: Different topics must NOT share the same `topic_key`. Use topic keys only for updates to the SAME logical topic.

---

## Search Protocol

### When to Search

Call `mem_search` BEFORE starting work when:
- User asks to "remember", "recall", or references past work
- Starting work that might have been done before
- User's first message mentions project, feature, or problem you have no context on

### Search Pattern

```typescript
// Step 1: Check recent context (fast)
const recent = mem_context({ project: "my-app", limit: 20 })

// Step 2: If not found, full-text search
const results = mem_search({
  query: "authentication JWT sessions",
  project: "my-app",
  type: "architecture",  // Optional filter
  limit: 10
})

// Step 3: Get full content (search returns truncated previews)
if (results.length > 0) {
  const full = mem_get_observation({ id: results[0].id })
  // Now use full.content for context
}
```

### Search Tips
- Use keywords from user's message
- Search returns truncated previews (300 chars) — ALWAYS call `mem_get_observation` for full content
- Filter by `type` to narrow results (e.g., `type: "bugfix"` for past bugs)
- Filter by `project` to avoid cross-project noise

---

## Session End Protocol (MANDATORY)

Before ending ANY session, call `mem_session_summary`:

```typescript
mem_session_summary({
  project: "my-app",
  session_id: "session-123",  // Optional, defaults to "manual-save-{project}"
  content: `
## Goal
[One sentence: what we were working on this session]

## Instructions
[User preferences or constraints discovered — skip if none]

## Discoveries
- [Technical finding, gotcha, or non-obvious learning 1]
- [Technical finding 2]

## Accomplished
- ✅ [Completed task 1 — with key implementation details]
- ✅ [Completed task 2 — mention files changed]
- 🔲 [Identified but not yet done — for next session]

## Relevant Files
- path/to/file.ts — [what it does or what changed]
- path/to/other.go — [role in the architecture]
  `
})
```

This is **NOT optional**. Without it, the next session starts blind.

---

## After Context Compaction

If you see a message about context compaction or reset:

1. **IMMEDIATELY** call `mem_session_summary` to persist pre-compaction work
2. Then call `mem_context` to recover additional context from previous sessions
3. Only THEN continue working

Without step 1, everything done before compaction is lost from memory.

---

## Examples

### Example 1: Fixing a Bug

```typescript
// After fixing the bug, save immediately:
mem_save({
  title: "Fixed N+1 query in user list endpoint",
  type: "bugfix",
  project: "my-api",
  scope: "project",
  content: `
**What**: Added eager loading for user.posts relation in GET /users endpoint

**Why**: Production logs showed 1000+ queries for a single user list page (N+1 problem)

**Where**: 
- src/controllers/users.ts:45 — Added .include('posts') to query
- src/models/user.ts:12 — Added posts relation definition

**Learned**: 
- Sequelize doesn't eager load by default — must explicitly include relations
- N+1 issues are hard to spot in dev (small datasets) but obvious in prod
- Always check query count in logs after adding new endpoints
  `
})
```

### Example 2: Architecture Decision

```typescript
mem_save({
  title: "Chose Zustand over Redux for state management",
  type: "decision",
  project: "my-app",
  scope: "project",
  topic_key: "architecture/state-management",
  content: `
**What**: Using Zustand for global state instead of Redux

**Why**: Team preferred simpler API, less boilerplate, and built-in TypeScript support

**Where**: 
- src/store/ — All store definitions
- package.json — Added zustand@4.5.0

**Learned**:
- Zustand is 10x less code than Redux for same functionality
- No providers needed — just import and use
- Middleware for persistence and devtools works out of the box
- Trade-off: Smaller ecosystem than Redux (fewer libraries/examples)
  `
})
```

### Example 3: Discovery

```typescript
mem_save({
  title: "Docker volume permissions require host UID mapping",
  type: "discovery",
  project: "my-app",
  scope: "project",
  content: `
**What**: Mounted volumes in Docker fail with permission errors unless UID matches

**Why**: Container runs as root by default, but volume is owned by host user (UID 1000)

**Where**: 
- docker-compose.yml:23 — Added user: "1000:1000"
- Dockerfile:8 — Changed from root to node user

**Learned**:
- Permission errors in containers are often UID mismatches, not actual permissions
- Always run containers as non-root user with matching host UID
- Alternative: use named volumes (but then lose easy host access)
  `
})
```

### Example 4: Session End

```typescript
mem_session_summary({
  project: "my-api",
  content: `
## Goal
Add authentication middleware to protect API endpoints

## Instructions
User wants JWT-based auth, no sessions (stateless)

## Discoveries
- express-jwt has breaking changes in v8 — must use v7.x for now
- JWT_SECRET must be 32+ chars or token validation fails silently
- Authorization header MUST use "Bearer" prefix (not "JWT")

## Accomplished
- ✅ Added JWT middleware to src/middleware/auth.ts
- ✅ Protected /api/users/* routes with requireAuth
- ✅ Added JWT_SECRET to .env.example
- 🔲 Add refresh token rotation (deferred to next session)

## Relevant Files
- src/middleware/auth.ts — JWT validation logic
- src/routes/users.ts — Protected routes
- .env.example — JWT_SECRET documented
  `
})
```

---

## What NOT to Save

❌ **Code that can be read** — Don't copy-paste entire functions  
❌ **Obvious facts** — "Changed variable name" without context  
❌ **Incomplete thoughts** — If unsure, note as "TODO: verify X"  
❌ **Duplicates** — Always search first to avoid redundant saves  
❌ **Generated boilerplate** — Save the pattern, not every instance  

---

## Integration with Sub-Agents

When launching sub-agents via Task tool:

### Non-SDD Tasks
Always include in sub-agent prompt:

```
CONTEXT:
[Include relevant memories retrieved via mem_search]

MEMORY DISCIPLINE:
If you make important discoveries, decisions, or fix bugs, save them to Alejandría via mem_save with project: '{project}'.

Before starting, check for available skills:
1. mem_search(query: "skill-registry", project: "{project}")
2. Fallback: read .atl/skill-registry.md
Load any skills whose triggers match your task.
```

### SDD/VT-Spec Phases
For structured workflows, pass artifact references (topic keys or file paths), NOT full content:

```
Read the spec artifact: mem_search(query: "sdd/{change-name}/spec", project: "{project}")
Then get full content: mem_get_observation(id: {id})
```

---

## Anti-Patterns to Avoid

### ❌ Saving Too Late
```
Bad: Wait until session end, forget 80% of details
Good: Save immediately after each accomplishment
```

### ❌ Saving Without Context
```
Bad: "Fixed bug in line 745"
Good: "Fixed bug where navigation called wrong method due to copy-paste error"
```

### ❌ Forgetting the "Why"
```
Bad: "Refactored AppState to use sub-structs"
Good: "Refactored AppState because 20+ flat fields caused cognitive overload"
```

### ❌ Not Searching First
```
Bad: Save duplicate memory about same topic
Good: Search first with mem_search to check for existing context
```

### ❌ Skipping Session Summary
```
Bad: End session without calling mem_session_summary
Good: Always call mem_session_summary before saying "done"
```

---

## Quick Reference Card

```
┌───────────────────────────────────────────────────┐
│  ALEJANDRÍA MEMORY WORKFLOW                      │
├───────────────────────────────────────────────────┤
│  START SESSION                                    │
│    1. mem_search for past context                │
│    2. mem_get_observation for full details       │
│                                                   │
│  DURING WORK                                      │
│    Save immediately after:                       │
│      • Bug fixed → mem_save (type: bugfix)       │
│      • Decision made → mem_save (type: decision) │
│      • Discovery → mem_save (type: discovery)    │
│                                                   │
│  END SESSION (MANDATORY)                          │
│    mem_session_summary with Goal/Discoveries/    │
│    Accomplished/Files                            │
│                                                   │
│  AFTER COMPACTION                                 │
│    1. mem_session_summary (save pre-compaction)  │
│    2. mem_context (recover context)              │
│    3. Continue work                              │
└───────────────────────────────────────────────────┘
```

---

## Success Metrics

Good Alejandría usage when:
- Future agents can understand past decisions without reading code
- New contributors ramp up faster by searching memories
- Bugs don't repeat because root causes are documented
- Patterns emerge from accumulated knowledge
- Cross-session work is seamless (no "wait, what was I doing?")

**Remember**: The best time to save knowledge is RIGHT NOW, while it's fresh. The second best time is never.

---

## CLI Alternative (When MCP Not Available)

If using Alejandría CLI directly instead of MCP:

```bash
# Save memory
alejandria store \
  --topic "project-name" \
  --importance high \
  --content "$(cat <<'EOF'
## Bug Fix: Description

**What**: Brief description
**Why**: Context
**Where**: Files affected
**Learned**: Key insights
EOF
)"

# Search memory
alejandria recall "keywords here"
alejandria recall --topic "project-name" "specific query"

# List recent memories
alejandria topics --json
```

But when MCP is available (via OpenCode, Claude Desktop, etc.), ALWAYS prefer MCP tools over CLI.

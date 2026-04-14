# Alejandría — Agent Skills Index

When working on this project, load the relevant skill(s) BEFORE writing any code.

## How to Use

1. Check the trigger column to find skills that match your current task
2. Load the skill by reading the SKILL.md file at the listed path
3. Follow ALL patterns and rules from the loaded skill
4. Multiple skills can apply simultaneously

## User Skills (Universal — For ANY Codebase)

These skills teach agents how to USE Alejandría effectively with any project:

| Skill | Trigger | Path |
|-------|---------|------|
| `using-alejandria` | Working with any codebase where persistent memory is valuable | [`skills/using-alejandria/SKILL.md`](skills/using-alejandria/SKILL.md) |

## Development Skills (Alejandría Contributors Only)

These skills are for DEVELOPING Alejandría itself. Only loaded when contributing to this project:

| Skill | Trigger | Path |
|-------|---------|------|
| `alejandria-tui-quality` | Any change to TUI rendering, navigation, keybindings, or user interaction | [`skills/dev/tui-quality/SKILL.md`](skills/dev/tui-quality/SKILL.md) |
| `alejandria-testing` | Adding features, fixing bugs, refactoring, or any behavior change | [`skills/dev/testing/SKILL.md`](skills/dev/testing/SKILL.md) |
| `alejandria-commit-hygiene` | Creating commits, branches, or reviewing merge requests | [`skills/dev/commit-hygiene/SKILL.md`](skills/dev/commit-hygiene/SKILL.md) |
| `alejandria-memory-discipline` | After completing tasks, making decisions, fixing bugs, or at session end | [`skills/dev/memory-discipline/SKILL.md`](skills/dev/memory-discipline/SKILL.md) |

## Planned Skills (TODO)

- `alejandria-visual-language` - Theme system, color usage, typography standards
- `alejandria-security-review` - Security testing, threat modeling, STRIDE analysis
- `alejandria-mcp-protocol` - MCP server implementation standards
- `alejandria-storage-patterns` - Database access patterns, migrations
- `alejandria-docs-alignment` - Documentation updates when code changes

## Creating New Skills

1. Create directory: `skills/<skill-name>/`
2. Write `skills/<skill-name>/SKILL.md` following the format:
   ```markdown
   ---
   name: alejandria-<skill-name>
   description: Brief description
   trigger: When to use this skill
   license: MIT
   metadata:
     author: appsec-team
     version: "1.0"
     project: alejandria
   ---
   
   ## When to Use
   ## Rules
   ## Examples
   ```
3. Add entry to this AGENTS.md file
4. Commit with: `docs(skills): add <skill-name> skill`

## Notes

- Skills are MANDATORY when their trigger matches your task
- Load skills at the START of your work, not after writing code
- Skills may reference each other (e.g., tui-quality → testing)
- CI will eventually enforce skill compliance

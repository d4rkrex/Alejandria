# Alejandría — Agent Skills Index

When working on this project, load the relevant skill(s) BEFORE writing any code.

## How to Use

1. Check the trigger column to find skills that match your current task
2. Load the skill by reading the SKILL.md file at the listed path
3. Follow ALL patterns and rules from the loaded skill
4. Multiple skills can apply simultaneously

## Skills

| Skill | Trigger | Path |
|-------|---------|------|
| `alejandria-tui-quality` | Any change to TUI rendering, navigation, keybindings, or user interaction | [`skills/tui-quality/SKILL.md`](skills/tui-quality/SKILL.md) |
| `alejandria-testing` | Adding features, fixing bugs, refactoring, or any behavior change | [`skills/testing/SKILL.md`](skills/testing/SKILL.md) |
| `alejandria-commit-hygiene` | Creating commits, branches, or reviewing merge requests | [`skills/commit-hygiene/SKILL.md`](skills/commit-hygiene/SKILL.md) |
| `alejandria-memory-discipline` | After completing tasks, making decisions, fixing bugs, or at session end | [`skills/memory-discipline/SKILL.md`](skills/memory-discipline/SKILL.md) |

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

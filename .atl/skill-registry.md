# Skill Registry — Alejandria

> Auto-generated skill and convention catalog for AI agent workflows

## Available Skills

No user-level or project-level skills detected.

Skills should be placed in one of these directories:
- User-level: `~/.claude/skills/`, `~/.config/opencode/skills/`, `~/.gemini/skills/`, `~/.cursor/skills/`, `~/.copilot/skills/`
- Project-level: `.claude/skills/`, `.gemini/skills/`, `.agent/skills/`, `skills/`

## Project Conventions

No agent convention files detected.

Convention files to add:
- `agents.md` or `AGENTS.md` — Agent workflow patterns and guidelines
- `CLAUDE.md` — Claude-specific instructions
- `.cursorrules` — Cursor IDE agent rules
- `GEMINI.md` — Gemini-specific instructions
- `copilot-instructions.md` — GitHub Copilot instructions

---

**Registry Format**

When skills are added, this registry will contain:
- Skill name, description, triggers
- Skill file path
- Related convention files (expanded from index files)

Sub-agents load this registry FIRST before starting any task to apply relevant skills and conventions.

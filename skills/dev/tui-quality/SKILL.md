---
name: alejandria-tui-quality
description: >
  Ratatui quality rules for Alejandría TUI.
  Trigger: Changes in TUI rendering, navigation, keybindings, or user interaction.
license: MIT
metadata:
  author: appsec-team
  version: "1.0"
  project: alejandria
---

## When to Use

Use this skill when:
- Adding new tabs or screens
- Modifying keybinding handlers
- Changing navigation flows
- Updating list rendering or detail views
- Modifying scroll behavior
- Adding empty/loading/error states

---

## UX Rules (MANDATORY)

### 1. Keyboard Consistency
- **Arrow keys + vim keys MUST both work** for all navigation
  - `j` / `↓` → move down / scroll down
  - `k` / `↑` → move up / scroll up
  - `h` / `←` → (reserved for future horizontal nav)
  - `l` / `→` → (reserved for future horizontal nav)
- `gg` → jump to first item / top of page
- `G` → jump to last item / bottom of page
- `Enter` → select / drill into details
- `Esc` → back / cancel
- `q` → quit (at top level)
- `?` → toggle help overlay
- `Ctrl+T` → cycle themes

### 2. Scroll Behavior
- **Scroll MUST auto-adjust when cursor moves out of visible area**
- Scroll offset MUST be clamped to valid range (0 to max_scroll)
- `max_scroll = total_lines.saturating_sub(visible_lines)`
- When scrolling manually:
  - Calculate `visible_lines` based on terminal height
  - Prevent scrolling beyond content bounds
- Example smart scroll implementation:
  ```rust
  if cursor >= scroll_offset + visible_lines {
      scroll_offset = cursor - visible_lines + 1;
  }
  if cursor < scroll_offset {
      scroll_offset = cursor;
  }
  ```

### 3. Empty States (CRITICAL)
- **NEVER show empty lists without explanation**
- Empty state MUST include:
  - Clear message explaining why it's empty
  - Helpful instruction on how to add content
  - Example: "No memories found. Press `/` to search or create a new one."
- Apply to:
  - Empty API key lists
  - Empty memory lists
  - Empty search results
  - Empty sessions
  - Empty topics

### 4. Loading States
- Long operations (>500ms) MUST show feedback
- Options:
  - Spinner/progress indicator
  - Status message ("Loading memories...")
  - Disable navigation during load
- Never leave user wondering if something is happening

### 5. Error States
- Errors MUST be visible and actionable
- Error display:
  - Red color for visibility
  - Clear error message (not just error code)
  - Suggestion for resolution when possible
  - Example: "Failed to load memories: Database locked. Try again in a moment."
- Clear error on next user action (any keypress)

### 6. Visual Feedback
- Selected items MUST be clearly highlighted
- Current tab MUST be visually distinct
- Active filters MUST be visible (yellow badge)
- Status bar MUST show contextual commands
- Theme colors MUST be consistent across all widgets

---

## Testing Rules (MANDATORY)

### Coverage Requirements
- Minimum **70% coverage** for TUI module
- All navigation handlers MUST have unit tests
- Empty/loading/error states MUST be tested explicitly
- Scroll boundary conditions MUST be tested

### Test Structure
```rust
#[cfg(test)]
mod tui_tests {
    use super::*;
    
    // Navigation tests
    #[test]
    fn test_help_tab_scroll_down() {
        let mut app = AppState::new(vec![]);
        app.current_tab = Tab::Help;
        app.help_scroll_offset = 0;
        
        // Simulate 'j' keypress
        // Assert scroll increased
        assert_eq!(app.help_scroll_offset, 1);
    }
    
    // Bounds tests
    #[test]
    fn test_scroll_cannot_exceed_content() {
        // Test that scrolling stops at content end
    }
    
    // Empty state tests
    #[test]
    fn test_empty_memories_shows_message() {
        // Verify empty state renders correctly
    }
}
```

### Required Test Types
1. **Navigation tests** - j/k/arrows/gg/G
2. **Scroll boundary tests** - min/max bounds
3. **Empty state tests** - verify messages render
4. **Theme tests** - colors apply correctly
5. **Input mode tests** - search/filter/normal

### Test Commands
```bash
cargo test --package alejandria-cli
cargo test tui_tests
cargo tarpaulin --out Html --exclude-files "crates/alejandria-mcp/*"
```

---

## Architecture Rules

### 1. Extract Testable Logic
- Input handlers SHOULD be pure functions when possible
- Example:
  ```rust
  // BAD: hard to test
  (KeyCode::Char('j'), _) => {
      app.help_scroll_offset = app.help_scroll_offset.saturating_add(1);
  }
  
  // GOOD: testable
  fn handle_scroll_down(offset: usize, max: usize) -> usize {
      offset.saturating_add(1).min(max)
  }
  
  (KeyCode::Char('j'), _) => {
      let max = calculate_max_scroll(app, area);
      app.help_scroll_offset = handle_scroll_down(app.help_scroll_offset, max);
  }
  ```

### 2. State Organization
- Group related state in sub-structs
- Example:
  ```rust
  struct ScrollState {
      offset: usize,
      max: usize,
      visible_lines: usize,
  }
  
  struct FilterState {
      active: bool,
      query: Option<String>,
      importance: Option<String>,
  }
  ```

### 3. Consistent Patterns
- Use same scroll logic across all tabs
- Reuse rendering helpers
- Theme-aware colors everywhere
- Status bar helpers for all tabs

---

## Visual Standards

### 1. Color Usage
- **Primary color** - Borders, headers, keybindings
- **Secondary color** - Highlights, warnings, info
- **Success color** - Active items, confirmations
- **Error color** - Errors, delete confirmations
- **Accent color** - Selected items, focus indicators

### 2. Typography
- Headers: Bold + primary color
- Keybindings: Bold + primary color
- Descriptions: Normal + gray
- Values: Bold + accent color

### 3. Layout
- Consistent spacing (2 lines for status bar)
- Unicode box drawing for sections (┌─┐│└┘)
- Clear visual hierarchy
- Breathable whitespace

### 4. Sparklines (when applicable)
- Use 8-level blocks: `▁▂▃▄▅▆▇█`
- Width: 20 characters standard
- Color: match theme semantic colors

---

## Pre-Commit Checklist

Before committing TUI changes:

- [ ] Arrow keys tested (↑↓←→)
- [ ] Vim keys tested (j/k/h/l/gg/G)
- [ ] Scroll bounds tested (cannot scroll beyond content)
- [ ] Empty states have helpful messages
- [ ] Error states tested and clear
- [ ] Theme colors applied consistently
- [ ] Status bar shows correct commands for tab
- [ ] Unit tests added for new handlers
- [ ] Coverage ≥70% for changed code
- [ ] Manual testing in actual terminal (not just cargo run)

---

## Common Pitfalls to Avoid

1. ❌ **Forgetting to handle both arrows + vim keys**
   - Always use: `(KeyCode::Char('j'), _) | (KeyCode::Down, _)`

2. ❌ **Scroll without bounds checking**
   - Always clamp: `.min(max_scroll)`

3. ❌ **Empty lists without explanation**
   - Always provide context and actionable help

4. ❌ **Hardcoded colors**
   - Always use: `app.current_theme.primary_color()`

5. ❌ **No tests for TUI code**
   - Extract logic to testable functions

6. ❌ **Forgetting to update status bar**
   - Add new commands to tab-specific section

7. ❌ **Not testing in real terminal**
   - `cargo run` behavior ≠ actual terminal behavior

---

## Examples

### Good: Smart Scroll with Bounds
```rust
fn handle_tab_scroll_down(app: &mut AppState, area: Rect) {
    let visible_lines = (area.height as usize).saturating_sub(6);
    let total_lines = get_content_line_count(app);
    let max_scroll = total_lines.saturating_sub(visible_lines);
    
    match app.current_tab {
        Tab::Help => {
            app.help_scroll_offset = app.help_scroll_offset
                .saturating_add(1)
                .min(max_scroll);
        }
        // ... other tabs
    }
}
```

### Good: Empty State Rendering
```rust
if app.memories_list.is_empty() {
    let empty_msg = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No memories found",
            Style::default().fg(theme.secondary_color()).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Try one of these:"),
        Line::from(vec![
            Span::raw("  • Press "),
            Span::styled("/", Style::default().fg(theme.primary_color()).add_modifier(Modifier::BOLD)),
            Span::raw(" to search"),
        ]),
        Line::from(vec![
            Span::raw("  • Create a memory from another tab"),
        ]),
    ];
    let paragraph = Paragraph::new(empty_msg).centered();
    f.render_widget(paragraph, area);
    return;
}
```

---

## Enforcement

This skill is MANDATORY for all TUI changes. Code review will block PRs that:
- Don't handle arrow keys
- Have scroll bugs
- Show empty lists without explanation
- Have no tests
- Use hardcoded colors

Target: **Zero TUI bugs, 100% keyboard consistency**

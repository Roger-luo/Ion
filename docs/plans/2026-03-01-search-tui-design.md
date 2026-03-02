# Search TUI: Two-Column Interactive Layout

## Summary

Replace the current `dialoguer::Select` picker in the search command's interactive mode with a full ratatui-based two-column TUI. The left panel shows an abbreviated list of results (name + stars). The right panel reactively displays details for the currently selected result.

## Layout

```
┌─ Search Results ─────────────┬─ Details ──────────────────────────┐
│                              │                                     │
│  > anthropics/brainstorming ★42 │  Owner: anthropics                │
│    org/test-skill        ★15 │  Stars: ★ 42                       │
│    user/code-review      ★8  │                                     │
│    skills-sh/formatter   ★3  │  Description:                       │
│                              │  Collaborative brainstorming skill   │
│                              │  for turning ideas into designs      │
│                              │                                     │
│                              │  Skill Description:                  │
│                              │  Explores user intent, requirements  │
│                              │  and design before implementation    │
│                              │                                     │
│                              │  Install:                            │
│                              │  ion add anthropics/skills/brain..   │
│                              │                                     │
├──────────────────────────────┴─────────────────────────────────────┤
│  ↑↓ Navigate  Enter Install  q Quit             github │
└────────────────────────────────────────────────────────────────────┘
```

### Left Panel (~40% width)
- Flat list of results sorted by stars (descending)
- Each row: repository name + star count
- Selected row highlighted with cursor indicator
- Small registry badge (color-coded) per item

### Right Panel (~60% width)
- Owner name (extracted from repo path, no extra API calls)
- Star count
- Repository description
- SKILL.md description (when available, shown as separate labeled section)
- Install command (`ion add <source>`)

### Footer
- Keybinding hints: Up/Down navigate, Enter install, q/Esc quit
- Registry source of currently selected item

## Architecture

Follows the established pattern from the config command TUI (`src/tui/`).

### New Files
- `src/tui/search_app.rs` — State struct (`SearchApp`) with results list, selected index, scroll offset
- `src/tui/search_ui.rs` — Rendering with `Layout::horizontal()` for two-column split
- `src/tui/search_event.rs` — Key event handler (navigate, select, quit)

### Modified Files
- `src/tui/mod.rs` — Export new search TUI modules
- `src/commands/search.rs` — Replace `pick_and_install()` with ratatui TUI launcher

### Data Flow
1. Search results arrive pre-sorted by stars (descending)
2. `SearchApp` holds a flat `Vec<SearchResult>` (installable items only)
3. Up/Down keys change `selected` index, right panel re-renders with new selection's data
4. Enter exits TUI and returns selected `SearchResult`; `search.rs` runs `ion add`
5. q/Esc exits TUI returning None; no action taken

### SearchApp State
```rust
struct SearchApp {
    results: Vec<SearchResult>,
    selected: usize,
    scroll_offset: usize,
}
```

No editing modes needed — just navigation and selection.

## Decisions
- **No extra API calls** — owner name extracted from existing repo path
- **Flat list** sorted by stars, not grouped by registry (registry shown as badge)
- **Enter installs immediately** — same behavior as current `pick_and_install`
- **SKILL.md description** shown as distinct section below repo description when available

# Search TUI Two-Column Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the dialoguer-based search picker with a ratatui two-column TUI showing a result list on the left and a reactive detail panel on the right.

**Architecture:** Three new files in `src/tui/` (search_app.rs, search_ui.rs, search_event.rs) following the existing config TUI pattern. The `pick_and_install()` function in `src/commands/search.rs` is replaced with a ratatui terminal loop that returns the user's selection.

**Tech Stack:** ratatui 0.29, crossterm 0.28 (already in Cargo.toml)

---

### Task 1: Create SearchApp state struct

**Files:**
- Create: `src/tui/search_app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Create `src/tui/search_app.rs`**

```rust
use ion_skill::search::SearchResult;

pub struct SearchApp {
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub should_quit: bool,
    pub should_install: bool,
}

impl SearchApp {
    pub fn new(mut results: Vec<SearchResult>) -> Self {
        // Sort by stars descending (None treated as 0)
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
        Self {
            results,
            selected: 0,
            scroll_offset: 0,
            should_quit: false,
            should_install: false,
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    /// Extract owner name from the result's name field (e.g., "owner/repo" -> "owner")
    pub fn owner_of(result: &SearchResult) -> &str {
        result.name.split('/').next().unwrap_or(&result.name)
    }
}
```

**Step 2: Add module export in `src/tui/mod.rs`**

Add to the existing file:
```rust
pub mod search_app;
```

**Step 3: Run `cargo check` to verify compilation**

Run: `cargo check 2>&1 | head -20`
Expected: compiles with no errors (warnings OK)

**Step 4: Commit**

```bash
git add src/tui/search_app.rs src/tui/mod.rs
git commit -m "feat(search-tui): add SearchApp state struct"
```

---

### Task 2: Create search event handler

**Files:**
- Create: `src/tui/search_event.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Create `src/tui/search_event.rs`**

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::search_app::SearchApp;

pub fn handle_search_key(app: &mut SearchApp, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            if !app.results.is_empty() {
                app.should_install = true;
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }
}
```

**Step 2: Add module export in `src/tui/mod.rs`**

Add:
```rust
pub mod search_event;
```

**Step 3: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/tui/search_event.rs src/tui/mod.rs
git commit -m "feat(search-tui): add search key event handler"
```

---

### Task 3: Create search UI renderer

**Files:**
- Create: `src/tui/search_ui.rs`
- Modify: `src/tui/mod.rs`

This is the main rendering file with the two-column layout.

**Step 1: Create `src/tui/search_ui.rs`**

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::search_app::SearchApp;

pub fn render_search(frame: &mut Frame, app: &SearchApp) {
    let area = frame.area();

    // Top-level vertical: main content + footer
    let chunks = Layout::vertical([
        Constraint::Min(3),    // Main two-column area
        Constraint::Length(1), // Help footer
    ])
    .split(area);

    // Horizontal split: left list (40%) + right detail (60%)
    let columns = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .split(chunks[0]);

    render_list(frame, app, columns[0]);
    render_detail(frame, app, columns[1]);
    render_footer(frame, app, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &SearchApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search Results ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.results.is_empty() {
        let msg = Paragraph::new("No installable results.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    // Calculate visible window based on scroll_offset
    let visible_height = inner.height as usize;
    let start = app.scroll_offset;
    let end = (start + visible_height).min(app.results.len());

    let mut lines: Vec<Line> = Vec::new();
    for i in start..end {
        let r = &app.results[i];
        let is_selected = i == app.selected;

        let prefix = if is_selected { "> " } else { "  " };

        let stars_str = match r.stars {
            Some(n) => format!(" *{n}"),
            None => String::new(),
        };

        // Registry badge color
        let badge_color = match r.registry.as_str() {
            "github" => Color::White,
            "agent" => Color::Magenta,
            _ => Color::Blue,
        };

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let stars_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(&r.name, style),
            Span::styled(stars_str, stars_style),
            Span::styled(
                format!(" [{}]", r.registry),
                Style::default().fg(badge_color),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_detail(frame: &mut Frame, app: &SearchApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(r) = app.selected_result() else {
        let msg = Paragraph::new("No result selected.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    };

    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let owner = SearchApp::owner_of(r);
    let stars_str = r.stars.map_or("—".to_string(), |n| n.to_string());

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Owner: ", label_style),
            Span::styled(owner, value_style),
        ]),
        Line::from(vec![
            Span::styled("Stars: ", label_style),
            Span::styled(format!("* {stars_str}"), value_style),
        ]),
        Line::from(""),
    ];

    // Repo description
    if !r.description.is_empty() {
        lines.push(Line::from(Span::styled("Description:", label_style)));
        // Wrap description to available width
        let wrap_width = inner.width.saturating_sub(2) as usize;
        for wrapped_line in wrap_text(&r.description, wrap_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped_line}"),
                value_style,
            )));
        }
        lines.push(Line::from(""));
    }

    // Skill description (from SKILL.md frontmatter)
    if let Some(ref skill_desc) = r.skill_description {
        lines.push(Line::from(Span::styled("Skill Description:", label_style)));
        let wrap_width = inner.width.saturating_sub(2) as usize;
        for wrapped_line in wrap_text(skill_desc, wrap_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped_line}"),
                value_style,
            )));
        }
        lines.push(Line::from(""));
    }

    // Install command
    if !r.source.is_empty() {
        lines.push(Line::from(Span::styled("Install:", label_style)));
        lines.push(Line::from(Span::styled(
            format!("  ion add {}", r.source),
            dim_style,
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_footer(frame: &mut Frame, _app: &SearchApp, area: Rect) {
    let help = Paragraph::new(Line::from(Span::styled(
        " ↑↓/jk Navigate  Enter Install  q/Esc Quit",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}

/// Simple word-wrapping: split text into lines of at most `width` characters.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
```

**Step 2: Add module export in `src/tui/mod.rs`**

Add:
```rust
pub mod search_ui;
```

**Step 3: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/tui/search_ui.rs src/tui/mod.rs
git commit -m "feat(search-tui): add two-column search UI renderer"
```

---

### Task 4: Wire up the TUI in search.rs, replacing pick_and_install

**Files:**
- Modify: `src/commands/search.rs:244-276` (replace `pick_and_install`)

**Step 1: Replace `pick_and_install` with a ratatui-based TUI launcher**

Replace the entire `pick_and_install` function (lines 244-276) with:

```rust
fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    use std::io;

    use crossterm::event::{self, Event};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    use crate::tui::search_app::SearchApp;
    use crate::tui::search_event::handle_search_key;
    use crate::tui::search_ui::render_search;

    let installable: Vec<SearchResult> = results
        .iter()
        .filter(|r| !r.source.is_empty())
        .cloned()
        .collect();
    if installable.is_empty() {
        println!("No installable results to select from.");
        return Ok(());
    }

    let mut app = SearchApp::new(installable);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|frame| render_search(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            handle_search_key(&mut app, key);
        }

        if app.should_quit || app.should_install {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle install
    if app.should_install {
        if let Some(chosen) = app.selected_result() {
            log::debug!("user selected: {} (source={})", chosen.name, chosen.source);
            println!("\nInstalling '{}'...", chosen.name);
            let status = std::process::Command::new("ion")
                .arg("add")
                .arg(&chosen.source)
                .status()?;
            if !status.success() {
                anyhow::bail!("ion add failed");
            }
        }
    }

    Ok(())
}
```

**Step 2: Remove the `dialoguer` import if no longer used elsewhere**

Check if `dialoguer` is used anywhere else in the project. If not used elsewhere, it can be removed from `Cargo.toml` dependencies in a follow-up. For now, just ensure search.rs no longer imports it.

**Step 3: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: compiles with no errors

**Step 4: Run `cargo test`**

Run: `cargo test 2>&1 | tail -20`
Expected: all existing tests pass

**Step 5: Commit**

```bash
git add src/commands/search.rs
git commit -m "feat(search-tui): wire up two-column TUI in search interactive mode"
```

---

### Task 5: Handle scroll_offset for long result lists

**Files:**
- Modify: `src/tui/search_app.rs` (update `move_down` to adjust scroll_offset)

**Step 1: Update `move_down` to accept visible height**

The `move_down` method needs to adjust `scroll_offset` when the cursor moves past the visible area. Update `SearchApp`:

```rust
pub fn adjust_scroll(&mut self, visible_height: usize) {
    if self.selected >= self.scroll_offset + visible_height {
        self.scroll_offset = self.selected - visible_height + 1;
    }
    if self.selected < self.scroll_offset {
        self.scroll_offset = self.selected;
    }
}
```

Then in `search_ui.rs`, call `app.adjust_scroll(visible_height)` in `render_list` before calculating the visible window. Since `app` is passed as `&SearchApp`, you'll need to change `render_search` and `render_list` to take `&mut SearchApp`.

Alternatively, keep it simpler: calculate scroll in `move_down`/`move_up` by storing a `visible_height` field that gets updated during render. The simplest approach: just pass a `visible_height` to the event handler, or store it on the app during render.

**Simpler approach:** Store `visible_height` on `SearchApp`, set it during render:

In `search_app.rs`, add field:
```rust
pub visible_height: usize,
```
Initialize to `0` in `new()`.

In `search_ui.rs` `render_list`, set it:
```rust
app.visible_height = inner.height as usize;
```
(This requires `&mut SearchApp` in render functions.)

In `move_down`:
```rust
pub fn move_down(&mut self) {
    if self.selected + 1 < self.results.len() {
        self.selected += 1;
        if self.visible_height > 0 && self.selected >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected - self.visible_height + 1;
        }
    }
}
```

**Step 2: Update render function signatures to take `&mut SearchApp`**

In `search_ui.rs`, change:
- `render_search(frame: &mut Frame, app: &SearchApp)` → `render_search(frame: &mut Frame, app: &mut SearchApp)`
- `render_list(frame: &mut Frame, app: &SearchApp, area: Rect)` → `render_list(frame: &mut Frame, app: &mut SearchApp, area: Rect)`

In `render_list`, before building lines:
```rust
app.visible_height = inner.height as usize;
```

In `search.rs` main loop, change to `&mut app`.

**Step 3: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/tui/search_app.rs src/tui/search_ui.rs src/commands/search.rs
git commit -m "feat(search-tui): handle scrolling for long result lists"
```

---

### Task 6: Manual smoke test

**Step 1: Build and run the search with interactive mode**

Run: `cargo build && ./target/debug/ion search brainstorming -i`

**Expected behavior:**
- Two-column layout appears in alternate screen
- Left panel shows results sorted by stars with `>` cursor on first item
- Right panel shows detail for selected result (owner, stars, description, skill desc, install cmd)
- Up/Down (or j/k) navigates the list, right panel updates reactively
- Enter exits and runs `ion add <source>`
- q or Esc exits without action
- Scrolling works if results exceed visible height

**Step 2: Test edge cases**
- Search with no results: `cargo run -- search xyznonexistent123 -i` — should show "No installable results"
- Search with 1 result — should show single item, Enter works
- Resize terminal while TUI is running — layout should adapt

**Step 3: Commit any fixes from smoke testing**

---

### Task 7: Clean up dialoguer dependency (if unused)

**Files:**
- Modify: `Cargo.toml` (remove `dialoguer` if unused elsewhere)

**Step 1: Check if dialoguer is used anywhere**

Run: `rg 'dialoguer' --type rust`

If only the old `search.rs` used it (now removed), remove from `Cargo.toml`.

**Step 2: Remove if unused**

In `Cargo.toml`, remove the `dialoguer` line from `[dependencies]`.

**Step 3: Run `cargo check && cargo test`**

Run: `cargo check && cargo test 2>&1 | tail -10`
Expected: compiles and all tests pass

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: remove unused dialoguer dependency"
```

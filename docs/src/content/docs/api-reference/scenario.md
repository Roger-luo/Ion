---
title: "scenario"
description: "Define and test CLI behavior scenarios under controlled terminal conditions."
order: 100
---

*Version 0.2.0*

# scenario

Define and test CLI behavior scenarios under controlled terminal conditions.

`scenario` provides infrastructure for running CLI applications across
different terminal environments — with or without a TTY, at various
terminal widths, with or without color support.

## Quick Start

```no_run
use scenario::{Scenario, Terminal};

// Run a command with piped stdio (no TTY)
let output = Scenario::new("echo")
    .arg("hello")
    .run()
    .unwrap();
assert!(output.stdout().contains("hello"));

// Run in a real PTY with specific dimensions
let output = Scenario::new("my-cli")
    .args(["--help"])
    .terminal(Terminal::pty(80, 24))
    .run()
    .unwrap();

// Interactive session
let mut session = Scenario::new("my-cli")
    .args(["init"])
    .terminal(Terminal::pty(80, 24))
    .spawn()
    .unwrap();
session.expect("Choose:").unwrap();
session.send_line("default").unwrap();
let output = session.wait().unwrap();
```

## Modules

| Module | Description |
|--------|-------------|
| [manifest](/docs/api-reference/scenario/manifest) | Parsing for `template.toml` manifest files. |
| [screen](/docs/api-reference/scenario/screen) |  |

## Re-exports

- `pub use error::Error` as **Error**
- `pub use key::Key` as **Key**
- `pub use output::Output` as **Output**
- `pub use project::Project` as **Project**
- `pub use project::ProjectBuilder` as **ProjectBuilder**
- `pub use scenario::Scenario` as **Scenario**
- `pub use scenario::SessionConfig` as **SessionConfig**
- `pub use scenario::Terminal` as **Terminal**
- `pub use screen::ScreenBuffer` as **ScreenBuffer**
- `pub use session::Session` as **Session**


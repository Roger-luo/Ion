---
title: "scenario::screen"
description: ""
order: 999
---

## ScreenBuffer

A virtual terminal screen buffer that interprets ANSI escape sequences.

Maintains a grid of characters representing what would be visible on
a terminal of the given dimensions after processing raw PTY output.

*…and private fields*

### Methods

#### `new`

```rust
pub fn new(rows: usize, cols: usize) -> Self
```

Create a new empty screen buffer with the given dimensions.

#### `process`

```rust
pub fn process(&mut self, raw_bytes: &[u8])
```

Feed raw PTY output through the VT parser, updating the grid.

#### `lines`

```rust
pub fn lines(&self) -> Vec<String>
```

Return the current screen content, one `String` per row,
with trailing spaces trimmed.


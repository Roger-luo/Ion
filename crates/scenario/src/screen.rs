use vte::{Params, Perform};

/// A virtual terminal screen buffer that interprets ANSI escape sequences.
///
/// Maintains a grid of characters representing what would be visible on
/// a terminal of the given dimensions after processing raw PTY output.
pub struct ScreenBuffer {
    grid: Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
    rows: usize,
    cols: usize,
}

impl ScreenBuffer {
    /// Create a new empty screen buffer with the given dimensions.
    pub fn new(rows: usize, cols: usize) -> Self {
        ScreenBuffer {
            grid: vec![vec![' '; cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
        }
    }

    /// Feed raw PTY output through the VT parser, updating the grid.
    pub fn process(&mut self, raw_bytes: &[u8]) {
        let mut parser = vte::Parser::new();
        parser.advance(self, raw_bytes);
    }

    /// Return the current screen content, one `String` per row,
    /// with trailing spaces trimmed.
    pub fn lines(&self) -> Vec<String> {
        self.grid
            .iter()
            .map(|row| {
                let s: String = row.iter().collect();
                s.trim_end().to_string()
            })
            .collect()
    }

    fn scroll_up(&mut self) {
        self.grid.remove(0);
        self.grid.push(vec![' '; self.cols]);
    }

    fn clear_line_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols {
            self.grid[self.cursor_row][col] = ' ';
        }
    }

    fn clear_line_to_cursor(&mut self) {
        for col in 0..=self.cursor_col.min(self.cols - 1) {
            self.grid[self.cursor_row][col] = ' ';
        }
    }

    fn clear_whole_line(&mut self) {
        for col in 0..self.cols {
            self.grid[self.cursor_row][col] = ' ';
        }
    }

    fn clear_screen(&mut self) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.grid[row][col] = ' ';
            }
        }
    }

    fn clear_screen_from_cursor(&mut self) {
        // Clear from cursor to end of current line
        self.clear_line_from_cursor();
        // Clear all lines below
        for row in (self.cursor_row + 1)..self.rows {
            for col in 0..self.cols {
                self.grid[row][col] = ' ';
            }
        }
    }

    fn clear_screen_to_cursor(&mut self) {
        // Clear from start of screen to cursor
        for row in 0..self.cursor_row {
            for col in 0..self.cols {
                self.grid[row][col] = ' ';
            }
        }
        self.clear_line_to_cursor();
    }
}

impl Perform for ScreenBuffer {
    fn print(&mut self, c: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.grid[self.cursor_row][self.cursor_col] = c;
            self.cursor_col += 1;
            if self.cursor_col >= self.cols {
                self.cursor_col = 0;
                if self.cursor_row + 1 >= self.rows {
                    self.scroll_up();
                } else {
                    self.cursor_row += 1;
                }
            }
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // Backspace
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            // Tab
            0x09 => {
                let next_tab = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next_tab.min(self.cols - 1);
            }
            // Newline (line feed)
            0x0A => {
                if self.cursor_row + 1 >= self.rows {
                    self.scroll_up();
                } else {
                    self.cursor_row += 1;
                }
            }
            // Carriage return
            0x0D => {
                self.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params: Vec<u16> = params.iter().map(|p| p[0]).collect();

        match action {
            // Cursor Up
            'A' => {
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            // Cursor Down
            'B' => {
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
            }
            // Cursor Forward
            'C' => {
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
            }
            // Cursor Back
            'D' => {
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            // Cursor Position (CUP) / Home
            'H' | 'f' => {
                let row = params.first().copied().unwrap_or(1).max(1) as usize;
                let col = params.get(1).copied().unwrap_or(1).max(1) as usize;
                self.cursor_row = (row - 1).min(self.rows - 1);
                self.cursor_col = (col - 1).min(self.cols - 1);
            }
            // Erase in Display
            'J' => {
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.clear_screen_from_cursor(),
                    1 => self.clear_screen_to_cursor(),
                    2 | 3 => self.clear_screen(),
                    _ => {}
                }
            }
            // Erase in Line
            'K' => {
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.clear_line_from_cursor(),
                    1 => self.clear_line_to_cursor(),
                    2 => self.clear_whole_line(),
                    _ => {}
                }
            }
            // SGR (colors/styles) - ignore
            'm' => {}
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text_output() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Hello, world!");
        let lines = screen.lines();
        assert_eq!(lines[0], "Hello, world!");
        assert_eq!(lines[1], "");
    }

    #[test]
    fn test_newline() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Line 1\r\nLine 2");
        let lines = screen.lines();
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
    }

    #[test]
    fn test_cursor_position() {
        let mut screen = ScreenBuffer::new(5, 20);
        // Move to row 2, col 5 (1-based) then write
        screen.process(b"\x1b[2;5HWorld");
        let lines = screen.lines();
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "    World");
    }

    #[test]
    fn test_cursor_home() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"XXXXX\x1b[HHello");
        let lines = screen.lines();
        assert_eq!(lines[0], "Hello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"ABCDEF\x1b[3DX");
        let lines = screen.lines();
        // "ABCDEF", then move left 3, write X over 'D'
        assert_eq!(lines[0], "ABCXEF");
    }

    #[test]
    fn test_clear_to_end_of_line() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Hello, world!\x1b[1;6H\x1b[K");
        let lines = screen.lines();
        // Cursor at col 6 (1-based) = col 5 (0-based), clear from there
        assert_eq!(lines[0], "Hello");
    }

    #[test]
    fn test_clear_whole_line() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Hello, world!\x1b[2K");
        let lines = screen.lines();
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_clear_screen() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Line 1\r\nLine 2\x1b[2J");
        let lines = screen.lines();
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "");
    }

    #[test]
    fn test_carriage_return_overwrites() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"Hello!\rWorld");
        let lines = screen.lines();
        assert_eq!(lines[0], "World!");
    }

    #[test]
    fn test_backspace() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"ABC\x08X");
        let lines = screen.lines();
        assert_eq!(lines[0], "ABX");
    }

    #[test]
    fn test_sgr_codes_ignored() {
        let mut screen = ScreenBuffer::new(5, 20);
        screen.process(b"\x1b[31mRed\x1b[0m Normal");
        let lines = screen.lines();
        assert_eq!(lines[0], "Red Normal");
    }

    #[test]
    fn test_scroll_up() {
        let mut screen = ScreenBuffer::new(3, 10);
        screen.process(b"Line 1\r\nLine 2\r\nLine 3\r\nLine 4");
        let lines = screen.lines();
        // Line 1 should have scrolled off
        assert_eq!(lines[0], "Line 2");
        assert_eq!(lines[1], "Line 3");
        assert_eq!(lines[2], "Line 4");
    }

    #[test]
    fn test_line_wrap() {
        let mut screen = ScreenBuffer::new(3, 5);
        screen.process(b"ABCDEFGH");
        let lines = screen.lines();
        assert_eq!(lines[0], "ABCDE");
        assert_eq!(lines[1], "FGH");
    }
}

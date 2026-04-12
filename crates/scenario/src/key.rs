/// Common terminal keys for interactive testing.
///
/// Each variant maps to the corresponding ANSI escape sequence or control
/// character that a real terminal would send when the key is pressed.
///
/// # Example
///
/// ```
/// use scenario::Key;
///
/// assert_eq!(Key::Enter.to_bytes(), b"\r");
/// assert_eq!(Key::Up.to_bytes(), b"\x1b[A");
/// assert_eq!(Key::Char('a').to_bytes(), vec![b'a']);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    CtrlC,
    CtrlD,
    CtrlZ,
    Char(char),
}

impl Key {
    /// Return the byte sequence for this key as a terminal would send it.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Key::Up => b"\x1b[A".to_vec(),
            Key::Down => b"\x1b[B".to_vec(),
            Key::Right => b"\x1b[C".to_vec(),
            Key::Left => b"\x1b[D".to_vec(),
            Key::Enter => b"\r".to_vec(),
            Key::Escape => b"\x1b".to_vec(),
            Key::Tab => b"\t".to_vec(),
            Key::Backspace => b"\x7f".to_vec(),
            Key::Delete => b"\x1b[3~".to_vec(),
            Key::Home => b"\x1b[H".to_vec(),
            Key::End => b"\x1b[F".to_vec(),
            Key::PageUp => b"\x1b[5~".to_vec(),
            Key::PageDown => b"\x1b[6~".to_vec(),
            Key::CtrlC => b"\x03".to_vec(),
            Key::CtrlD => b"\x04".to_vec(),
            Key::CtrlZ => b"\x1a".to_vec(),
            Key::Char(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_keys() {
        assert_eq!(Key::Up.to_bytes(), b"\x1b[A");
        assert_eq!(Key::Down.to_bytes(), b"\x1b[B");
        assert_eq!(Key::Right.to_bytes(), b"\x1b[C");
        assert_eq!(Key::Left.to_bytes(), b"\x1b[D");
    }

    #[test]
    fn enter_escape_tab() {
        assert_eq!(Key::Enter.to_bytes(), b"\r");
        assert_eq!(Key::Escape.to_bytes(), b"\x1b");
        assert_eq!(Key::Tab.to_bytes(), b"\t");
    }

    #[test]
    fn editing_keys() {
        assert_eq!(Key::Backspace.to_bytes(), b"\x7f");
        assert_eq!(Key::Delete.to_bytes(), b"\x1b[3~");
        assert_eq!(Key::Home.to_bytes(), b"\x1b[H");
        assert_eq!(Key::End.to_bytes(), b"\x1b[F");
    }

    #[test]
    fn page_keys() {
        assert_eq!(Key::PageUp.to_bytes(), b"\x1b[5~");
        assert_eq!(Key::PageDown.to_bytes(), b"\x1b[6~");
    }

    #[test]
    fn control_keys() {
        assert_eq!(Key::CtrlC.to_bytes(), b"\x03");
        assert_eq!(Key::CtrlD.to_bytes(), b"\x04");
        assert_eq!(Key::CtrlZ.to_bytes(), b"\x1a");
    }

    #[test]
    fn char_ascii() {
        assert_eq!(Key::Char('a').to_bytes(), b"a");
        assert_eq!(Key::Char('Z').to_bytes(), b"Z");
        assert_eq!(Key::Char('0').to_bytes(), b"0");
    }

    #[test]
    fn char_multibyte() {
        let bytes = Key::Char('é').to_bytes();
        assert_eq!(bytes, "é".as_bytes());

        let bytes = Key::Char('🦀').to_bytes();
        assert_eq!(bytes, "🦀".as_bytes());
    }
}

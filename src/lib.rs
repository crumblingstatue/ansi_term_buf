#![warn(
    missing_docs,
    clippy::panic,
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::nursery
)]

//! A simple, minimal ANSI terminal emulator whose contents can be get as a string.

mod parser;

use parser::{AnsiParser, TermCmd};

/// Minimalistic ANSI terminal emulator.
///
/// Use [`feed`]to feed it data, [`contents_to_string`] to get its contents as a string.
///
/// [`feed`]: Self::feed
/// [`contents_to_string`]: Self::contents_to_string
pub struct Term {
    term_state: TermState,
    ansi_parser: AnsiParser,
}

struct TermState {
    width: u16,
    height: usize,
    cells: Vec<char>,
    cursor: Cursor,
}

impl TermState {
    fn new(width: u16) -> Self {
        Self {
            width,
            height: 0,
            cells: Vec::new(),
            cursor: Cursor::default(),
        }
    }
    fn contents_to_string(&self) -> String {
        let mut buf = String::with_capacity(self.width as usize * self.height);
        for y in 0..self.height {
            buf.extend(self.line_slice(y));
            buf.push('\n');
        }
        buf
    }
    fn line_slice(&self, y: usize) -> &[char] {
        let from = y * self.width as usize;
        let to = from + self.width as usize;
        &self.cells[from..to]
    }
    fn put_char(&mut self, ch: char) {
        self.extend_while_cursor_past();
        self.cells[self.cursor.index(self.width)] = ch;
        self.cursor.x += 1;
        if self.cursor.x >= self.width {
            self.cursor.x = 0;
            self.cursor.y += 1;
        }
    }
    fn extend(&mut self) {
        self.cells
            .extend(std::iter::repeat_n(' ', self.width as usize));
        self.height += 1;
    }
    fn extend_while_cursor_past(&mut self) {
        while self.cursor.y >= self.height {
            self.extend();
        }
    }
    fn erase_from_cursor_to_eol(&mut self) {
        for x in self.cursor.x..self.width {
            let idx = self.cursor.y * self.width as usize + x as usize;
            if idx >= self.cells.len() {
                break;
            }
            self.cells[idx] = ' ';
        }
    }
    fn clear(&mut self, mode: u8) {
        if mode != 2 {
            log::warn!("Clear mode {mode} not implemented.");
        }
        self.cells.fill(' ');
    }
}

#[derive(Default)]
struct Cursor {
    x: u16,
    y: usize,
}

impl Cursor {
    const fn index(&self, width: u16) -> usize {
        self.y * width as usize + self.x as usize
    }
}

impl Term {
    /// Create a new terminal with the specified width
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            term_state: TermState::new(width),
            ansi_parser: AnsiParser::default(),
        }
    }
    /// Feed bytes to the terminal, updating its state
    pub fn feed(&mut self, data: &[u8]) {
        self.ansi_parser.advance(data, |cmd| match cmd {
            TermCmd::PutChar(c) => self.term_state.put_char(c),
            TermCmd::CarriageReturn => self.term_state.cursor.x = 0,
            TermCmd::LineFeed => self.term_state.cursor.y += 1,
            TermCmd::CursorUp(n) => {
                self.term_state.cursor.y = self.term_state.cursor.y.saturating_sub(n as usize);
            }
            TermCmd::CursorDown(n) => {
                self.term_state.cursor.y += n as usize;
            }
            TermCmd::CursorLeft(n) => {
                self.term_state.cursor.x = self.term_state.cursor.x.saturating_sub(u16::from(n));
            }
            TermCmd::CursorRight(n) => {
                self.term_state.cursor.x += u16::from(n);
            }
            TermCmd::CursorCrUp(n) => {
                self.term_state.cursor.y = self.term_state.cursor.y.saturating_sub(n as usize);
                self.term_state.cursor.x = 0;
            }
            TermCmd::CursorCrDown(n) => {
                self.term_state.cursor.y += n as usize;
                self.term_state.cursor.x = 0;
            }
            TermCmd::CursorSet { x, y } => {
                self.term_state.cursor.x = x.into();
                self.term_state.cursor.y = y as usize;
            }
            TermCmd::EraseFromCursorToEol => self.term_state.erase_from_cursor_to_eol(),
            TermCmd::Clear(mode) => self.term_state.clear(mode),
        });
    }
    /// Completely reset the terminal to its initial state
    pub fn reset(&mut self) {
        self.term_state.cursor = Cursor::default();
        self.term_state.cells.clear();
        self.term_state.height = 0;
        self.ansi_parser = AnsiParser::default();
    }
    /// Get the contents of the terminal as a string
    #[must_use]
    pub fn contents_to_string(&self) -> String {
        self.term_state.contents_to_string()
    }
    /// Returns whether the terminal buffer is "empty" (nothing has been written to it yet)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.term_state.cells.is_empty()
    }
}

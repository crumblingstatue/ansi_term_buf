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
    state: TermState,
    ansi_parser: AnsiParser,
    /// The last rendered contents when a sync update was completed
    last_contents: String,
}

struct TermState {
    width: u16,
    height: usize,
    cells: Vec<char>,
    cursor: Cursor,
    /// Whether the buffer is currently updating according to synchronized output protocol
    sync_update: bool,
}

impl TermState {
    fn new(width: u16) -> Self {
        Self {
            width,
            height: 0,
            cells: Vec::new(),
            cursor: Cursor::default(),
            sync_update: false,
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
        self.add_row_while_cursor_past();
        self.cells[self.cursor.index(self.width)] = ch;
        self.cursor.x += 1;
        if self.cursor.x >= self.width {
            self.cursor.x = 0;
            self.cursor.y += 1;
        }
    }
    fn add_row(&mut self) {
        self.cells
            .extend(std::iter::repeat_n(' ', self.width as usize));
        self.height += 1;
    }
    fn add_row_while_cursor_past(&mut self) {
        while self.cursor.y >= self.height {
            self.add_row();
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
        // Clear rather than fill with ' ' in order to avoid unbounded growth
        // due to how self.add_row() works
        self.cells.clear();
        self.height = 0;
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
            state: TermState::new(width),
            ansi_parser: AnsiParser::default(),
            last_contents: String::new(),
        }
    }
    /// Feed bytes to the terminal, updating its state
    pub fn feed(&mut self, data: &[u8]) {
        self.ansi_parser.advance(data, |cmd| match cmd {
            TermCmd::PutChar(c) => self.state.put_char(c),
            TermCmd::CarriageReturn => self.state.cursor.x = 0,
            TermCmd::LineFeed => self.state.cursor.y += 1,
            TermCmd::CursorUp(n) => {
                self.state.cursor.y = self.state.cursor.y.saturating_sub(n as usize);
            }
            TermCmd::CursorDown(n) => {
                self.state.cursor.y += n as usize;
            }
            TermCmd::CursorLeft(n) => {
                self.state.cursor.x = self.state.cursor.x.saturating_sub(u16::from(n));
            }
            TermCmd::CursorRight(n) => {
                self.state.cursor.x += u16::from(n);
            }
            TermCmd::CursorCrUp(n) => {
                self.state.cursor.y = self.state.cursor.y.saturating_sub(n as usize);
                self.state.cursor.x = 0;
            }
            TermCmd::CursorCrDown(n) => {
                self.state.cursor.y += n as usize;
                self.state.cursor.x = 0;
            }
            TermCmd::CursorSet { x, y } => {
                self.state.cursor.x = (x.saturating_sub(1)).into();
                self.state.cursor.y = y.saturating_sub(1) as usize;
            }
            TermCmd::EraseFromCursorToEol => self.state.erase_from_cursor_to_eol(),
            TermCmd::Clear(mode) => self.state.clear(mode),
            TermCmd::BeginSyncUpdate => self.state.sync_update = true,
            TermCmd::EndSyncUpdate => {
                self.state.sync_update = false;
                self.last_contents = self.state.contents_to_string();
            }
        });
    }
    /// Completely reset the terminal to its initial state
    pub fn reset(&mut self) {
        self.state.cursor = Cursor::default();
        self.state.cells.clear();
        self.state.height = 0;
        self.ansi_parser = AnsiParser::default();
    }
    /// Get the contents of the terminal as a string
    #[must_use]
    pub fn contents_to_string(&self) -> String {
        if self.state.sync_update {
            self.last_contents.clone()
        } else {
            self.state.contents_to_string()
        }
    }
    /// Returns whether the terminal buffer is "empty" (nothing has been written to it yet)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.state.cells.is_empty()
    }
}

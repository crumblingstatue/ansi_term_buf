use std::iter::zip;

const ESC: char = 0x1b as char;

#[derive(Debug)]
pub struct AnsiParser {
    status: Status,
    param_bytes: Vec<u8>,
}

trait ParamBytesExt {
    fn parse_first(&self) -> Option<u8>;
    fn parse<const N: usize>(&self) -> Option<[u8; N]>;
}

impl ParamBytesExt for Vec<u8> {
    fn parse_first(&self) -> Option<u8> {
        // NOTE: Parameters are text that must be parsed into a number
        let s = std::str::from_utf8(self).ok()?;
        let mut args = s.split(';');
        args.next()?.parse().ok()
    }
    fn parse<const N: usize>(&self) -> Option<[u8; N]> {
        let s = std::str::from_utf8(self).ok()?;
        let args = s.split(';');
        let mut arr = [0; N];
        for (arg, out) in zip(args, &mut arr) {
            *out = arg.parse().ok()?;
        }
        Some(arr)
    }
}

impl Default for AnsiParser {
    fn default() -> Self {
        Self {
            status: Status::Init,
            param_bytes: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Status {
    Init,
    Esc,
    ControlSeqStart,
}

#[derive(Debug)]
pub enum TermCmd {
    PutChar(char),
    CarriageReturn,
    LineFeed,
    /// Move cursor up this many lines
    CursorUp(u8),
    /// Move cursor down this many lines
    CursorDown(u8),
    /// Move cursor left this many columns
    CursorLeft(u8),
    /// Move cursor right this many columns
    CursorRight(u8),
    /// Beginning of line, this many lines down
    CursorCrDown(u8),
    /// Beginning of line, this many lines up
    CursorCrUp(u8),
    /// Set the cursor to (x, y)
    CursorSet {
        x: u8,
        y: u8,
    },
    /// Erase from cursor to the end of line
    EraseFromCursorToEol,
    /// Clear the screen, in the manner specified by the argument
    Clear(u8),
    /// Begin synchronized update
    BeginSyncUpdate,
    /// End synchronized update
    EndSyncUpdate,
}

impl AnsiParser {
    pub fn advance(&mut self, bytes: &[u8], mut term_callback: impl FnMut(TermCmd)) {
        for chnk in bytes.utf8_chunks() {
            for ch in chnk.valid().chars() {
                match self.status {
                    Status::Init => match ch {
                        ESC => self.status = Status::Esc,
                        '\r' => {
                            term_callback(TermCmd::CarriageReturn);
                        }
                        '\n' => term_callback(TermCmd::LineFeed),
                        c => term_callback(TermCmd::PutChar(c)),
                    },
                    Status::Esc => {
                        match ch {
                            '=' => {
                                // Unknown, ignore
                                self.status = Status::Init;
                            }
                            '[' => {
                                // Control sequence start
                                self.status = Status::ControlSeqStart;
                            }
                            _ => log::error!("Unexpected ansi [{:x}]", ch as u32),
                        }
                    }
                    Status::ControlSeqStart => {
                        match ch as u32 {
                            0x30..=0x3F => {
                                self.param_bytes.push(ch as u8);
                            }
                            0x40..=0x7E => {
                                // Terminator byte
                                match ch {
                                    // color/etc, ignore
                                    'm' => {}
                                    'K' => {
                                        term_callback(TermCmd::EraseFromCursorToEol);
                                    }
                                    'A' => {
                                        // Move cursor up N lines
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorUp(n.unwrap_or(1)));
                                    }
                                    'B' => {
                                        // Move down N lines
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorDown(n.unwrap_or(1)));
                                    }
                                    'C' => {
                                        // Move cursor right N columns
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorRight(n.unwrap_or(1)));
                                    }
                                    'D' => {
                                        // Move cursor left N columns
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorLeft(n.unwrap_or(1)));
                                    }
                                    'E' => {
                                        // Beginning of next line, N lines down
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorCrDown(n.unwrap_or(1)));
                                    }
                                    'F' => {
                                        // Beginning of prev line, N lines up
                                        let n = self.param_bytes.parse_first();
                                        term_callback(TermCmd::CursorCrUp(n.unwrap_or(1)));
                                    }
                                    'H' => {
                                        let [x, y] = self.param_bytes.parse().unwrap_or([1, 1]);
                                        term_callback(TermCmd::CursorSet { x, y });
                                    }
                                    'J' => {
                                        let mode = self.param_bytes.parse_first().unwrap_or(2);
                                        term_callback(TermCmd::Clear(mode));
                                    }
                                    'h' => {
                                        if self.param_bytes == b"?2026" {
                                            term_callback(TermCmd::BeginSyncUpdate);
                                        }
                                    }
                                    'l' => {
                                        if self.param_bytes == b"?2026" {
                                            term_callback(TermCmd::EndSyncUpdate);
                                        }
                                    }
                                    etc => {
                                        log::warn!(
                                            "Ignored control code: '{ch}', params: {params:?}",
                                            ch = etc,
                                            params = std::str::from_utf8(&self.param_bytes)
                                        );
                                    }
                                }
                                self.status = Status::Init;
                                self.param_bytes.clear();
                            }
                            _ => log::error!("Unexpected ansi <{:x}>", ch as u32),
                        }
                    }
                }
            }
        }
    }
}

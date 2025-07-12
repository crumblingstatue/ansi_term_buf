const ESC: u8 = 0x1b;

#[derive(Debug)]
pub struct AnsiParser {
    status: Status,
    params: Vec<u8>,
}

impl Default for AnsiParser {
    fn default() -> Self {
        Self {
            status: Status::Init,
            params: Vec::new(),
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
    PutChar(u8),
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
    /// Erase from cursor to the end of line
    EraseFromCursorToEol,
}

impl AnsiParser {
    pub fn advance(&mut self, bytes: &[u8], mut term_callback: impl FnMut(TermCmd)) {
        for &byte in bytes {
            match self.status {
                Status::Init => match byte {
                    ESC => self.status = Status::Esc,
                    b'\r' => {
                        term_callback(TermCmd::CarriageReturn);
                    }
                    b'\n' => term_callback(TermCmd::LineFeed),
                    c => term_callback(TermCmd::PutChar(c)),
                },
                Status::Esc => {
                    match byte {
                        b'=' => {
                            // Unknown, ignore
                            self.status = Status::Init;
                        }
                        b'[' => {
                            // Control sequence start
                            self.status = Status::ControlSeqStart;
                        }
                        _ => log::error!("Unexpected ansi [{byte:x}]"),
                    }
                }
                Status::ControlSeqStart => {
                    match byte {
                        0x30..=0x3F => {
                            self.params.push(byte);
                            log::debug!("pushed param, now we have {:?}", self.params);
                        }
                        0x40..=0x7E => {
                            // Terminator byte
                            match byte {
                                // color/etc, ignore
                                b'm' => {}
                                b'K' => {
                                    term_callback(TermCmd::EraseFromCursorToEol);
                                }
                                b'A' => {
                                    // Move cursor up N lines
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorUp(n.cloned().unwrap_or(1)));
                                }
                                b'B' => {
                                    // Move down N lines
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorDown(n.cloned().unwrap_or(1)));
                                }
                                b'C' => {
                                    // Move cursor right N columns
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorRight(n.cloned().unwrap_or(1)));
                                }
                                b'D' => {
                                    // Move cursor left N columns
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorLeft(n.cloned().unwrap_or(1)));
                                }
                                b'E' => {
                                    // Beginning of next line, N lines down
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorCrDown(n.cloned().unwrap_or(1)));
                                }
                                b'F' => {
                                    // Beginning of prev line, N lines up
                                    let n = self.params.first();
                                    term_callback(TermCmd::CursorCrUp(n.cloned().unwrap_or(1)));
                                }
                                etc => {
                                    log::warn!(
                                        "Ignored control code (ch, hex, dec): '{ch}', {etc:X?}, {etc}, params(hex, dec): {params:X?}, {params:?}",
                                        ch = etc as char,
                                        params = self.params
                                    );
                                }
                            }
                            self.status = Status::Init;
                            self.params.clear();
                        }
                        _ => log::error!("Unexpected ansi <{byte:x}>"),
                    }
                }
            }
        }
    }
}

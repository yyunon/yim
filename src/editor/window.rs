pub use crate::editor::cursor::Cursor;
pub use crate::editor::cursor::Terminal;

use super::terminal;

pub struct Window {
    cursor: Cursor,
    n_rows: usize,
    n_cols: usize,
}

impl Window {
    pub(crate) fn new(cursor: Cursor) -> Self {
        Self {
            cursor: cursor,
            n_rows: 0,
            n_cols: 0,
        }
    }
    pub(crate) fn init_window(&mut self, terminal: &mut Terminal) {
        self.set_window_size(terminal);
    }
    pub(crate) fn set_window_size(&mut self, terminal: &mut Terminal) {
        let (cols, rows) = terminal.term_size();
        if cols == 0 {
            self.calculate_window(terminal)
        } else {
            self.cursor.rows(rows);
            self.cursor.cols(cols);
        }
    }
    pub(crate) fn calculate_window(&mut self, terminal: &mut Terminal) {
        terminal.write(b"\x1B[6n");

        terminal.flush();

        let mut buffer = [0u8; 32];
        //print!("{}[6n", 27 as char);
        let mut i = 0;
        //^[[5;1R
        while i < buffer.len() - 1 {
            terminal.read(&mut buffer);
            if buffer[i] == b'R' {
                break;
            }
            i += 1;
        }
        buffer[i] = b'\0';

        if buffer[1] != b'[' {
            log::error!("Couldn't parse device status report");
            return;
        }
        self.cursor.rows = buffer[2] as usize;
        self.cursor.cols = buffer[6] as usize;
    }
}

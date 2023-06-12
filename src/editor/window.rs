use std::cell::RefCell;
use std::rc::Rc;

pub use crate::editor::cursor::Cursor;
pub use crate::editor::cursor::Terminal;

pub struct Window {
    cursor: Rc<RefCell<Cursor>>,
    terminal: Rc<RefCell<Terminal>>,
    n_rows: usize,
    n_cols: usize,
}

impl Window {
    pub(crate) fn new(cursor: &Rc<RefCell<Cursor>>, terminal: &Rc<RefCell<Terminal>>) -> Self {
        Self {
            cursor: cursor.clone(),
            terminal: terminal.clone(),
            n_rows: 0,
            n_cols: 0,
        }
    }
    pub(crate) fn init_window(&mut self) {
        self.set_window_size();
    }
    pub(crate) fn set_window_size(&mut self) {
        let (cols, rows) = self.terminal.borrow_mut().term_size();
        if cols == 0 {
            self.calculate_window()
        } else {
            self.cursor.borrow_mut().rows(rows);
            self.cursor.borrow_mut().cols(cols);
        }
    }
    pub(crate) fn calculate_window(&mut self) {
        self.terminal.borrow_mut().write(b"\x1B[6n");

        self.terminal.borrow_mut().flush();

        let mut buffer = [0u8; 32];
        //print!("{}[6n", 27 as char);
        let mut i = 0;
        //^[[5;1R
        while i < buffer.len() - 1 {
            self.terminal.borrow_mut().read(&mut buffer);
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
        self.cursor.borrow_mut().rows = buffer[2] as usize;
        self.cursor.borrow_mut().cols = buffer[6] as usize;
    }
}

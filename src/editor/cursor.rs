use std::io::Write;

pub use crate::editor::constants::*;
pub use crate::editor::terminal::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Cursor {
    pub c_x: usize,
    pub c_y: usize,
    pub rows: usize,
    pub cols: usize,
    pub row_offset: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            c_x: 0,
            c_y: 0,
            rows: 0,
            cols: 0,
            row_offset: 0,
        }
    }
    pub fn clear(&mut self) {
        self.c_x = 0;
        self.c_y = 0;
        self.rows = 0;
        self.cols = 0;
        self.row_offset = 0;
    }
    pub(crate) fn x(&mut self, d: usize) {
        self.c_x = d
    }
    pub(crate) fn y(&mut self, d: usize) {
        self.c_y = d
    }
    pub(crate) fn rows(&mut self, d: usize) {
        self.rows = d
    }
    pub(crate) fn cols(&mut self, d: usize) {
        self.cols = d
    }
    pub(crate) fn calculate_row_offset(&mut self) {
        if self.c_y < self.row_offset {
            self.row_offset = self.c_y;
        } else if self.c_y >= self.row_offset + self.rows {
            self.row_offset = self.c_y - self.rows + 1;
        }
    }
    // Gets the cursor returns the location in the file
    pub(crate) fn calculate_file_index(&self, new_lines: &Vec<i32>, x: usize, y: usize) -> usize {
        //We know that new lines array is sorted as that is the wau we insert
        let (il, _) = self.calculate_row_of_insert_indices(y, new_lines);
        il + x
    }
    // Gets the display index row axis index and return row printable c_x, c_y
    pub(crate) fn calculate_row_of_insert_indices(
        &self,
        i: usize,
        new_lines: &Vec<i32>,
    ) -> (usize, usize) {
        if i >= new_lines.len() {
            return (0, 0);
        }
        let index_r = new_lines[i] as usize;
        let mut index_l = 0;
        if i != 0 {
            index_l = new_lines[i - 1] + 1;
        }
        (index_l as usize, index_r as usize)
    }
    pub(crate) fn move_cursor(
        &mut self,
        new_lines: &Vec<i32>,
        direction: CursorDirections,
        offset: usize,
    ) -> Result<(), ()> {
        // TODO: Make here better A lot of repetittions
        let mut row_insert_size = 0;
        if self.c_y < self.rows {
            let (index_l, index_r) =
                self.calculate_row_of_insert_indices(self.c_y as usize, &new_lines);
            row_insert_size = index_r - index_l;
        }
        match direction {
            CursorDirections::LineBegin => self.c_x = 0,
            CursorDirections::LineEnd => self.c_x = row_insert_size,
            CursorDirections::Left => {
                if self.c_x != 0 {
                    self.c_x -= offset
                } else if self.c_y > 0 {
                    self.move_cursor(new_lines, CursorDirections::Up, offset);
                    //self.c_y -= offset;
                    let (index_l, index_r) =
                        self.calculate_row_of_insert_indices(self.c_y, &new_lines);
                    //row_insert_size = self.data.buffer[index_l..index_r].len();
                    row_insert_size = index_r - index_l;
                    if offset > row_insert_size {
                        self.c_x = 0
                    } else {
                        self.c_x = row_insert_size - offset + 1
                    }
                }
            }
            CursorDirections::Down => {
                if (new_lines.len() as i32) - (offset as i32) > self.c_y as i32 {
                    self.c_y += offset
                } else {
                    self.c_y = new_lines.len() - 1
                }
            }
            CursorDirections::Up => {
                if self.c_y > offset - 1 {
                    self.c_y -= offset
                } else {
                    self.c_y = 0
                }
            }
            CursorDirections::Right => {
                if row_insert_size != 0 && self.c_x < row_insert_size - offset + 1 {
                    self.c_x += offset
                } else if row_insert_size != 0
                    && self.c_x >= row_insert_size - offset + 1
                    && self.c_y != new_lines.len() - offset
                {
                    self.c_y += offset;
                    self.c_x = 0;
                }
            }
        }
        if self.c_y < self.rows {
            let (index_l, index_r) =
                self.calculate_row_of_insert_indices(self.c_y as usize, &new_lines);
            //row_insert_size = self.data.buffer[index_l..index_r].len();
            row_insert_size = index_r - index_l;
        }
        if row_insert_size != 0 && self.c_x > row_insert_size {
            self.c_x = row_insert_size - 1;
        }
        Ok(())
    }
    pub(crate) fn naive_move_cursor(
        &mut self,
        terminal: &mut Terminal,
        direction: CursorDirections,
        offset: usize,
    ) {
        // Does not calculate borders
        match direction {
            CursorDirections::LineBegin | CursorDirections::LineEnd => !unimplemented!(),
            CursorDirections::Up => {
                if terminal
                    .stdout
                    .write(format!("\x1B[{}A", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Down => {
                if terminal
                    .stdout
                    .write(format!("\x1B[{}B", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Right => {
                if terminal
                    .stdout
                    .write(format!("\x1B[{}C", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Left => {
                if terminal
                    .stdout
                    .write(format!("\x1B[{}D", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
        }
    }
    pub(crate) fn naive_move_cursor_2d(&mut self, terminal: &mut Terminal, x: usize, y: usize) {
        // Does not calculate borders
        if terminal
            .stdout
            .write(format!("\x1B[{};{}H", x, y).as_bytes())
            .unwrap() as u32
            != 5
        {
            log::error!("Couldn't go to command mode",);
        }
    }
    pub(crate) fn cursor_limits(&self, t: usize, mode: bool) -> usize {
        if t < 0 {
            return 0;
        }
        match mode {
            false => {
                if t > self.cols {
                    self.cols as usize
                } else {
                    t as usize
                }
            }
            true => {
                if t > self.rows {
                    self.rows as usize
                } else {
                    t as usize
                }
            }
        }
    }
}

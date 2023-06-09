use chrono::offset::Utc;
use chrono::DateTime;
use std::io::{BufReader, BufWriter, Read, Stdin, Stdout, Write};
use std::time::SystemTime;

mod buffer;
mod terminal;

pub use crate::editor::buffer::AppendBuffer;
pub use crate::editor::terminal::Terminal;

extern crate libc;

struct Keys;
impl Keys {
    fn is_cntrl(c: usize) -> bool {
        c == 127 || (c >= 0 && c <= 31)
    }
    fn cntrl(c: u8) -> u8 {
        c & 0x1f
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum CursorDirections {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum EditorCommands {
    Exit,
    Healthy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum EditorModes {
    Normal,
    Insert,
}

pub struct Editor {
    pub mode: EditorModes,
    state: EditorCommands,
    terminal: Terminal,
    rows: usize,
    cols: usize,
    c_x: usize,
    c_y: usize,
    row_offset: usize,
    append_buffer: AppendBuffer,
    data: AppendBuffer,
    files: String,
    status_message: String,
    status_message_time: SystemTime,
    dirty: i8,
    highlight_register: Vec<(usize, usize)>,
}
impl Editor {
    pub(crate) fn new(stdin: Stdin, stdout: Stdout) -> Self {
        Self {
            mode: EditorModes::Normal,
            state: EditorCommands::Healthy,
            terminal: Terminal::new(stdin, stdout),
            rows: 0,
            cols: 0,
            row_offset: 0,
            c_x: 0,
            c_y: 0,
            append_buffer: AppendBuffer::default(),
            data: AppendBuffer::default(),
            files: "".to_string(),
            status_message: "".to_string(),
            status_message_time: SystemTime::now(),
            dirty: 0,
            highlight_register: Vec::default(),
        }
    }
    pub(crate) fn init_editor(&mut self) {
        self.terminal.enable_raw_mode();
        self.c_x = 0;
        self.c_y = 0;
        self.set_window_size();
        self.rows -= 2;
    }
    pub(crate) fn launch_engine(&mut self) {
        loop {
            log::debug!("Mode {:?}", self.mode);
            log::debug!("C_X {}, C_Y {}, RO {}", self.x(), self.y(), self.ro());
            self.render();
            let option = self.process_key_press().unwrap();
            if option == EditorCommands::Exit {
                break;
            }
        }
    }
    pub(crate) fn x(&self) -> usize {
        self.c_x
    }
    pub(crate) fn y(&self) -> usize {
        self.c_y
    }
    pub(crate) fn ro(&self) -> usize {
        self.row_offset
    }
    pub(crate) fn open(&mut self, input_file: &str) -> std::io::Result<()> {
        self.files = input_file.to_string();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(input_file)?;
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut self.data.buffer)?;
        self.data.update_buffers();
        Ok(())
    }
    pub(crate) fn change_mode(&mut self, m: EditorModes) -> Option<EditorCommands> {
        self.mode = m;
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn set_window_size(&mut self) {
        let (cols, rows) = self.terminal.term_size();
        if cols == 0 {
            self.set_cursor_position()
        } else {
            self.rows = rows;
            self.cols = cols;
        }
        log::debug!("App Rows: {}, Columns: {}", self.rows, self.cols);
    }
    pub(crate) fn calculate_row_offset(&mut self) {
        if self.c_y < self.row_offset {
            self.row_offset = self.c_y;
        } else if self.c_y >= self.row_offset + self.rows {
            self.row_offset = self.c_y - self.rows + 1;
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
    // Gets the cursor returns the location in the file
    pub(crate) fn calculate_file_index(&self, x: usize, y: usize) -> usize {
        //We know that new lines array is sorted as that is the wau we insert
        let (il, ir) = self.calculate_row_of_insert_indices(y);
        log::debug!("{},{}", il, ir);
        return il + x;
    }
    // Gets the display index row axis index and return row printable c_x, c_y
    pub(crate) fn calculate_row_of_insert_indices(&self, i: usize) -> (usize, usize) {
        if i >= self.data.new_lines.len() {
            return (0, 0);
        }
        let index_r = self.data.new_lines[i] as usize;
        let mut index_l = 0;
        if i != 0 {
            index_l = self.data.new_lines[i - 1] + 1;
        }
        (index_l as usize, index_r as usize)
    }
    pub(crate) fn set_status_message(&mut self, ins: &str) {
        self.status_message = ins.to_string();
    }
    pub(crate) fn clear_status_message_from_editor(&mut self) {
        let status_len: usize = self.status_message.capacity();
        let mut cmd_buffer = String::new();
        cmd_buffer.push(b':' as char);
        for _i in 0..status_len {
            cmd_buffer.push(' ');
        }
        self.terminal.stdout.write(cmd_buffer.as_bytes()).unwrap();
    }
    pub(crate) fn draw(&mut self) {
        for _y in 0..self.rows {
            let file_row = _y + self.row_offset;
            if file_row >= self.data.new_lines.len() {
                self.append_buffer.append(b"~");
            } else {
                // TODO Ref here HANDLE COL limits
                let (index_l, index_r) = self.calculate_row_of_insert_indices(file_row as usize);
                // TODO Def very Bad
                for (_, (high_l, high_r)) in self.highlight_register.iter().enumerate() {
                    if high_l > &index_l {
                        self.append_buffer
                            .append(&self.data.buffer[index_l..*high_l]);
                        self.append_buffer.append(b"\x1B[0;33m"); //YELLOW
                        self.append_buffer
                            .append(&self.data.buffer[*high_l..*high_r]);
                        self.append_buffer.append(b"\x1B[0m");
                        self.append_buffer
                            .append(&self.data.buffer[*high_r..index_r]);
                    }
                }
                if self.highlight_register.len() == 0 {
                    self.append_buffer
                        .append(&self.data.buffer[index_l..index_r]);
                }
            }
            self.append_buffer.append(b"\x1B[K");
            self.append_buffer.append(b"\r\n");
        }
    }
    pub(crate) fn draw_message_bar(&mut self) {
        self.append_buffer.append(b"\x1B[K");
        self.append_buffer.append_str(&self.status_message);
    }
    pub(crate) fn draw_status_bar(&mut self) {
        self.append_buffer.append(b"\x1B[7m");
        //rstatus
        // ROW COUNT
        let mut status = String::new();
        let mut rstatus: String = format!(" [{}/{}] ", self.c_y + 1, self.rows);
        // DATE TIME
        let datetime: DateTime<Utc> = self.status_message_time.into();
        let dt_string = datetime.format("%T %d/%m/%Y").to_string();
        rstatus.push_str(&dt_string);
        // status
        // PRINT mode
        let mode = match self.mode {
            EditorModes::Insert => String::from("[--INSERT--]"),
            EditorModes::Normal => String::from("[--NORMAL--]"),
        };
        status.push_str(&mode);
        //FILE NAME
        if self.files.is_empty() {
            status.push_str("[No Name]")
        } else {
            status.push_str(&self.files);
        }
        // DIRTY
        status.push_str(match self.dirty {
            0 => "",
            1 => "(modified)",
            _ => unreachable!("modified or not modified"),
        });

        //WRITE STAT
        if status.len() > self.cols {
            self.append_buffer
                .append_str(&status[0..self.cols as usize]);
        } else {
            self.append_buffer.append_str(&status);
        }
        let mut len = 0;
        while len < self.cols {
            if self.cols - len - status.len() == rstatus.len() {
                self.append_buffer.append_str(&rstatus);
                break;
            } else {
                self.append_buffer.append(b" ");
                len += 1;
            }
        }
        self.append_buffer.append(b"\x1B[m");
        self.append_buffer.append(b"\r\n");
    }
    pub(crate) fn render(&mut self) {
        self.calculate_row_offset();
        self.append_buffer.append(b"\x1B[?25l");
        self.append_buffer.append(b"\x1B[H");
        self.draw();
        self.draw_status_bar();
        self.draw_message_bar();
        self.append_buffer.append_str(
            format!(
                "\x1B[{};{}H",
                (self.c_y - self.row_offset) + 1,
                self.c_x + 1
            )
            .as_str(),
        );
        self.append_buffer.append(b"\x1B[?25h");
        self.append_buffer.write(&mut self.terminal.stdout);
    }
    pub(crate) fn set_cursor_position(&mut self) {
        let mut buffer = [0u8; 32];
        //print!("{}[6n", 27 as char);
        if self.terminal.stdout.write(b"\x1B[6n").unwrap() != 4 {
            log::error!("coulnd't write device status report");
            return;
        }
        self.terminal.stdout.lock().flush().unwrap();
        let mut i = 0;
        //^[[5;1R
        while i < buffer.len() - 1 {
            self.terminal.stdin.read(&mut buffer).unwrap();
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
        self.rows = buffer[2] as usize;
        self.cols = buffer[6] as usize;
    }
    pub(crate) fn process_key_press(&mut self) -> Option<EditorCommands> {
        let key = self.terminal.read_key();
        //let exit_key = Keys::cntrl(b'q');
        match (key, self.mode) {
            (None, _) => None,
            (Some(k), EditorModes::Normal) => self.handle_normal_mode(k),
            (Some(k), EditorModes::Insert) => self.handle_insert_mode(k),
        }
    }
    pub(crate) fn handle_normal_mode(&mut self, k: u8) -> Option<EditorCommands> {
        match k {
            //TODO: These also move cursor
            x if x == Keys::cntrl(b'u') || x == Keys::cntrl(b'd') => self.navigate(k),
            b'h' | b'l' | b'j' | b'k' => self.navigate(k),
            b'a' => self.move_cursor_insert(k),
            b':' => self.parse_status_cmd_blocking(),
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'i' => self.change_mode(EditorModes::Insert),
            _ => Some(EditorCommands::Healthy),
        }
    }
    pub(crate) fn naive_move_cursor_2d(&mut self, x: usize, y: usize) {
        // Does not calculate borders
        if self
            .terminal
            .stdout
            .write(format!("\x1B[{};{}H", x, y).as_bytes())
            .unwrap() as u32
            != 5
        {
            log::error!("Couldn't go to command mode",);
        }
    }
    pub(crate) fn naive_move_cursor(&mut self, direction: CursorDirections, offset: usize) {
        // Does not calculate borders
        match direction {
            CursorDirections::Up => {
                if self
                    .terminal
                    .stdout
                    .write(format!("\x1B[{}A", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Down => {
                if self
                    .terminal
                    .stdout
                    .write(format!("\x1B[{}B", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Right => {
                if self
                    .terminal
                    .stdout
                    .write(format!("\x1B[{}C", offset).as_bytes())
                    .unwrap() as u32
                    != 3
                {
                    log::error!("Couldn't go to command mode");
                }
            }
            CursorDirections::Left => {
                if self
                    .terminal
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
    pub(crate) fn save_buffer(&mut self, file_name: &str) -> Result<(), ()> {
        let mut f_name = "";
        if file_name.is_empty() {
            f_name = &self.files;
        } else {
            f_name = file_name;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(f_name)
            .map_err(|err| {
                self.set_status_message(format!("{} Cannot save to file", err).as_str());
            })
            .unwrap();
        let mut writer = BufWriter::new(&file);
        file.set_len(self.data.buffer.len() as u64).unwrap();
        let buffer_len = writer.write(&self.data.buffer).unwrap();
        self.set_status_message(format!("{} B written", buffer_len).as_str());
        self.dirty = 0;
        Ok(())
    }
    pub(crate) fn find_in_file(&mut self, word: &str) {
        self.highlight_register = self.data.find(word);
    }
    pub(crate) fn save_file(&mut self, file_name: &str) -> Result<(), ()> {
        self.save_buffer("").unwrap();
        self.save_buffer(file_name).unwrap();
        Ok(())
    }
    pub(crate) fn exit_editor(&mut self) -> Option<EditorCommands> {
        let _ = self.terminal.stdout.write(b"\x1b[2J");
        let _ = self.terminal.stdout.write(b"\x1b[H");
        Some(EditorCommands::Exit)
    }
    pub(crate) fn run_cmd(&mut self, args: Vec<&str>) -> Option<EditorCommands> {
        //Replace this with YIM engine later
        let args_length = args.len();
        if args_length <= 0 {
            return Some(EditorCommands::Healthy);
        }
        let args_args = &args[1..];
        match args[0] {
            "w" => {
                self.save_file(&args_args.join(""));
                Some(EditorCommands::Healthy)
            }
            "s" | "search" => {
                self.find_in_file(&args_args.join(""));
                Some(EditorCommands::Healthy)
            }
            "o" => {
                self.open(&args_args.join(""));
                Some(EditorCommands::Healthy)
            }
            "q" => {
                if self.dirty != 0 {
                    self.set_status_message(
                        "You have unsaved changes, press :q! to quit without saving",
                    );
                    Some(EditorCommands::Healthy)
                } else {
                    self.exit_editor()
                }
            }
            "q!" => self.exit_editor(),
            "wq" => {
                self.save_file(&args_args.join(""));
                self.exit_editor()
            }
            _ => {
                self.set_status_message("This command does not exist!!!");
                Some(EditorCommands::Healthy)
            }
        }
    }
    pub(crate) fn parse_status_cmd_blocking(&mut self) -> Option<EditorCommands> {
        //In this mode we show user typed value.
        //self.terminal.control_echo(true);
        // TODO: Hacky render fix alter
        self.naive_move_cursor_2d(self.rows + 2, 0);
        self.clear_status_message_from_editor();
        let mut cmd = String::new();
        self.naive_move_cursor_2d(self.rows + 2, 2);
        // REFREFREFACTOR
        loop {
            let key = self.terminal.read_key();
            if key.unwrap() == b'\x7F' {
                //BACKSPACE is clicked
                // ALL this to have backspace HAHAHA
                cmd.pop();
                self.naive_move_cursor(CursorDirections::Left, 1);
                if self.terminal.stdout.write(b" ").unwrap() as u32 != 1 {
                    log::error!("Couldn't write");
                }
                self.naive_move_cursor(CursorDirections::Left, 1);
                continue;
            }
            if key.unwrap() == 13 as u8 {
                //Until ENTER is clicked
                break;
            } else {
                if self.terminal.stdout.write(&[key.unwrap()]).unwrap() as u32 != 1 {
                    log::error!("Couldn't write",);
                }
            }
            cmd.push(key.unwrap() as char);
        }
        log::debug!("{:?} {}", cmd, cmd.len());
        //self.terminal.control_echo(false);
        self.run_cmd(cmd.split(" ").collect())
    }
    pub(crate) fn handle_insert_mode(&mut self, k: u8) -> Option<EditorCommands> {
        match k {
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'\x7F' => self.remove_char(),
            _ => self.insert_char(k),
        }
    }
    pub(crate) fn move_cursor_insert(&mut self, k: u8) -> Option<EditorCommands> {
        // TODO: Make here better A lot of repetittions
        let mut row_insert_size = 0;
        if self.c_y < self.rows {
            let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            row_insert_size = self.data.buffer[index_l..index_r].len();
        }
        match k {
            b'a' => {
                if row_insert_size != 0 && self.c_x < row_insert_size - 1 {
                    self.c_x += 1
                } else if row_insert_size != 0 && self.c_x >= row_insert_size - 1 {
                    self.c_y += 1;
                    self.c_x = 0;
                }
                self.change_mode(EditorModes::Insert);
            }
            _ => unreachable!(),
        }
        if self.c_y < self.rows {
            let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            row_insert_size = self.data.buffer[index_l..index_r].len();
        }
        if row_insert_size != 0 && self.c_x > row_insert_size {
            self.c_x = row_insert_size - 1;
        }
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn move_cursor(
        &mut self,
        direction: CursorDirections,
        offset: usize,
    ) -> Option<EditorCommands> {
        // TODO: Make here better A lot of repetittions
        let mut row_insert_size = 0;
        if self.c_y < self.rows {
            let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            row_insert_size = self.data.buffer[index_l..index_r].len();
        }
        match direction {
            CursorDirections::Left => {
                if self.c_x != 0 {
                    self.c_x -= offset
                } else if self.c_y > 0 {
                    self.move_cursor(CursorDirections::Up, offset);
                    //self.c_y -= offset;
                    let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y);
                    row_insert_size = self.data.buffer[index_l..index_r].len();
                    if offset > row_insert_size {
                        self.c_x = 0
                    } else {
                        self.c_x = row_insert_size - offset + 1
                    }
                }
            }
            CursorDirections::Down => {
                if (self.data.new_lines.len() as i32) - (offset as i32) > self.c_y as i32 {
                    self.c_y += offset
                } else {
                    self.c_y = self.data.new_lines.len() - 1
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
                    && self.c_y != self.data.new_lines.len() - offset
                {
                    self.c_y += offset;
                    self.c_x = 0;
                }
            }
        }
        if self.c_y < self.rows {
            let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            row_insert_size = self.data.buffer[index_l..index_r].len();
        }
        if row_insert_size != 0 && self.c_x > row_insert_size {
            self.c_x = row_insert_size - 1;
        }
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn navigate(&mut self, k: u8) -> Option<EditorCommands> {
        // TODO: Make here better A lot of repetittions
        match k {
            b'h' => self.move_cursor(CursorDirections::Left, 1),
            b'j' => self.move_cursor(CursorDirections::Down, 1),
            b'k' => self.move_cursor(CursorDirections::Up, 1),
            b'l' => self.move_cursor(CursorDirections::Right, 1),
            x if x == Keys::cntrl(b'd') => self.move_cursor(CursorDirections::Down, 20),
            x if x == Keys::cntrl(b'u') => self.move_cursor(CursorDirections::Up, 20),
            _ => unreachable!(),
        }
    }
    pub(crate) fn remove_char(&mut self) -> Option<EditorCommands> {
        log::debug!("{} {} {}", self.c_x, self.c_y, self.row_offset);
        if self.c_x == 0 && self.c_y == 0 {
            return Some(EditorCommands::Healthy); //Early return
        }
        //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
        //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
        let ind = self.calculate_file_index(self.c_x as usize, self.c_y as usize) - 1;
        self.data.remove(ind);
        log::debug!("{} {} {}", self.c_x, self.c_y, self.row_offset);
        log::debug!("{:?}", self.data.buffer);
        self.move_cursor(CursorDirections::Left, 1);
        log::debug!("{} {} {}", self.c_x, self.c_y, self.row_offset);
        log::debug!("{:?}", self.data.buffer);
        self.data.update_buffers();
        log::debug!("{} {} {}", self.c_x, self.c_y, self.row_offset);
        log::debug!("{:?}", self.data.buffer);
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn insert_char(&mut self, ch: u8) -> Option<EditorCommands> {
        log::debug!("Handling new {}", ch);
        if ch == 13 as u8 || self.c_y == self.cols {
            let ind = self.calculate_file_index(self.c_x as usize, self.c_y as usize);
            self.data.insert(ind, b'\n');
            self.c_x = 0;
            self.c_y += 1;
        } else {
            let ind = self.calculate_file_index(self.c_x as usize, self.c_y as usize);
            log::debug!("il {}", self.c_x);
            self.data.insert(ind, ch);
            self.c_x += 1;
        }
        self.dirty = 1;
        Some(EditorCommands::Healthy)
    }
}

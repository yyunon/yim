use chrono::offset::Utc;
use chrono::DateTime;
use std::io::{BufReader, BufWriter, Read, Stdin, Stdout, Write};
use std::time::SystemTime;

mod buffer;
mod constants;
mod cursor;
mod terminal;

pub use crate::editor::constants::*;

pub use crate::editor::buffer::AppendBuffer;
pub use crate::editor::cursor::Cursor;
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

pub struct Editor {
    pub mode: EditorModes,
    pub cursor: Cursor,
    state: EditorCommands,
    terminal: Terminal,
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
            cursor: Cursor::new(),
            state: EditorCommands::Healthy,
            terminal: Terminal::new(stdin, stdout),
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
        self.cursor.clear();
        self.set_window_size();
        self.cursor.rows -= 2;
    }
    pub(crate) fn set_window_size(&mut self) {
        let (cols, rows) = self.terminal.term_size();
        if cols == 0 {
            self.calculate_window()
        } else {
            self.cursor.rows(rows);
            self.cursor.cols(cols);
        }
    }
    pub(crate) fn calculate_window(&mut self) {
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
        self.cursor.rows = buffer[2] as usize;
        self.cursor.cols = buffer[6] as usize;
    }
    pub(crate) fn launch_engine(&mut self) {
        loop {
            log::debug!("Mode {:?}", self.mode);
            self.render();
            let option = self.process_key_press().unwrap();
            if option == EditorCommands::Exit {
                break;
            }
        }
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
    // Gets the display index row axis index and return row printable c_x, c_y
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
        for _y in 0..self.cursor.rows {
            let file_row = _y + self.cursor.row_offset;
            if file_row >= self.data.new_lines.len() {
                self.append_buffer.append(b"~");
            } else {
                // TODO Ref here HANDLE COL limits
                let (index_l, index_r) = self
                    .cursor
                    .calculate_row_of_insert_indices(file_row as usize, &self.data.new_lines);
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
        let mut rstatus: String = format!(" [{}/{}] ", self.cursor.c_y + 1, self.cursor.rows);
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
        if status.len() > self.cursor.cols {
            self.append_buffer
                .append_str(&status[0..self.cursor.cols as usize]);
        } else {
            self.append_buffer.append_str(&status);
        }
        let mut len = 0;
        while len < self.cursor.cols {
            if self.cursor.cols - len - status.len() == rstatus.len() {
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
        self.cursor.calculate_row_offset();
        self.append_buffer.append(b"\x1B[?25l");
        self.append_buffer.append(b"\x1B[H");
        self.draw();
        self.draw_status_bar();
        self.draw_message_bar();
        self.append_buffer.append_str(
            format!(
                "\x1B[{};{}H",
                (self.cursor.c_y - self.cursor.row_offset) + 1,
                self.cursor.c_x + 1
            )
            .as_str(),
        );
        self.append_buffer.append(b"\x1B[?25h");
        self.append_buffer.write(&mut self.terminal.stdout);
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
            b'a' | b'I' | b'A' => self.move_cursor_insert(k),
            b':' => self.parse_status_cmd_blocking(),
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'i' => self.change_mode(EditorModes::Insert),
            _ => Some(EditorCommands::Healthy),
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
        self.cursor
            .naive_move_cursor_2d(&mut self.terminal, self.cursor.rows + 2, 0);
        self.clear_status_message_from_editor();
        let mut cmd = String::new();
        self.cursor
            .naive_move_cursor_2d(&mut self.terminal, self.cursor.rows + 2, 2);
        // REFREFREFACTOR
        loop {
            let key = self.terminal.read_key();
            if key.unwrap() == b'\x7F' {
                //BACKSPACE is clicked
                // ALL this to have backspace HAHAHA
                cmd.pop();
                self.cursor
                    .naive_move_cursor(&mut self.terminal, CursorDirections::Left, 1);
                if self.terminal.stdout.write(b" ").unwrap() as u32 != 1 {
                    log::error!("Couldn't write");
                }
                self.cursor
                    .naive_move_cursor(&mut self.terminal, CursorDirections::Left, 1);
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
        match k {
            b'I' => {
                self.cursor
                    .move_cursor(&self.data.new_lines, CursorDirections::LineBegin, 1);
                self.change_mode(EditorModes::Insert);
            }
            b'A' => {
                self.cursor
                    .move_cursor(&self.data.new_lines, CursorDirections::LineEnd, 1);
                self.change_mode(EditorModes::Insert);
            }
            b'a' => {
                self.cursor
                    .move_cursor(&self.data.new_lines, CursorDirections::Right, 1);
                self.change_mode(EditorModes::Insert);
            }
            _ => unreachable!(),
        }
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn navigate(&mut self, k: u8) -> Option<EditorCommands> {
        // TODO: Make here better A lot of repetittions
        match k {
            b'h' => self
                .cursor
                .move_cursor(&self.data.new_lines, CursorDirections::Left, 1),
            b'j' => self
                .cursor
                .move_cursor(&self.data.new_lines, CursorDirections::Down, 1),
            b'k' => self
                .cursor
                .move_cursor(&self.data.new_lines, CursorDirections::Up, 1),
            b'l' => self
                .cursor
                .move_cursor(&self.data.new_lines, CursorDirections::Right, 1),
            x if x == Keys::cntrl(b'd') => {
                self.cursor
                    .move_cursor(&self.data.new_lines, CursorDirections::Down, 20)
            }
            x if x == Keys::cntrl(b'u') => {
                self.cursor
                    .move_cursor(&self.data.new_lines, CursorDirections::Up, 20)
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn remove_char(&mut self) -> Option<EditorCommands> {
        if self.cursor.c_x == 0 && self.cursor.c_y == 0 {
            return Some(EditorCommands::Healthy); //Early return
        }
        //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
        //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
        let ind = self.cursor.calculate_file_index(
            &self.data.new_lines,
            self.cursor.c_x as usize,
            self.cursor.c_y as usize,
        ) - 1;
        self.data.remove(ind);
        self.cursor
            .move_cursor(&self.data.new_lines, CursorDirections::Left, 1);
        self.data.update_buffers();
        Some(EditorCommands::Healthy)
    }
    pub(crate) fn insert_char(&mut self, ch: u8) -> Option<EditorCommands> {
        log::debug!("Handling new {}", ch);
        if ch == 13 as u8 || self.cursor.c_y == self.cursor.cols {
            let ind = self.cursor.calculate_file_index(
                &self.data.new_lines,
                self.cursor.c_x as usize,
                self.cursor.c_y as usize,
            );
            self.data.insert(ind, b'\n');
            self.cursor.c_x = 0;
            self.cursor.c_y += 1;
        } else {
            let ind = self.cursor.calculate_file_index(
                &self.data.new_lines,
                self.cursor.c_x as usize,
                self.cursor.c_y as usize,
            );
            self.data.insert(ind, ch);
            self.cursor.c_x += 1;
        }
        self.dirty = 1;
        Some(EditorCommands::Healthy)
    }
}

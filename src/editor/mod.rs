use chrono::offset::Utc;
use chrono::DateTime;
use std::cell::RefCell;
use std::io::{BufReader, BufWriter, Read, Stdin, Stdout, Write};
use std::rc::Rc;
use std::time::SystemTime;

mod buffer;
mod constants;
mod cursor;
mod engine;
mod graphics;
mod terminal;
mod window;

pub use crate::editor::constants::*;

pub use crate::editor::buffer::AppendBuffer;
pub use crate::editor::cursor::Cursor;
pub use crate::editor::engine::*;
pub use crate::editor::terminal::Terminal;
pub use crate::editor::window::Window;

extern crate libc;

struct Keys;
impl Keys {
    fn is_cntrl(c: usize) -> bool {
        c == 127 || (c >= 0 && c <= 31)
    }
    fn is_number(c: usize) -> bool {
        c >= 48 && c <= 57
    }
    fn cntrl(c: u8) -> u8 {
        c & 0x1f
    }
}

#[derive(Default, Debug, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub struct EditorConfigs {
    pub x_offset: usize,
    pub y_offset: usize,
}

pub struct EditorContext {
    pub mode: EditorModes,
    pub state: EditorHealth,
    pub status_message: String,
    pub status_message_time: SystemTime,
    pub highlight_register: Vec<(usize, usize)>,
    pub h_reg: usize,
    pub line_reg: usize,
    pub dirty: i8,
    pub files: String,
}

pub struct Editor {
    pub cursor: Rc<RefCell<Cursor>>,
    pub window: Option<Window>,
    pub terminal: Rc<RefCell<Terminal>>,
    pub context: Rc<RefCell<EditorContext>>,
    pub editor_configs: EditorConfigs,
    append_buffer: AppendBuffer,
    data: AppendBuffer,
}
impl Editor {
    pub(crate) fn new(stdin: Stdin, stdout: Stdout) -> Self {
        let ec = EditorContext {
            mode: EditorModes::Normal,
            state: EditorHealth::Healthy,
            status_message: "".to_string(),
            status_message_time: SystemTime::now(),
            dirty: 0,
            highlight_register: Vec::default(),
            h_reg: 0,
            line_reg: 0,
            files: "".to_string(),
        };
        Self {
            cursor: Cursor::new(),
            window: None,
            terminal: Terminal::new(stdin, stdout),
            context: Rc::new(RefCell::new(ec)),
            editor_configs: EditorConfigs::default(),
            append_buffer: AppendBuffer::default(),
            data: AppendBuffer::default(),
        }
    }
    pub(crate) fn init_editor(&mut self) {
        self.terminal.borrow_mut().enable_raw_mode();
        self.cursor.borrow_mut().clear();
        self.window = Some(Window::new(&self.cursor, &self.terminal));
        self.window.as_mut().map(|w| w.set_window_size());
        self.cursor.borrow_mut().rows -= 2;
        self.cursor.borrow_mut().editor_configs = self.editor_configs.clone();
    }
    pub(crate) fn launch_engine(&mut self) {
        loop {
            log::debug!("Mode {:?}", self.context.borrow_mut().mode);
            graphics::render(
                &self.context,
                &self.terminal,
                &mut self.cursor.borrow_mut(),
                &self.data,
                &mut self.append_buffer,
            );
            let option = self.process_key_press().unwrap();
            if option == EditorHealth::Exit {
                break;
            }
        }
    }
    pub(crate) fn open(&mut self, input_file: &str) -> std::io::Result<()> {
        self.context.borrow_mut().files = input_file.to_string();
        log::debug!("{:?}", self.context.borrow().files);
        log::debug!("{:?}", input_file);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(input_file)?;
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut self.data.buffer)?;
        self.data.update_buffers();
        log::debug!("{:?}", self.data);
        Ok(())
    }
    pub(crate) fn change_mode(&mut self, m: EditorModes) -> Option<EditorHealth> {
        self.context.borrow_mut().mode = m;
        Some(EditorHealth::Healthy)
    }
    // Gets the display index row axis index and return row printable c_x, c_y
    pub(crate) fn set_status_message(&mut self, ins: &str) {
        self.context.borrow_mut().status_message = ins.to_string();
    }
    pub(crate) fn clear_status_message_from_editor(&mut self) {
        let status_len: usize = self.context.borrow().status_message.capacity();
        let mut cmd_buffer = String::new();
        cmd_buffer.push(b':' as char);
        for _i in 0..status_len {
            cmd_buffer.push(' ');
        }
        self.terminal
            .borrow_mut()
            .stdout
            .write(cmd_buffer.as_bytes())
            .unwrap();
    }
    pub(crate) fn file_index_to_cursor(&mut self) -> (usize, usize) {
        let mut i_x = 0;
        let mut i_y: i32 = -1;
        let value = self.context.borrow().highlight_register
            [self.context.borrow().h_reg % (self.context.borrow().highlight_register.len() + 2)]
            .0;
        for (i, d) in self.data.new_lines.iter().enumerate() {
            if value < *d as usize {
                i_y = i as i32;
                i_x = *d; // A value before
                break;
            }
        }
        if i_y < 0 {
            //Means it is the last file_index
            i_y = (self.data.new_lines.len() - 1) as i32;
            i_x = self.data.new_lines[self.data.new_lines.len() - 1];
        }
        log::debug!("{:?}", self.data.new_lines);
        log::debug!("{}, {}, {}", value, i_y, i_x);
        ((i_x - value as i32) as usize, i_y as usize)
    }
    pub(crate) fn process_key_press(&mut self) -> Option<EditorHealth> {
        let key = self.terminal.borrow_mut().read_key();
        let mode = self.context.borrow().mode;
        //let exit_key = Keys::cntrl(b'q');
        match (key, mode) {
            (None, _) => None,
            (Some(k), EditorModes::Normal) => self.handle_normal_mode(k),
            (Some(k), EditorModes::Insert) => self.handle_insert_mode(k),
        }
    }
    pub(crate) fn update_h_reg(&mut self, k: u8) -> Option<EditorHealth> {
        if self.context.borrow().highlight_register.len() == 0 {
            return Some(EditorHealth::Healthy);
        }
        log::debug!("Update n");
        let mut tmp = 0;
        match k {
            b'n' => tmp = self.context.borrow().h_reg + 1,
            b'N' => tmp = self.context.borrow().h_reg - 1,
            _ => !unreachable!(),
        }
        //self.h_reg = self.highlight_register[tmp % (self.highlight_register.len() + 2)].0;
        let mut c = self.context.borrow_mut();
        c.h_reg = tmp % c.highlight_register.len();
        Some(EditorHealth::Healthy)
    }
    pub(crate) fn update_line_reg(&mut self, k: u8) -> Option<EditorHealth> {
        let mut x = [0u8; 4];
        let mut num: Vec<u32> = Vec::new();
        x[3] = k;
        num.push(u32::from_be_bytes(x) - 48);
        //log::debug!("{:?}:{:?}", x, u32::from_be_bytes(x));
        self.context.borrow_mut().line_reg = 0;
        loop {
            let key = self.terminal.borrow_mut().read_key();
            if key.unwrap() == 13 as u8 {
                //Until ENTER is clicked
                break;
            }
            x = [0u8; 4];
            x[3] = key.unwrap();
            //log::debug!("{:?}:{:?}", x, u32::from_be_bytes(x));
            num.push(u32::from_be_bytes(x) - 48);
        }
        let accumulator = num
            .into_iter()
            .rev()
            .enumerate()
            .fold(0, |s, (i, d)| s + (i as u32) * 10 + d);

        self.context.borrow_mut().line_reg = accumulator as usize - 1;

        //log::debug!("{:?}", accumulator - 1);

        Some(EditorHealth::Healthy)
    }
    pub(crate) fn handle_normal_mode(&mut self, k: u8) -> Option<EditorHealth> {
        match k {
            //TODO: These also move cursor
            x if x == Keys::cntrl(b'u') || x == Keys::cntrl(b'd') => self.navigate(k),
            x if Keys::is_number(x.into()) => self.update_line_reg(k),
            b'h' | b'l' | b'j' | b'k' => self.navigate(k),
            b'n' | b'N' => self.update_h_reg(k),
            b'd' => operations::normal::delete_operations(
                &self.context,
                &self.cursor,
                &self.terminal,
                &mut self.data,
                k,
            ),
            //b'g' => self.update_h_reg(k),
            b'a' | b'I' | b'A' => self.move_cursor_insert(k),
            b':' => self.parse_status_cmd_blocking(),
            //b'n' => self.go_to_reg(),
            b'/' => operations::normal::find_in_file_blocking(
                &self.context,
                &self.cursor,
                &self.terminal,
                &self.data,
            ),
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'i' => self.change_mode(EditorModes::Insert),
            _ => Some(EditorHealth::Healthy),
        }
    }
    pub(crate) fn save_buffer(&mut self, file_name: &str) -> Result<(), ()> {
        let mut f_name = self.context.borrow().files.clone();
        if !file_name.is_empty() {
            f_name = file_name.to_string();
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
        self.context.borrow_mut().dirty = 0;
        Ok(())
    }
    pub(crate) fn find_in_file(&mut self, word: &str) {
        self.context.borrow_mut().highlight_register = self.data.find(word);
    }
    pub(crate) fn save_file(&mut self, file_name: &str) -> Result<(), ()> {
        self.save_buffer("").unwrap();
        self.save_buffer(file_name).unwrap();
        Ok(())
    }
    pub(crate) fn exit_editor(&mut self) -> Option<EditorHealth> {
        let _ = self.terminal.borrow_mut().stdout.write(b"\x1b[2J");
        let _ = self.terminal.borrow_mut().stdout.write(b"\x1b[H");
        Some(EditorHealth::Exit)
    }
    pub(crate) fn run_cmd(&mut self, args: Vec<&str>) -> Option<EditorHealth> {
        //Replace this with YIM engine later
        let args_length = args.len();
        if args_length <= 0 {
            return Some(EditorHealth::Healthy);
        }
        let args_args = &args[1..];
        match args[0] {
            "w" => {
                self.save_file(&args_args.join(""));
                Some(EditorHealth::Healthy)
            }
            "s" | "search" => {
                self.find_in_file(&args_args.join(""));
                Some(EditorHealth::Healthy)
            }
            "o" => {
                self.open(&args_args.join(""));
                Some(EditorHealth::Healthy)
            }
            "q" => {
                if self.context.borrow_mut().dirty != 0 {
                    self.set_status_message(
                        "You have unsaved changes, press :q! to quit without saving",
                    );
                    Some(EditorHealth::Healthy)
                } else {
                    self.exit_editor()
                }
            }
            "noh" => {
                //self.clear_highlight_register();
                self.context.borrow_mut().highlight_register.clear();
                Some(EditorHealth::Healthy)
            }
            "q!" => self.exit_editor(),
            "wq" => {
                self.save_file(&args_args.join(""));
                self.exit_editor()
            }
            _ => {
                self.set_status_message("This command does not exist!!!");
                Some(EditorHealth::Healthy)
            }
        }
    }
    pub(crate) fn parse_status_cmd_blocking(&mut self) -> Option<EditorHealth> {
        //In this mode we show user typed value.
        //self.terminal.borrow_mut().control_echo(true);
        // TODO: Hacky render fix alter
        self.cursor
            .borrow()
            .naive_move_cursor_2d(&self.terminal, self.cursor.borrow().rows + 2, 0);
        self.clear_status_message_from_editor();
        let mut cmd = String::new();
        self.cursor
            .borrow()
            .naive_move_cursor_2d(&self.terminal, self.cursor.borrow().rows + 2, 2);
        // REFREFREFACTOR
        loop {
            let key = self.terminal.borrow_mut().read_key().unwrap();
            if key == b'\x7F' {
                //BACKSPACE is clicked
                // ALL this to have backspace HAHAHA
                cmd.pop();
                self.cursor
                    .borrow()
                    .naive_move_cursor(&self.terminal, CursorDirections::Left, 1);
                if self.terminal.borrow_mut().stdout.write(b" ").unwrap() as u32 != 1 {
                    log::error!("Couldn't write");
                }
                self.cursor
                    .borrow()
                    .naive_move_cursor(&self.terminal, CursorDirections::Left, 1);
                continue;
            } else if key == 13 as u8 {
                //Until ENTER is clicked
                break;
            } else if key == 27 as u8 {
                break;
            } else {
                if self.terminal.borrow_mut().stdout.write(&[key]).unwrap() as u32 != 1 {
                    log::error!("Couldn't write",);
                }
            }
            cmd.push(key as char);
        }
        log::debug!("{:?} {}", cmd, cmd.len());
        //self.terminal.borrow_mut().control_echo(false);
        self.run_cmd(cmd.split(" ").collect())
    }
    pub(crate) fn handle_insert_mode(&mut self, k: u8) -> Option<EditorHealth> {
        match k {
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'\x7F' => {
                self.context.borrow_mut().dirty = 1;
                operations::insert::remove_char(&self.cursor, &mut self.data)
            }
            _ => {
                self.context.borrow_mut().dirty = 1;
                operations::insert::insert_char(&self.cursor, &mut self.data, k)
            }
        }
    }
    pub(crate) fn move_cursor_insert(&mut self, k: u8) -> Option<EditorHealth> {
        match k {
            b'I' => {
                self.cursor
                    .borrow_mut()
                    .move_cursor(&self.data.new_lines, CursorDirections::LineBegin, 1)
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            b'A' => {
                self.cursor
                    .borrow_mut()
                    .move_cursor(&self.data.new_lines, CursorDirections::LineEnd, 1)
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            b'a' => {
                self.cursor
                    .borrow_mut()
                    .move_cursor(&self.data.new_lines, CursorDirections::Right, 1)
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            _ => unreachable!(),
        }
        Some(EditorHealth::Healthy)
    }
    pub(crate) fn navigate(&mut self, k: u8) -> Option<EditorHealth> {
        // TODO: Make here better A lot of repetittions
        log::debug!("{:?}", self.cursor.borrow());
        match k {
            b'h' => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Left, 1)
                .unwrap(),
            b'j' => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Down, 1)
                .unwrap(),
            b'k' => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Up, 1)
                .unwrap(),
            b'l' => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Right, 1)
                .unwrap(),
            x if x == Keys::cntrl(b'd') => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Down, 20)
                .unwrap(),
            x if x == Keys::cntrl(b'u') => self
                .cursor
                .borrow_mut()
                .move_cursor(&self.data.new_lines, CursorDirections::Up, 20)
                .unwrap(),
            _ => unreachable!(),
        }
        Some(EditorHealth::Healthy)
    }
}

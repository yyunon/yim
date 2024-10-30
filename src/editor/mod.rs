use chrono::offset::Utc;
use std::io::{Stdin, Stdout};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

mod buffer;
mod constants;
mod cursor;
mod engine;
mod file;
mod graphics;
mod terminal;
mod window;

pub use crate::editor::constants::*;

pub use crate::editor::buffer::AppendBuffer;
pub use crate::editor::cursor::Cursor;
pub use crate::editor::engine::*;
pub use crate::editor::file::File;
pub use crate::editor::terminal::Terminal;
pub use crate::editor::window::Window;

use self::operations::normal::Renderer;

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
    pub files: File,
}

pub struct EditorControllers {
    pub cursor: Cursor,
    pub window: Option<Window>,
    pub terminal: Terminal,
    pub data: AppendBuffer,
}

pub struct Editor {
    pub editor_controller: Option<EditorControllers>,
    pub editor_context: Option<EditorContext>,
    pub renderer: Option<Renderer>,
    pub editor_configs: EditorConfigs,
}
impl Editor {
    pub(crate) fn new(stdin: Stdin, stdout: Stdout, editor_configs: Option<EditorConfigs>) -> Self {
        let ec = EditorContext {
            mode: EditorModes::Normal,
            state: EditorHealth::Healthy,
            status_message: "".to_string(),
            status_message_time: SystemTime::now(),
            dirty: 0,
            highlight_register: Vec::default(),
            h_reg: 0,
            line_reg: usize::MAX,
            files: File {
                file_name: "".to_string(),
            },
        };
        Self {
            editor_controller: Some(EditorControllers {
                cursor: Cursor::new(),
                terminal: Terminal::new(stdin, stdout),
                window: None,
                data: AppendBuffer::default(),
            }),
            editor_context: Some(ec),
            editor_configs: editor_configs.unwrap_or_else(|| EditorConfigs::default()),
            renderer: None,
        }
    }
    pub(crate) fn init_editor(&mut self) {
        let editor_controller = self.editor_controller.as_mut().unwrap();
        editor_controller.terminal.enable_raw_mode();
        editor_controller.cursor.clear();
        editor_controller.window = Some(Window::new());
        editor_controller
            .window
            .as_mut()
            .map(|w| w.set_window_size(&mut editor_controller.cursor, &mut editor_controller.terminal));
        editor_controller.cursor.rows -= 2;
        editor_controller.cursor.editor_configs = self.editor_configs.clone();

        log::debug!("{:?}", editor_controller.cursor);
        log::debug!("{:?}", editor_controller.window);

        self.renderer = Some(Renderer::new());
    }
    pub(crate) fn launch_editor(&mut self) {
        loop {
            self.render();
            let option = self.process_key_press();
            if let Some(EditorHealth::Exit) = option {
                break;
            }
        }
    }

    pub(crate) fn render(&mut self) {
        let ed_controller = self.editor_controller.as_mut().unwrap();
        let ed_context = self.editor_context.as_ref().unwrap();
        let rend = self.renderer.as_mut().unwrap();
        rend
            .render(ed_controller, 
                    ed_context);

        log::debug!("Rendering thread is spawned");
    }

    pub(crate) fn open(&mut self, inputf: &str) -> std::io::Result<()> {
        self.editor_context
            .as_mut()
            .unwrap()
            .files
            .open(inputf, &mut self.editor_controller.as_mut().unwrap().data)
    }
    pub(crate) fn change_mode(&mut self, m: EditorModes) -> Option<EditorHealth> {
        self.editor_context.as_mut().unwrap().mode = m;
        Some(EditorHealth::Healthy)
    }
    // Gets the display index row axis index and return row printable c_x, c_y
    pub(crate) fn set_status_message(&mut self, ins: &str) {
        self.editor_context.as_mut().unwrap().status_message = ins.to_string();
    }
    pub(crate) fn clear_status_message_from_editor(&mut self) {
        let status_len: usize = self
            .editor_context
            .as_ref()
            .unwrap()
            .status_message
            .capacity();
        let mut cmd_buffer = String::new();
        cmd_buffer.push(b':' as char);
        for _i in 0..status_len {
            cmd_buffer.push(' ');
        }
        self.editor_controller
            .as_mut()
            .unwrap()
            .terminal
            .write(cmd_buffer.as_bytes());
    }
    pub(crate) fn file_index_to_cursor(&mut self) -> (usize, usize) {
        let mut i_x = 0;
        let mut i_y: i32 = -1;
        let h_reg = self.editor_context.as_ref().unwrap().h_reg;
        let highlight_register = self.editor_context.as_ref().unwrap().highlight_register.clone();
        let value = self
            .editor_context
            .as_mut()
            .unwrap()
            .highlight_register[h_reg
            % (highlight_register
                .len()
                + 2)]
            .0;
        for (i, d) in self
            .editor_controller
            .as_ref()
            .unwrap()
            .data
            .new_lines
            .iter()
            .enumerate()
        {
            if value < *d as usize {
                i_y = i as i32;
                i_x = *d; // A value before
                break;
            }
        }
        if i_y < 0 {
            //Means it is the last file_index
            i_y = (self.editor_controller.as_ref().unwrap().data.new_lines.len() - 1) as i32;
            i_x = self.editor_controller.as_ref().unwrap().data.new_lines
                [self.editor_controller.as_ref().unwrap().data.new_lines.len() - 1];
        }
        log::debug!("{:?}", self.editor_controller.as_ref().unwrap().data.new_lines);
        log::debug!("{}, {}, {}", value, i_y, i_x);
        ((i_x - value as i32) as usize, i_y as usize)
    }
    pub(crate) fn process_key_press(&mut self) -> Option<EditorHealth> {
        let key = self.editor_controller.as_mut().unwrap().terminal.read_key();
        let context = self.editor_context.as_ref().unwrap();
        log::debug!("{:?}", key);
        log::debug!("{:?}", context.mode);
        //let exit_key = Keys::cntrl(b'q');
        match (key, context.mode) {
            (None, _) => None,
            (Some(k), EditorModes::Normal) => self.handle_normal_mode(k),
            (Some(k), EditorModes::Insert) => self.handle_insert_mode(k),
        }
    }
    pub(crate) fn update_h_reg(&mut self, k: u8) -> Option<EditorHealth> {
        if self
            .editor_context
            .as_ref()
            .unwrap()
            .highlight_register
            .len()
            == 0
        {
            return Some(EditorHealth::Healthy);
        }
        log::debug!("Update n");
        let mut tmp = 0;
        match k {
            b'n' => tmp = self.editor_context.as_ref().unwrap().h_reg + 1,
            b'N' => tmp = self.editor_context.as_ref().unwrap().h_reg - 1,
            _ => !unreachable!(),
        }
        //self.h_reg = self.highlight_register[tmp % (self.highlight_register.len() + 2)].0;
        let mut c = self.editor_context.as_mut().unwrap();
        c.h_reg = tmp % c.highlight_register.len();
        Some(EditorHealth::Healthy)
    }
    pub(crate) fn update_line_reg(&mut self, k: u8) -> Option<EditorHealth> {
        let mut x = [0u8; 4];
        let mut num: Vec<u32> = Vec::new();
        x[3] = k;
        num.push(u32::from_be_bytes(x) - 48);
        //log::debug!("{:?}:{:?}", x, u32::from_be_bytes(x));
        loop {
            let key = self.editor_controller.as_mut().unwrap().terminal.read_key();
            if key.unwrap() == 13 as u8 {
                //Until ENTER is clicked
                break;
            }
            if Keys::is_number(key.unwrap().into()) {
                x = [0u8; 4];
                x[3] = key.unwrap();
                //log::debug!("{:?}:{:?}", x, u32::from_be_bytes(x));
                num.push(u32::from_be_bytes(x) - 48);
            }
            if key.unwrap() == b'G' as u8 {
                break;
            }
        }
        let accumulator = num
            .into_iter()
            .rev()
            .enumerate()
            .fold(0, |s, (i, d)| s + (i as u32) * 10 + d);

        self.editor_context.as_mut().unwrap().line_reg = accumulator as usize - 1;

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
                self.editor_context.as_mut().unwrap(),
                self.editor_controller.as_mut().unwrap(),
                k,
            ),
            //b'g' => self.update_h_reg(k),
            b'a' | b'I' | b'A' => self.move_cursor_insert(k),
            b':' => self.parse_status_cmd_blocking(),
            //b'n' => self.go_to_reg(),
            //b'/' => operations::normal::find_in_file_blocking(
            //    &self.renderer.as_ref().unwrap(),
            //    &self.context,
            //    &self.cursor,
            //    &self.terminal,
            //    &self.data.
            //),
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'i' => self.change_mode(EditorModes::Insert),
            _ => Some(EditorHealth::Healthy),
        }
    }
    pub(crate) fn find_in_file(&mut self, word: &str) {
        self.editor_context
            .as_mut()
            .unwrap()
            .highlight_register = self.editor_controller.as_mut().unwrap().data.find(word);
    }
    pub(crate) fn save_file(&mut self, file_name: &str) -> Result<(), ()> {
        let status = self
            .editor_context
            .as_mut()
            .unwrap()
            .files
            .save_buffer(
                vec!["", file_name],
                &mut self.editor_controller.as_mut().unwrap().data,
            )
            .unwrap();
        self.set_status_message(&status);
        self.editor_context.as_mut().unwrap().dirty = 0;
        Ok(())
    }
    pub(crate) fn exit_editor(&mut self) -> Option<EditorHealth> {
        let _ = self
            .editor_controller
            .as_mut()
            .unwrap()
            .terminal
            .write(b"\x1b[2J");
        let _ = self.editor_controller.as_mut().unwrap().terminal.write(b"\x1b[H");
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
                self.editor_context.as_mut().unwrap().files.open(
                    &args_args.join(""),
                    &mut self.editor_controller.as_mut().unwrap().data,
                );
                Some(EditorHealth::Healthy)
            }
            "q" => {
                if self.editor_context.as_ref().unwrap().dirty != 0 {
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
                self.editor_context
                    .as_mut()
                    .unwrap()
                    .highlight_register
                    .clear();
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
        //self.terminal.control_echo(true);
        // TODO: Hacky render fix alter
        let editor_controller = self.editor_controller.as_mut().unwrap();
        let cursor_c = editor_controller.cursor;
        cursor::naive_move_cursor_2d(
                &mut editor_controller.terminal,
                cursor_c.rows + 2,
                0,
            );
        
        // Clear status msg from editor
        let status_len: usize = self
            .editor_context
            .as_ref()
            .unwrap()
            .status_message
            .capacity();
        let mut cmd_buffer = String::new();
        cmd_buffer.push(b':' as char);
        for _i in 0..status_len {
            cmd_buffer.push(' ');
        }
        editor_controller
            .terminal
            .write(cmd_buffer.as_bytes());
        let mut cmd = String::new();
        cursor::naive_move_cursor_2d(
                &mut editor_controller.terminal,
                cursor_c.rows + 2,
                2,
            );

        // REFREFREFACTOR
        loop {
            let key = 
                editor_controller
                .terminal
                .read_key()
                .unwrap();
            if key == b'\x7F' {
                //BACKSPACE is clicked
                // ALL this to have backspace HAHAHA
                cmd.pop();
                cursor::naive_move_cursor(
                    &mut editor_controller.terminal,
                    CursorDirections::Left,
                    1,
                );
                editor_controller.terminal.write(b" ");
                cursor::naive_move_cursor(
                    &mut  editor_controller.terminal,
                    CursorDirections::Left,
                    1,
                );
                continue;
            } else if key == 13 as u8 {
                //Until ENTER is clicked
                break;
            } else if key == 27 as u8 {
                break;
            } else {
                editor_controller.terminal.write(&[key]);
            }
            cmd.push(key as char);
        }
        log::debug!("{:?} {}", cmd, cmd.len());
        //self.terminal.control_echo(false);
        self.run_cmd(cmd.split(" ").collect())
    }
    pub(crate) fn handle_insert_mode(&mut self, k: u8) -> Option<EditorHealth> {
        let ed_c  = self.editor_controller.as_mut().unwrap();
        match k {
            b'\x1B' => self.change_mode(EditorModes::Normal),
            b'\x7F' => {
                self.editor_context.as_mut().unwrap().dirty = 1;
                operations::insert::remove_char(
                    &mut ed_c.cursor,
                    &mut ed_c.data,
                )
            }
            _ => {
                self.editor_context.as_mut().unwrap().dirty = 1;
                operations::insert::insert_char(
                    &mut ed_c.cursor,
                    &mut ed_c.data,
                    k,
                )
            }
        }
    }
    pub(crate) fn move_cursor_insert(&mut self, k: u8) -> Option<EditorHealth> {
        let editor_controller = self.editor_controller.as_mut().unwrap();
        let data_new_lines = &editor_controller.data.new_lines;
        match k {
            b'I' => {
                editor_controller
                    .cursor
                    .move_cursor(
                        &data_new_lines,
                        CursorDirections::LineBegin,
                        1,
                    )
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            b'A' => {
                editor_controller
                    .cursor
                    .move_cursor(
                        &data_new_lines,
                        CursorDirections::LineEnd,
                        1,
                    )
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            b'a' => {
                editor_controller
                    .cursor
                    .move_cursor(
                        &data_new_lines,
                        CursorDirections::Right,
                        1,
                    )
                    .unwrap();
                self.change_mode(EditorModes::Insert);
            }
            _ => unreachable!(),
        }
        Some(EditorHealth::Healthy)
    }
    pub(crate) fn navigate(&mut self, k: u8) -> Option<EditorHealth> {
        let editor_controller = self.editor_controller.as_mut().unwrap();
        // TODO: Make here better A lot of repetittions
        log::debug!("{:?}", editor_controller.cursor);
        let data_new_lines = &editor_controller.data.new_lines;
        match k {
            b'h' => editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Left,
                    1,
                )
                .unwrap(),
            b'j' => 
                editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Down,
                    1,
                )
                .unwrap(),
            b'k' => 
                editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Up,
                    1,
                )
                .unwrap(),
            b'l' => 
                editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Right,
                    1,
                )
                .unwrap(),
            x if x == Keys::cntrl(b'd') => 
                editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Down,
                    20,
                )
                .unwrap(),
            x if x == Keys::cntrl(b'u') => 
                editor_controller
                .cursor
                .move_cursor(
                    &data_new_lines,
                    CursorDirections::Up,
                    20,
                )
                .unwrap(),
            _ => unreachable!(),
        }
        Some(EditorHealth::Healthy)
    }
}

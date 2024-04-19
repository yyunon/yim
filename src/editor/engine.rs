use std::sync::{Arc, Mutex};

pub trait IOperator {
    type OpType;
    fn new(signature: String, function: Self::OpType) -> Arc<Mutex<Self>>;
    fn run(&self) -> Result<(), ()>;
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub signature: String,
    pub ftor: fn() -> Result<(), ()>,
    //pub reciprocal_to: Option<Weak<Mutex<Operator>>>,
}

impl IOperator for Operator {
    type OpType = fn() -> Result<(), ()>;
    fn new(signature: String, ftor: fn() -> Result<(), ()>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            signature,
            ftor,
            //reciprocal_to: None,
        }))
    }

    //pub(crate) fn show_reciprocal(&self) {
    //    println!(
    //        "{:?} is reciprocal to {:?}",
    //        self.signature,
    //        self.reciprocal_to
    //            .as_ref()
    //            .map(|s| Weak::upgrade(s).unwrap())
    //            .map(|s| Mutex::borrow(&s).signature.clone())
    //    );
    //}

    fn run(&self) -> Result<(), ()> {
        let x = self.ftor;
        x()
    }
}

pub mod operations {
    pub mod insert {
        pub use crate::editor::constants::*;
        pub use crate::editor::AppendBuffer;
        pub use crate::editor::Cursor;
        pub(crate) fn remove_char(
            cursor: &Cursor,
            data: &mut AppendBuffer,
        ) -> Option<EditorHealth> {
            if cursor.absx() == 0 && cursor.absy() == 0 {
                return Some(EditorHealth::Healthy); //Early return
            }
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
            let ind = cursor.calculate_file_index(
                &data.new_lines,
                cursor.absx() as usize,
                cursor.absy() as usize,
            ) - 1;
            data.remove(ind);
            cursor.move_cursor(&data.new_lines, CursorDirections::Left, 1);
            data.update_buffers();
            Some(EditorHealth::Healthy)
        }

        pub(crate) fn insert_char(
            cursor: &Cursor,
            data: &mut AppendBuffer,
            ch: u8,
        ) -> Option<EditorHealth> {
            log::debug!("Handling new {}", ch);
            log::debug!("{:?}", cursor);
            if ch == 13 as u8 || cursor.y() == cursor.cols {
                let ind = cursor.calculate_file_index(
                    &data.new_lines,
                    cursor.absx() as usize,
                    cursor.absy() as usize,
                );
                data.insert(ind, b'\n');
                cursor.set_x(0);
                cursor.up_y(1);
            } else {
                log::debug!("{:?}", cursor);
                let ind = cursor.calculate_file_index(
                    &data.new_lines,
                    cursor.absx() as usize,
                    cursor.absy() as usize,
                );
                data.insert(ind, ch);
                cursor.up_x(1);
            }
            //dirty = 1;
            Some(EditorHealth::Healthy)
        }
    }
    pub mod normal {
        pub use crate::editor::constants::*;
        pub use crate::editor::graphics::*;
        pub use crate::editor::AppendBuffer;
        pub use crate::editor::Cursor;
        pub use crate::editor::Terminal;
        pub use crate::editor::*;
        pub(crate) fn delete_operations(
            context: &EditorContext,
            cursor: &Cursor,
            terminal: &Terminal,
            data: &mut AppendBuffer,
            k: u8,
        ) -> Option<EditorHealth> {
            let mut x = [0u8; 4];
            let mut cmd = String::new();
            cmd.push(k as char);
            loop {
                if cmd.len() == 2 {
                    break;
                }
                let key = terminal.read_key().unwrap();
                if key == 27 as u8 {
                    break;
                }
                cmd.push(key as char);
            }

            let _ = match cmd.as_str() {
                "dd" => delete_line(context, cursor, terminal, data),
                "dk" => delete(context, cursor, terminal, data, CursorDirections::Up),
                "dj" => delete(context, cursor, terminal, data, CursorDirections::Down),
                "dl" => delete(context, cursor, terminal, data, CursorDirections::Right),
                "dh" => delete(context, cursor, terminal, data, CursorDirections::Left),
                _ => (),
            };

            Some(EditorHealth::Healthy)
        }
        pub(crate) fn delete(
            context: &EditorContext,
            cursor: &Cursor,
            terminal: &Terminal,
            data: &mut AppendBuffer,
            direction: CursorDirections,
        ) {
            match direction {
                CursorDirections::Up => {
                    cursor.move_cursor(&data.new_lines, CursorDirections::Up, 1);
                    delete_line(context, cursor, terminal, data);
                    delete_line(context, cursor, terminal, data);
                }
                CursorDirections::Down => {
                    delete_line(context, cursor, terminal, data);
                    delete_line(context, cursor, terminal, data);
                }
                CursorDirections::Left => {
                    let ind = cursor.calculate_file_index(
                        &data.new_lines,
                        cursor.absx() as usize,
                        cursor.absy() as usize,
                    ) - 1;
                    data.remove(ind);
                    cursor.move_cursor(&data.new_lines, CursorDirections::Left, 1);
                    data.update_buffers();
                }
                CursorDirections::Right => {
                    cursor.move_cursor(&data.new_lines, CursorDirections::Right, 1);
                    let ind = cursor.calculate_file_index(
                        &data.new_lines,
                        cursor.absx() as usize,
                        cursor.absy() as usize,
                    ) - 1;
                    data.remove(ind);
                    cursor.move_cursor(&data.new_lines, CursorDirections::Left, 1);
                    data.update_buffers();
                }
                _ => (),
            }
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
        }
        pub(crate) fn delete_line(
            context: &EditorContext,
            cursor: &Cursor,
            terminal: &Terminal,
            data: &mut AppendBuffer,
        ) {
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
            let mut line_begin = 0;
            if cursor.y() == 0 {
                line_begin = 0;
            } else {
                line_begin = data.new_lines[cursor.y() - 1] as usize;
            }
            let mut line_end = data.new_lines[cursor.y()] as usize;
            log::debug!("Deleting lines {}..{}", line_begin, line_end);
            data.remove_slice(line_begin..line_end);
            //cursor
            //    .lock().unwrap()
            //    .move_cursor(&data.new_lines, CursorDirections::Up, 1);
            context.dirty = 1;
            data.update_buffers();
        }
        pub(crate) fn clear_status_message_from_editor(
            terminal: Terminal,
            status_message: &String,
        ) {
            let status_len: usize = status_message.capacity();
            let mut cmd_buffer = String::new();
            cmd_buffer.push(b'/' as char);
            for _i in 0..status_len {
                cmd_buffer.push(' ');
            }
            terminal.write(cmd_buffer.as_bytes());
        }
        //pub(crate) fn find_in_file_blocking(
        //    graphics: &Arc<Mutex<Renderer>>,
        //    context: &Arc<Mutex<EditorContext>>,
        //    cursor: &Arc<Mutex<Cursor>>,
        //    terminal: &Arc<Mutex<Terminal>>,
        //    data: &AppendBuffer,
        //) -> Option<EditorHealth> {
        //    //In this mode we show user typed value.
        //    //self.terminal.control_echo(true);
        //    // TODO: Hacky render fix alter
        //    let mut t_c = &mut *cursor.
        //    let mut t_ter = &mut *terminal.
        //    t_c.naive_move_cursor_2d(&terminal, t_c.rows + 2, 0);
        //    clear_status_message_from_editor(
        //        terminal.clone(),
        //        &context.status_message,
        //    );
        //    let mut word = String::new();
        //    t_c.naive_move_cursor_2d(&terminal, t_c.rows + 2, 2);
        //    // REFREFREFACTOR
        //    loop {
        //        let key = terminal.read_key().unwrap();
        //        if key == b'\x7F' {
        //            //BACKSPACE is clicked
        //            // ALL this to have backspace HAHAHA
        //            word.pop();
        //            //t_c.naive_move_cursor(&terminal, CursorDirections::Left, 1);
        //            //terminal.write(b" ");
        //            //t_c.naive_move_cursor(&terminal, CursorDirections::Left, 1);
        //            continue;
        //        }
        //        if key == 27 as u8 || key == b'\r' {
        //            //Until ENTER is clicked
        //            break;
        //        } else {
        //            word.push(key as char);
        //            t_ter.write(&[key]);
        //            context.highlight_register = data.find(&word);
        //            let mut append_buffer = AppendBuffer::default();
        //            //draw(context, cursor, data, &mut append_buffer);
        //            //log::debug!("Found: {:?}", word);
        //            //log::debug!("Found: {:?}", context.highlight_register);
        //            graphics
        //                .lock()
        //                .unwrap()
        //                .render(t_c, &data, &mut append_buffer);
        //        }
        //    }
        //    Some(EditorHealth::Healthy)
        //}
    }
}

use std::sync::{Arc, Mutex};

pub use crate::editor::EditorControllers;

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
            cursor: &mut Cursor,
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
            cursor: &mut Cursor,
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
                let ind = cursor.calculate_file_index(
                    &data.new_lines,
                    cursor.absx() as usize,
                    cursor.absy() as usize,
                );
                log::debug!("{:?}", ind);
                data.insert(ind, ch);
                cursor.up_x(1);
            }
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
            context: &mut EditorContext,
            editor_controller: &mut EditorControllers,
            k: u8,
        ) -> Option<EditorHealth> {
            let mut x = [0u8; 4];
            let mut cmd = String::new();
            cmd.push(k as char);
            loop {
                if cmd.len() == 2 {
                    break;
                }
                let key = editor_controller.terminal.read_key().unwrap();
                if key == 27 as u8 {
                    break;
                }
                cmd.push(key as char);
            }

            let _ = match cmd.as_str() {
                "dd" => delete_line(context, editor_controller),
                "dk" => delete(context, editor_controller, CursorDirections::Up),
                "dj" => delete(context, editor_controller, CursorDirections::Down),
                "dl" => delete(context, editor_controller, CursorDirections::Right),
                "dh" => delete(context, editor_controller, CursorDirections::Left),
                _ => (),
            };

            Some(EditorHealth::Healthy)
        }
        pub(crate) fn delete(
            context: &mut EditorContext,
            editor_controller: &mut EditorControllers,
            direction: CursorDirections,
        ) {
            match direction {
                CursorDirections::Up => {
                    editor_controller.cursor.move_cursor(&editor_controller.data.new_lines, CursorDirections::Up, 1);
                    delete_line(context, editor_controller);
                    delete_line(context, editor_controller);
                }
                CursorDirections::Down => {
                    delete_line(context, editor_controller);
                    delete_line(context, editor_controller);
                }
                CursorDirections::Left => {
                    let ind = editor_controller.cursor.calculate_file_index(
                        &editor_controller.data.new_lines,
                        editor_controller.cursor.absx() as usize,
                        editor_controller.cursor.absy() as usize,
                    ) - 1;
                    editor_controller.data.remove(ind);
                    editor_controller.cursor.move_cursor(&editor_controller.data.new_lines, CursorDirections::Left, 1);
                    editor_controller.data.update_buffers();
                }
                CursorDirections::Right => {
                    editor_controller.cursor.move_cursor(&editor_controller.data.new_lines, CursorDirections::Right, 1);
                    let ind = editor_controller.cursor.calculate_file_index(
                        &editor_controller.data.new_lines,
                        editor_controller.cursor.absx() as usize,
                        editor_controller.cursor.absy() as usize,
                    ) - 1;
                    editor_controller.data.remove(ind);
                    editor_controller.cursor.move_cursor(&editor_controller.data.new_lines, CursorDirections::Left, 1);
                    editor_controller.data.update_buffers();
                }
                _ => (),
            }
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
        }
        pub(crate) fn delete_line(
            context: &mut EditorContext,
            editor_controller: &mut EditorControllers,
        ) {
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
            let mut line_begin = 0;
            if editor_controller.cursor.y() == 0 {
                line_begin = 0;
            } else {
                line_begin = editor_controller.data.new_lines[editor_controller.cursor.y() - 1] as usize;
            }
            let mut line_end = editor_controller.data.new_lines[editor_controller.cursor.y()] as usize;
            log::debug!("Deleting lines {}..{}", line_begin, line_end);
            editor_controller.data.remove_slice(line_begin..line_end);
            //cursor
            //    .lock().unwrap()
            //    .move_cursor(&data.new_lines, CursorDirections::Up, 1);
            context.dirty = 1;
            editor_controller.data.update_buffers();
        }
        pub(crate) fn clear_status_message_from_editor(
            terminal: &mut Terminal,
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
    }
}

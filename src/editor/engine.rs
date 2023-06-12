use std::{
    cell::Ref,
    cell::RefCell,
    rc::{Rc, Weak},
};

pub trait IOperator {
    type OpType;
    fn new(signature: String, function: Self::OpType) -> Rc<RefCell<Self>>;
    fn run(&self) -> Result<(), ()>;
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub signature: String,
    pub ftor: fn() -> Result<(), ()>,
    //pub reciprocal_to: Option<Weak<RefCell<Operator>>>,
}

impl IOperator for Operator {
    type OpType = fn() -> Result<(), ()>;
    fn new(signature: String, ftor: fn() -> Result<(), ()>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
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
    //            .map(|s| RefCell::borrow(&s).signature.clone())
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
        use std::{
            cell::Ref,
            cell::RefCell,
            rc::{Rc, Weak},
        };
        pub(crate) fn remove_char(
            cursor: &Rc<RefCell<Cursor>>,
            data: &mut AppendBuffer,
        ) -> Option<EditorHealth> {
            if cursor.borrow_mut().absx() == 0 && cursor.borrow_mut().absy() == 0 {
                return Some(EditorHealth::Healthy); //Early return
            }
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
            let ind = cursor.borrow().calculate_file_index(
                &data.new_lines,
                cursor.borrow().absx() as usize,
                cursor.borrow().absy() as usize,
            ) - 1;
            data.remove(ind);
            cursor
                .borrow_mut()
                .move_cursor(&data.new_lines, CursorDirections::Left, 1);
            data.update_buffers();
            Some(EditorHealth::Healthy)
        }

        pub(crate) fn insert_char(
            cursor: &Rc<RefCell<Cursor>>,
            data: &mut AppendBuffer,
            ch: u8,
        ) -> Option<EditorHealth> {
            log::debug!("Handling new {}", ch);
            log::debug!("{:?}", cursor.borrow());
            if ch == 13 as u8 || cursor.borrow().y() == cursor.borrow().cols {
                let ind = cursor.borrow().calculate_file_index(
                    &data.new_lines,
                    cursor.borrow().absx() as usize,
                    cursor.borrow().absy() as usize,
                );
                data.insert(ind, b'\n');
                cursor.borrow_mut().set_x(0);
                cursor.borrow_mut().up_y(1);
            } else {
                log::debug!("{:?}", cursor.borrow());
                let ind = cursor.borrow().calculate_file_index(
                    &data.new_lines,
                    cursor.borrow().absx() as usize,
                    cursor.borrow().absy() as usize,
                );
                data.insert(ind, ch);
                cursor.borrow_mut().up_x(1);
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
        use std::{
            cell::Ref,
            cell::RefCell,
            rc::{Rc, Weak},
        };
        pub(crate) fn delete_operations(
            context: &Rc<RefCell<EditorContext>>,
            cursor: &Rc<RefCell<Cursor>>,
            terminal: &Rc<RefCell<Terminal>>,
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
                let key = terminal.borrow_mut().read_key().unwrap();
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
            context: &Rc<RefCell<EditorContext>>,
            cursor: &Rc<RefCell<Cursor>>,
            terminal: &Rc<RefCell<Terminal>>,
            data: &mut AppendBuffer,
            direction: CursorDirections,
        ) {
            match direction {
                CursorDirections::Up => {
                    cursor
                        .borrow_mut()
                        .move_cursor(&data.new_lines, CursorDirections::Up, 1);
                    delete_line(context, cursor, terminal, data);
                    delete_line(context, cursor, terminal, data);
                }
                CursorDirections::Down => {
                    delete_line(context, cursor, terminal, data);
                    delete_line(context, cursor, terminal, data);
                }
                CursorDirections::Left => {
                    let ind = cursor.borrow().calculate_file_index(
                        &data.new_lines,
                        cursor.borrow().absx() as usize,
                        cursor.borrow().absy() as usize,
                    ) - 1;
                    data.remove(ind);
                    cursor
                        .borrow_mut()
                        .move_cursor(&data.new_lines, CursorDirections::Left, 1);
                    data.update_buffers();
                }
                CursorDirections::Right => {
                    cursor
                        .borrow_mut()
                        .move_cursor(&data.new_lines, CursorDirections::Right, 1);
                    let ind = cursor.borrow().calculate_file_index(
                        &data.new_lines,
                        cursor.borrow().absx() as usize,
                        cursor.borrow().absy() as usize,
                    ) - 1;
                    data.remove(ind);
                    cursor
                        .borrow_mut()
                        .move_cursor(&data.new_lines, CursorDirections::Left, 1);
                    data.update_buffers();
                }
                _ => (),
            }
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
        }
        pub(crate) fn delete_line(
            context: &Rc<RefCell<EditorContext>>,
            cursor: &Rc<RefCell<Cursor>>,
            terminal: &Rc<RefCell<Terminal>>,
            data: &mut AppendBuffer,
        ) {
            //let (index_l, index_r) = self.calculate_row_of_insert_indices(self.c_y as usize);
            //self.c_x = self.data.buffer[index_l..index_r].len() - 1;
            let mut line_begin = 0;
            if cursor.borrow().y() == 0 {
                line_begin = 0;
            } else {
                line_begin = data.new_lines[cursor.borrow().y() - 1] as usize;
            }
            let mut line_end = data.new_lines[cursor.borrow().y()] as usize;
            log::debug!("Deleting lines {}..{}", line_begin, line_end);
            data.remove_slice(line_begin..line_end);
            //cursor
            //    .borrow_mut()
            //    .move_cursor(&data.new_lines, CursorDirections::Up, 1);
            context.borrow_mut().dirty = 1;
            data.update_buffers();
        }
        pub(crate) fn clear_status_message_from_editor(
            terminal: &Rc<RefCell<Terminal>>,
            status_message: &String,
        ) {
            let status_len: usize = status_message.capacity();
            let mut cmd_buffer = String::new();
            cmd_buffer.push(b'/' as char);
            for _i in 0..status_len {
                cmd_buffer.push(' ');
            }
            terminal.borrow_mut().write(cmd_buffer.as_bytes());
        }
        pub(crate) fn find_in_file_blocking(
            context: &Rc<RefCell<EditorContext>>,
            cursor: &Rc<RefCell<Cursor>>,
            terminal: &Rc<RefCell<Terminal>>,
            data: &AppendBuffer,
        ) -> Option<EditorHealth> {
            //In this mode we show user typed value.
            //self.terminal.borrow_mut().control_echo(true);
            // TODO: Hacky render fix alter
            let mut t_c = cursor.borrow_mut();
            t_c.naive_move_cursor_2d(&terminal, t_c.rows + 2, 0);
            clear_status_message_from_editor(terminal, &context.borrow().status_message);
            let mut word = String::new();
            t_c.naive_move_cursor_2d(&terminal, t_c.rows + 2, 2);
            // REFREFREFACTOR
            loop {
                let key = terminal.borrow_mut().read_key().unwrap();
                if key == b'\x7F' {
                    //BACKSPACE is clicked
                    // ALL this to have backspace HAHAHA
                    word.pop();
                    //t_c.naive_move_cursor(&terminal, CursorDirections::Left, 1);
                    //terminal.borrow_mut().write(b" ");
                    //t_c.naive_move_cursor(&terminal, CursorDirections::Left, 1);
                    continue;
                }
                if key == 27 as u8 || key == b'\r' {
                    //Until ENTER is clicked
                    break;
                } else {
                    word.push(key as char);
                    terminal.borrow_mut().write(&[key]);
                    context.borrow_mut().highlight_register = data.find(&word);
                    let mut append_buffer = AppendBuffer::default();
                    //draw(context, cursor, data, &mut append_buffer);
                    log::debug!("Found: {:?}", word);
                    log::debug!("Found: {:?}", context.borrow_mut().highlight_register);
                    render(context, terminal, &mut t_c, &data, &mut append_buffer);
                }
            }
            Some(EditorHealth::Healthy)
        }
    }
}

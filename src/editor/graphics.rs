pub use crate::editor::constants::*;
pub use crate::editor::AppendBuffer;
pub use crate::editor::Cursor;
pub use crate::editor::Terminal;
pub use crate::editor::*;
use chrono::DateTime;
use std::{
    cell::Ref,
    cell::RefCell,
    rc::{Rc, Weak},
};

pub(crate) fn render(
    context: &Rc<RefCell<EditorContext>>,
    terminal: &Rc<RefCell<Terminal>>,
    cursor: &mut Cursor,
    data: &AppendBuffer,
    append_buffer: &mut AppendBuffer,
) {
    cursor.calculate_row_offset();
    append_buffer.append(b"\x1B[?25l");
    append_buffer.append(b"\x1B[H");
    draw(context, cursor, data, append_buffer);
    draw_status_bar(context, terminal, cursor, data, append_buffer);
    draw_message_bar(context, append_buffer);
    if context.borrow().h_reg > 0 && context.borrow().mode == EditorModes::Normal {
        let (i_x, i_y) = file_index_to_cursor(context, data);
        append_buffer.append_str(
            format!(
                "\x1B[{};{}H",
                i_y - cursor.row_offset + cursor.editor_configs.y_offset,
                i_x + cursor.editor_configs.x_offset
            )
            .as_str(),
        );
    } else {
        append_buffer.append_str(
            format!(
                "\x1B[{};{}H",
                (cursor.y() - cursor.row_offset) + 1,
                cursor.x() + 1
            )
            .as_str(),
        );
    }
    append_buffer.append(b"\x1B[?25h");
    append_buffer.write(&mut terminal.borrow_mut().stdout);
}

pub(crate) fn draw_message_bar(
    context: &Rc<RefCell<EditorContext>>,
    append_buffer: &mut AppendBuffer,
) {
    append_buffer.append(b"\x1B[K");
    append_buffer.append_str(&context.borrow().status_message);
}
pub(crate) fn draw_status_bar(
    context: &Rc<RefCell<EditorContext>>,
    terminal: &Rc<RefCell<Terminal>>,
    cursor: &Cursor,
    data: &AppendBuffer,
    append_buffer: &mut AppendBuffer,
) {
    append_buffer.append(b"\x1B[7m");
    //rstatus
    // ROW COUNT
    let mut status = String::new();
    let mut rstatus: String = format!(" [{}/{}] ", cursor.y() + 1, cursor.rows);
    // DATE TIME
    let datetime: DateTime<Utc> = context.borrow().status_message_time.into();
    let dt_string = datetime.format("%T %d/%m/%Y").to_string();
    rstatus.push_str(&dt_string);
    // status
    // PRINT mode
    let mode = match context.borrow().mode {
        EditorModes::Insert => String::from("[--INSERT--]"),
        EditorModes::Normal => String::from("[--NORMAL--]"),
    };
    status.push_str(&mode);
    //FILE NAME
    if context.borrow().files.is_empty() {
        status.push_str("[No Name]")
    } else {
        status.push_str(&context.borrow().files);
    }
    // DIRTY
    status.push_str(match context.borrow().dirty {
        0 => "",
        1 => "(modified)",
        _ => unreachable!("modified or not modified"),
    });

    //WRITE STAT
    if status.len() > cursor.cols {
        append_buffer.append_str(&status[0..cursor.cols as usize]);
    } else {
        append_buffer.append_str(&status);
    }
    let mut len = 0;
    while len < cursor.cols {
        if cursor.cols - len - status.len() == rstatus.len() {
            append_buffer.append_str(&rstatus);
            break;
        } else {
            append_buffer.append(b" ");
            len += 1;
        }
    }
    append_buffer.append(b"\x1B[m");
    append_buffer.append(b"\r\n");
}
pub(crate) fn draw(
    context: &Rc<RefCell<EditorContext>>,
    cursor: &mut Cursor,
    data: &AppendBuffer,
    append_buffer: &mut AppendBuffer,
) {
    for _y in 0..cursor.rows {
        let file_row = _y + cursor.row_offset;
        let absolute_numbers = &format!(
            "{:>width$} ",
            file_row,
            width = cursor.editor_configs.x_offset - 1
        )
        .to_string();
        append_buffer.append_str(&absolute_numbers);
        //cursor.editor_configs.x_offset = absolute_numbers.len();
        if file_row >= data.new_lines.len() && file_row <= data.new_lines.len() {
            append_buffer.append(b"~");
        } else {
            // TODO Ref here HANDLE COL limits
            let (index_l, index_r) =
                cursor.calculate_row_of_insert_indices(file_row as usize, &data.new_lines);
            // TODO Def very Bad
            let mut v: Vec<(usize, usize)> = Vec::new();

            for (_, (high_l, high_r)) in context.borrow().highlight_register.iter().enumerate() {
                if *high_l >= index_l && *high_r <= index_r {
                    v.push((*high_l, *high_r))
                }
            }
            let mut prev = -1 as i32;
            for (_, (high_l, high_r)) in v.iter().enumerate() {
                if prev < 0 {
                    append_buffer.append(&data.buffer[index_l..*high_l]);
                    append_buffer.append(constants::BIYellow); //YELLOW
                                                               //let offset = constants::BIYellow.len();
                    append_buffer.append(&data.buffer[*high_l..*high_r]);
                    append_buffer.append(constants::Color_Off);
                    append_buffer.append(b"\x1B[0m");
                    //append_buffer
                    //   .append(&data.buffer[*high_r..index_r]);
                } else {
                    append_buffer.append(&data.buffer[prev as usize..*high_l]);
                    append_buffer.append(constants::BIYellow); //YELLOW
                    append_buffer.append(&data.buffer[*high_l..*high_r]);
                    append_buffer.append(constants::Color_Off);
                }
                prev = *high_r as i32;
            }
            if v.len() == 0 {
                append_buffer.append(&data.buffer[index_l..index_r]);
            } else {
                append_buffer.append(&data.buffer[prev as usize..index_r]);
            }
        }
        append_buffer.append(b"\x1B[K");
        append_buffer.append(b"\r\n");
    }
}
pub(crate) fn file_index_to_cursor(
    context: &Rc<RefCell<EditorContext>>,
    data: &AppendBuffer,
) -> (usize, usize) {
    let mut i_x = 0;
    let mut i_y: i32 = -1;
    let value = context.borrow().highlight_register
        [context.borrow().h_reg % (context.borrow().highlight_register.len() + 2)]
        .0;
    for (i, d) in data.new_lines.iter().enumerate() {
        if value < *d as usize {
            i_y = i as i32;
            i_x = *d; // A value before
            break;
        }
    }
    if i_y < 0 {
        //Means it is the last file_index
        i_y = (data.new_lines.len() - 1) as i32;
        i_x = data.new_lines[data.new_lines.len() - 1];
    }
    //log::debug!("{:?}", data.new_lines);
    //log::debug!("{}, {}, {}", value, i_y, i_x);
    ((i_x - value as i32) as usize, i_y as usize)
}

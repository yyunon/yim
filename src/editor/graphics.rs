pub use crate::editor::constants::*;
pub use crate::editor::AppendBuffer;
pub use crate::editor::Cursor;
pub use crate::editor::Terminal;
pub use crate::editor::*;
use chrono::DateTime;

pub struct Renderer {
    pub terminal: Terminal,
    pub context: EditorContext,
    pub append_buffer: AppendBuffer,
}

impl Renderer {
    pub(crate) fn new(terminal: Terminal, context: EditorContext) -> Self {
        Self {
            terminal: terminal,
            context: context,
            append_buffer: AppendBuffer::default(),
        }
    }

    pub(crate) fn render(&mut self, cursor: &mut Cursor, data: &AppendBuffer) {
        //let context = &self.context;
        //let mut terminal = self.terminal;
        //let mut cursor = &*self.cursor.borrow_mut();
        log::debug!("{:?}", cursor);
        cursor.calculate_row_offset();
        self.append_buffer.append(b"\x1B[?25l");
        self.append_buffer.append(b"\x1B[H");

        Self::draw(self.context, cursor, data, &mut self.append_buffer);
        Self::draw_status_bar(self.context, cursor, data, &mut self.append_buffer);
        Self::draw_message_bar(self.context, &mut self.append_buffer);

        if self.context.h_reg > 0 && self.context.mode == EditorModes::Normal {
            let (i_x, i_y) = Self::file_index_to_cursor(self.context, data);
            self.append_buffer.append_str(
                format!(
                    "\x1B[{};{}H",
                    i_y - cursor.row_offset + cursor.editor_configs.y_offset,
                    i_x + cursor.editor_configs.x_offset
                )
                .as_str(),
            );
        } else if self.context.line_reg != usize::MAX && self.context.mode == EditorModes::Normal {
            let new_y = self.context.line_reg;
            cursor.set_y(new_y);
            self.append_buffer.append_str(
                format!(
                    "\x1B[{};{}H",
                    (cursor.y() - cursor.row_offset) + 1,
                    cursor.x() + 1
                )
                .as_str(),
            );
            //context.line_reg = usize::MAX;
        } else {
            self.append_buffer.append_str(
                format!(
                    "\x1B[{};{}H",
                    (cursor.y() - cursor.row_offset) + 1,
                    cursor.x() + 1
                )
                .as_str(),
            );
        }
        self.append_buffer.append(b"\x1B[?25h");
        self.append_buffer.write(self.terminal);
    }

    pub(crate) fn draw_message_bar(context: EditorContext, append_buffer: &mut AppendBuffer) {
        //let context = t_context;
        append_buffer.append(b"\x1B[K");
        append_buffer.append_str(&context.status_message);
    }
    pub(crate) fn draw_status_bar(
        context: EditorContext,
        cursor: &Cursor,
        data: &AppendBuffer,
        append_buffer: &mut AppendBuffer,
    ) {
        //let context = *t_context;
        append_buffer.append(b"\x1B[7m");
        //rstatus
        // ROW COUNT
        let mut status = String::new();
        let mut rstatus: String = format!(" [{}/{}] ", cursor.y() + 1, cursor.rows);
        // DATE TIME
        let datetime: DateTime<Utc> = context.status_message_time.into();
        let dt_string = datetime.format("%T %d/%m/%Y").to_string();
        rstatus.push_str(&dt_string);
        // status
        // PRINT mode
        let mode = match context.mode {
            EditorModes::Insert => String::from("[--INSERT--]"),
            EditorModes::Normal => String::from("[--NORMAL--]"),
        };
        status.push_str(&mode);
        //FILE NAME
        if context.files.file_name.is_empty() {
            status.push_str("[No Name]")
        } else {
            status.push_str(&context.files.file_name);
        }
        // DIRTY
        status.push_str(match context.dirty {
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
        context: EditorContext,
        cursor: &mut Cursor,
        data: &AppendBuffer,
        append_buffer: &mut AppendBuffer,
    ) {
        //let context = *t_context;
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

                for (_, (high_l, high_r)) in context.highlight_register.iter().enumerate() {
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
        context: EditorContext,
        data: &AppendBuffer,
    ) -> (usize, usize) {
        //let context = *t_context;
        let mut i_x = 0;
        let mut i_y: i32 = -1;
        let value =
            context.highlight_register[context.h_reg % (context.highlight_register.len() + 2)].0;
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
}

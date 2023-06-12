use log;
use std::env;
use std::io::{stdin, stdout};
use std::string::String;
use syslog::Facility;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

mod editor;
//pub use crate::editor::engine::OpStack;
//pub use crate::editor::engine::Operator;
pub use crate::editor::Editor;
pub use crate::editor::EditorConfigs;

fn main() -> std::io::Result<()> {
    //engine_example();
    let ed = EditorConfigs {
        x_offset: 4,
        y_offset: 0,
    };
    syslog::init(Facility::LOG_USER, log::LevelFilter::Debug, Some("yim")).unwrap();
    log::info!("Launching yim...");
    let args: Vec<String> = env::args().collect();

    let stdin = stdin();
    let stdout = stdout();
    let mut editor = Editor::new(stdin, stdout);
    editor.editor_configs = ed;

    editor.init_editor();
    let mut openfile = "";
    if args.len() > 1 {
        openfile = &args[1];
        editor.open(openfile)?;
    }
    editor.set_status_message("Welcome Yuksel!");
    editor.launch_engine();
    Ok(())
}

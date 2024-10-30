use log;
use std::env;
use std::io::{stdin, stdout};
use std::string::String;
use syslog::Facility;

use std::sync::{Arc, Mutex};

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

    let mut editor = Editor::new(stdin(), stdout(), Some(ed));

    editor.init_editor();
    if args.len() > 1 {
        editor.open(&args[1])?;
    }
    //editor.set_status_message("Welcome Yuksel!");
    editor.launch_editor();
    Ok(())
}

use log;
use std::env;
use std::io::{stdin, stdout};
use std::string::String;
use syslog::Facility;

mod editor;
pub use crate::editor::Editor;

fn main() -> std::io::Result<()> {
    syslog::init(Facility::LOG_USER, log::LevelFilter::Debug, Some("yim")).unwrap();
    log::info!("Launching yim...");
    let args: Vec<String> = env::args().collect();

    let stdin = stdin();
    let stdout = stdout();
    let mut editor = Editor::new(stdin, stdout);

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

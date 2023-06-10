#![feature(fn_traits, unboxed_closures)]

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
pub use crate::editor::engine::OpStack;
pub use crate::editor::engine::Operator;
pub use crate::editor::Editor;

fn test_1() -> bool {
    println!("This is a function 1");
    true
}
fn test_2() -> bool {
    println!("This is a function 2");
    true
}
fn engine_example() {
    let one: fn() -> bool = test_1;
    let two: fn() -> bool = test_2;
    let mut op_add = OpStack::NewOp("insert_char", one);
    let mut op_subtract = OpStack::NewOp("subtract_char", two);
    OpStack::BiDirectionalLink(&op_add, &op_subtract);
    OpStack::ShowRelationship(&op_add, &op_subtract);
    OpStack::Borrow(&op_add)("A test".to_string());
    std::process::exit(0);
}

fn main() -> std::io::Result<()> {
    engine_example();
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

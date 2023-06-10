#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CursorDirections {
    LineBegin,
    LineEnd,
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum EditorCommands {
    Exit,
    Healthy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum EditorModes {
    Normal,
    Insert,
}

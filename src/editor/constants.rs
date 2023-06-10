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
pub enum EditorHealth {
    Exit,
    Healthy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum EditorModes {
    Normal,
    Insert,
}

pub const Color_Off: &[u8] = b"\x1B[0m"; // Text Reset;
pub const Black: &[u8] = b"\x1B[0;30m"; // Black
pub const Red: &[u8] = b"\x1B[0;31m"; // Red
pub const Green: &[u8] = b"\x1B[0;32m"; // Green
pub const Yellow: &[u8] = b"\x1B[0;33m"; // Yellow
pub const Blue: &[u8] = b"\x1B[0;34m"; // Blue
pub const Purple: &[u8] = b"\x1B[0;35m"; // Purple
pub const Cyan: &[u8] = b"\x1B[0;36m"; // Cyan
pub const White: &[u8] = b"\x1B[0;37m"; // White
pub const BBlack: &[u8] = b"\x1B[1;30m"; // Black
pub const BRed: &[u8] = b"\x1B[1;31m"; // Red
pub const BGreen: &[u8] = b"\x1B[1;32m"; // Green
pub const BYellow: &[u8] = b"\x1B[1;33m"; // Yellow
pub const BBlue: &[u8] = b"\x1B[1;34m"; // Blue
pub const BPurple: &[u8] = b"\x1B[1;35m"; // Purple
pub const BCyan: &[u8] = b"\x1B[1;36m"; // Cyan
pub const BWhite: &[u8] = b"\x1B[1;37m"; // White
pub const UBlack: &[u8] = b"\x1B[4;30m"; // Black
pub const URed: &[u8] = b"\x1B[4;31m"; // Red
pub const UGreen: &[u8] = b"\x1B[4;32m"; // Green
pub const UYellow: &[u8] = b"\x1B[4;33m"; // Yellow
pub const UBlue: &[u8] = b"\x1B[4;34m"; // Blue
pub const UPurple: &[u8] = b"\x1B[4;35m"; // Purple
pub const UCyan: &[u8] = b"\x1B[4;36m"; // Cyan
pub const UWhite: &[u8] = b"\x1B[4;37m"; // White
pub const On_Black: &[u8] = b"\x1B[40m"; // Black
pub const On_Red: &[u8] = b"\x1B[41m"; // Red
pub const On_Green: &[u8] = b"\x1B[42m"; // Green
pub const On_Yellow: &[u8] = b"\x1B[43m"; // Yellow
pub const On_Blue: &[u8] = b"\x1B[44m"; // Blue
pub const On_Purple: &[u8] = b"\x1B[45m"; // Purple
pub const On_Cyan: &[u8] = b"\x1B[46m"; // Cyan
pub const On_White: &[u8] = b"\x1B[47m"; // White
pub const IBlack: &[u8] = b"\x1B[0;90m"; // Black
pub const IRed: &[u8] = b"\x1B[0;91m"; // Red
pub const IGreen: &[u8] = b"\x1B[0;92m"; // Green
pub const IYellow: &[u8] = b"\x1B[0;93m"; // Yellow
pub const IBlue: &[u8] = b"\x1B[0;94m"; // Blue
pub const IPurple: &[u8] = b"\x1B[0;95m"; // Purple
pub const ICyan: &[u8] = b"\x1B[0;96m"; // Cyan
pub const IWhite: &[u8] = b"\x1B[0;97m"; // White
pub const BIBlack: &[u8] = b"\x1B[1;90m"; // Black
pub const BIRed: &[u8] = b"\x1B[1;91m"; // Red
pub const BIGreen: &[u8] = b"\x1B[1;92m"; // Green
pub const BIYellow: &[u8] = b"\x1B[1;93m"; // Yellow
pub const BIBlue: &[u8] = b"\x1B[1;94m"; // Blue
pub const BIPurple: &[u8] = b"\x1B[1;95m"; // Purple
pub const BICyan: &[u8] = b"\x1B[1;96m"; // Cyan
pub const BIWhite: &[u8] = b"\x1B[1;97m"; // White
pub const On_IBlack: &[u8] = b"\x1B[0;100m"; // Black
pub const On_IRed: &[u8] = b"\x1B[0;101m"; // Red
pub const On_IGreen: &[u8] = b"\x1B[0;102m"; // Green
pub const On_IYellow: &[u8] = b"\x1B[0;103m"; // Yellow
pub const On_IBlue: &[u8] = b"\x1B[0;104m"; // Blue
pub const On_IPurple: &[u8] = b"\x1B[0;105m"; // Purple
pub const On_ICyan: &[u8] = b"\x1B[0;106m"; // Cyan
pub const On_IWhite: &[u8] = b"\x1B[0;107m"; // White

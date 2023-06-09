use std::fmt;
use std::io::{Read, Stdin, Stdout, Write};
use std::os::fd::AsRawFd;
use std::process::exit;

extern crate libc;

extern "C" {
    pub fn getchar() -> libc::c_int;
    pub fn tcgetattr(fd: libc::c_int, termios: *mut libc::termios) -> libc::c_int;
    pub fn tcsetattr(
        fd: libc::c_int,
        optional_actions: libc::c_int,
        termios: *mut libc::termios,
    ) -> libc::c_int;
    pub fn iscntrl(c: libc::c_int) -> libc::c_int;
    pub fn ioctl(fd: libc::c_int, request: libc::c_ulong, ...) -> libc::c_int;
}

fn die(msg: &char) {
    log::error!("{msg}");
    exit(1)
}
//#[derive(copy, clone)]
pub struct Terminal {
    pub(crate) raw: libc::termios,
    pub(crate) stdin: Stdin,
    pub(crate) stdout: Stdout,
}
impl Terminal {
    pub(crate) fn new(stdin: Stdin, stdout: Stdout) -> Self {
        let mut raw = libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_line: 0,
            c_cc: [0; 32],
            c_ispeed: 0,
            c_ospeed: 0,
        };
        unsafe { tcgetattr(libc::STDIN_FILENO, &mut raw) };
        Self {
            raw: raw,
            stdin: stdin,
            stdout: stdout,
        }
    }

    pub(crate) fn enable_raw_mode(&mut self) {
        let term_local_flags = libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN;
        let term_input_flags = libc::IXON | libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP;
        let term_postporc_flags = libc::OPOST;
        let term_c_flags = libc::CS8;
        unsafe { tcgetattr(self.stdin.as_raw_fd(), &mut self.raw) };
        let mut tmp_raw = self.raw.clone();

        tmp_raw.c_lflag &= !(term_local_flags);
        tmp_raw.c_iflag &= !(term_input_flags);
        tmp_raw.c_oflag &= !(term_postporc_flags);
        tmp_raw.c_cflag |= term_c_flags;

        unsafe { tcsetattr(self.stdin.as_raw_fd(), libc::TCSAFLUSH, &mut tmp_raw) };
    }

    pub(crate) fn control_echo(&mut self, enable: bool) {
        let mut tmp_raw = libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_line: 0,
            c_cc: [0; 32],
            c_ispeed: 0,
            c_ospeed: 0,
        };
        unsafe { tcgetattr(self.stdin.as_raw_fd(), &mut tmp_raw) };

        let term_local_flags = libc::ECHO;
        match enable {
            false => {
                tmp_raw.c_lflag &= !(term_local_flags);
            }
            true => {
                tmp_raw.c_lflag |= term_local_flags;
            }
        }

        unsafe { tcsetattr(self.stdin.as_raw_fd(), libc::TCSAFLUSH, &mut tmp_raw) };
    }

    pub(crate) fn term_size(&mut self) -> (usize, usize) {
        print!("{esc}[999C", esc = 27 as char);
        print!("{esc}[999B", esc = 27 as char);
        let mut w_size = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            let res = ioctl(self.stdout.as_raw_fd(), libc::TIOCGWINSZ, &mut w_size);
            if res == -1 || w_size.ws_col == 0 {
                log::debug!("ioctl unsuccesful getting from term");
                if self.stdout.write(b"\x1b[999C\x1b[998B").unwrap() != 12 {
                    log::error!("Cannot get winsize");
                }
                (0, 0)
            } else {
                (w_size.ws_col as usize, w_size.ws_row as usize)
            }
        }
    }

    pub(crate) fn disable_raw_mode(&mut self) {
        unsafe { tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &mut self.raw) };
    }
    pub(crate) fn read_key(&mut self) -> Option<u8> {
        let mut res: Option<u8> = None;
        let mut buf = [0u8; 1];
        let mut error_handle = false;
        self.stdout.lock().flush().unwrap();
        self.stdin.read_exact(&mut buf).map_err(|err| {
            log::error!("cannot read key {err}");
            error_handle = true;
        });
        if !error_handle {
            res = Some(buf[0]);
        }
        res
    }
}
impl fmt::Debug for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "terminal {{ raw: {{ c_iflag: {c_iflag:#02x}, c_oflag: {c_oflag:#02x}, c_cflag: {c_cflag:#02x}, c_lflag: {c_lflag:#02x}, c_line: {c_line:#02x}, c_cc: {c_cc:#02x?}, c_ispeed: {c_ispeed:#02x}, c_ospeed: {c_ospeed:#02x} }} }}",
            c_iflag = self.raw.c_iflag,
            c_oflag = self.raw.c_oflag,
            c_cflag = self.raw.c_cflag,
            c_lflag = self.raw.c_lflag,
            c_line = self.raw.c_line,
            c_cc = self.raw.c_cc,
            c_ispeed = self.raw.c_ispeed,
            c_ospeed = self.raw.c_ospeed,
        )
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        log::error!("disabling raw mode");
        self.disable_raw_mode();
    }
}

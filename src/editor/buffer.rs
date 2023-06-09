use std::fmt;
use std::io::{Stdout, Write};
struct SliceDisplay<'a, T: 'a>(&'a [T]);

impl<'a, T: fmt::Display + 'a> fmt::Display for SliceDisplay<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for item in self.0 {
            if !first {
                write!(f, ", {}", item)?;
            } else {
                write!(f, "{}", item)?;
            }
            first = false;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct AppendBuffer {
    pub(crate) buffer: Vec<u8>,
    pub(crate) size: u32,
    pub(crate) new_lines: Vec<i32>,
}
impl AppendBuffer {
    pub(crate) fn append(&mut self, input_stream: &[u8]) {
        self.buffer.extend_from_slice(input_stream);
        self.size += input_stream.len() as u32;
    }
    pub(crate) fn append_str(&mut self, input_stream: &str) {
        let s = input_stream.as_bytes();
        self.buffer.extend_from_slice(s);
        self.size += s.len() as u32;
    }
    pub(crate) fn insert(&mut self, index: usize, input_stream: u8) {
        self.buffer.insert(index, input_stream);
        self.size += 1;
        self.update_buffers();
    }
    pub(crate) fn remove(&mut self, index: usize) {
        self.buffer.remove(index);
        self.size -= 1;
        // update later
        //self.update_buffers();
    }
    pub(crate) fn find(&self, word: &str) -> Vec<(usize, usize)> {
        let buf_word = word.as_bytes();
        let w_len = buf_word.len();
        let mut result = Vec::new();
        let b_len = self.buffer.len();
        // TODO: n2 CAN YOU MAKE THIS FASTER?
        for x in 0..b_len {
            let mut found: bool = true;
            if x + w_len >= b_len {
                break;
            }
            for y in 0..w_len {
                //log::debug!("{},{} || {}=={}", x, y, self.buffer[x + y], buf_word[y]);
                found &= self.buffer[x + y] == buf_word[y];
            }
            if found {
                result.push((x, x + w_len));
            }
        }
        log::debug!("{:?}", result);
        result
        //self.buffer.iter().position(|r| *r == d).unwrap()
    }
    pub(crate) fn write(&mut self, stdout: &mut Stdout) {
        //log::debug!("{}", SliceDisplay(&self.buffer));
        //log::debug!("{:?}", SliceDisplay(&self.buffer));
        if stdout.write(&self.buffer).unwrap() as u32 != self.size {
            log::error!("Couldn't render");
        }
        self.free();
    }
    pub(crate) fn update_buffers(&mut self) {
        self.size = self.buffer.len() as u32;
        // TODO: Don't have to iterate evrytime
        self.new_lines = self
            .buffer
            .iter()
            .enumerate()
            .filter(|(_, d)| **d == b'\n')
            .map(|(i, _)| i as i32)
            .collect::<Vec<i32>>();
    }
    pub(crate) fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }
    pub(crate) fn free(&mut self) {
        self.size = 0;
        self.buffer.clear();
    }
}

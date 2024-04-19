use super::AppendBuffer;
use std::io::{BufReader, BufWriter, Read, Stdin, Stdout, Write};

pub struct File {
    pub file_name: String,
}

impl File {
    pub(crate) fn open(
        &mut self,
        input_file: &str,
        data: &mut AppendBuffer,
    ) -> std::io::Result<()> {
        self.file_name = input_file.to_string();
        log::debug!("{:?}", input_file);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(input_file)?;
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut data.buffer)?;
        data.update_buffers();
        Ok(())
    }
    pub(crate) fn save_buffer(
        &self,
        as_name: Vec<&str>,
        data: &mut AppendBuffer,
    ) -> Result<String, ()> {
        let mut status = String::new();
        let mut buffer_len = 0;
        for (_, fi) in as_name.into_iter().enumerate() {
            let mut f_name = self.file_name.clone();
            let t = fi.to_string();
            if !t.is_empty() {
                f_name = t;
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(f_name)
                .map_err(|err| {
                    status = format!("{} Cannot save to file", err);
                })
                .unwrap();
            let mut writer = BufWriter::new(&file);
            file.set_len(data.buffer.len() as u64).unwrap();
            buffer_len = writer.write(&data.buffer).unwrap();
        }
        status = format!("{} B written", buffer_len);
        Ok(status)
    }
}

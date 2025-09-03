use std::io::{Read, Result, Write};

pub fn copy<R: Read, W: Write>(reader: &mut R, writer: &mut W, buffer_size: usize) -> Result<()> {
    let mut buffer = vec![0u8; buffer_size];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
    }

    return Ok(());
}

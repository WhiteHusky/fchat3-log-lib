pub mod structs;
use crate::structs::FChatMessage;
use crate::structs::ReaderResult;
use std::io::{Read, Write, Seek, SeekFrom};
use byteorder;
use byteorder::LittleEndian;

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct FChatMessageReader {
    buf: Box<dyn Read>,
}

impl FChatMessageReader {
    pub fn new<T: 'static +  Read>(buf: T) -> FChatMessageReader {
        FChatMessageReader {buf: Box::new(buf)}
    }
}

impl Iterator for FChatMessageReader {
    type Item = ReaderResult;

    fn next(&mut self) -> Option<Self::Item> {
        Some(FChatMessage::read_from_buf(&mut self.buf))
    }
}

pub struct FChatMessageReaderReversed {
    buf: Box<dyn ReadSeek>,
}

impl FChatMessageReaderReversed {
    pub fn new<T: 'static +  ReadSeek>(mut buf: T) -> FChatMessageReaderReversed {
        buf.seek(SeekFrom::End(0)).unwrap();
        FChatMessageReaderReversed { buf: Box::new(buf)}
    }
}

fn reverse_seek<B: Seek + byteorder::ReadBytesExt>(buf: &mut B) -> std::io::Result<()> {
    buf.seek(SeekFrom::Current(-2))?;
    let reverse_feed = buf.read_u16::<LittleEndian>()?;
    buf.seek(SeekFrom::Current(-2))?;
    buf.seek(SeekFrom::Current((reverse_feed as i64) * -1))?;
    Ok(())
}

impl Iterator for FChatMessageReaderReversed {
    type Item = ReaderResult;

    fn next(&mut self) -> Option<Self::Item> {
        match reverse_seek(&mut self.buf) {
            Ok(_) => {}
            Err(_) => {return None}
        };
        let result = FChatMessage::read_from_buf(&mut self.buf);
        match reverse_seek(&mut self.buf) {
            Ok(_) => {}
            Err(_) => {return None}
        };
        Some(result)
    }
}

pub struct FChatWriter {
    buf: Box<dyn Write>,
}

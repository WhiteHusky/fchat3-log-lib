pub mod structs;
use crate::structs::FChatMessage;
use std::io::{Read, Write, Seek};

pub trait ReadSeek: Read + Seek {}

pub struct FChatMessageReader {
    buf: Box<dyn Read>,
}

impl FChatMessageReader {
    pub fn new<T: 'static +  Read>(buf: T) -> FChatMessageReader {
        FChatMessageReader {buf: Box::new(buf)}
    }
}

impl Iterator for FChatMessageReader {
    type Item = Result<FChatMessage, structs::ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(FChatMessage::read_from_buf(&mut self.buf))
    }
}

pub struct FChatMessageReaderReversed {
    buf: Box<dyn ReadSeek>,
}

pub struct FChatWriter {
    buf: Box<dyn Write>,
}

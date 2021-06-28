pub mod fchat_message;
pub mod error;
pub mod fchat_index;
use crate::fchat_message::FChatMessage;
use chrono::Datelike;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use std::io::{Write, Seek};
use std::io::{SeekFrom, Read};
use crate::fchat_message::{FChatMessageReaderResult, FChatMessage as ChatMessage};
use crate::fchat_index::FChatIndex as Index;
use crate::fchat_index::FChatIndexOffset as IndexOffset;
use crate::error::Error;

// TODO: Look into dynamic dispatch
// https://discordapp.com/channels/442252698964721669/443150878111694848/742291981849460736

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ReadBytesExt> ReadSeek for T {}

pub trait ReadSeekWrite: Read + Seek + Write {}
impl<T: Read + Seek + Write + WriteBytesExt + ReadBytesExt> ReadSeekWrite for T {}

pub trait SeekWrite: Seek + Write {}
impl<T: Seek + Write + WriteBytesExt> SeekWrite for T {}

pub fn different_day<A: Datelike, B: Datelike>(d1: A, d2: B) -> bool {
    d1.year() != d2.year() || d1.month() != d2.month() || d1.day() != d2.day()
}

pub struct FChatMessageReader<'a> {
    buf: Box<dyn Read + 'a>,
}

impl FChatMessageReader<'_> {
    pub fn new<'message_reader, T: 'message_reader +  Read>(buf: T) -> FChatMessageReader<'message_reader> {
        FChatMessageReader { buf: Box::new(buf) }
    }
}

impl Iterator for FChatMessageReader<'_> {
    type Item = FChatMessageReaderResult;

    fn next(&mut self) -> Option<Self::Item> {
        match ChatMessage::read_from_buf(&mut self.buf) {
            Ok(message) => {Some(Ok(message))}
            Err(Error::EOF(_)) => { None }
            Err(err) => { Some(Err(err)) }
        }
    }
}

pub struct FChatMessageReaderReversed {
    buf: Box<dyn ReadSeek>,
}

impl FChatMessageReaderReversed {
    pub fn new<T: 'static + ReadSeek>(mut buf: T) -> FChatMessageReaderReversed {
        buf.seek(SeekFrom::End(0)).unwrap();
        FChatMessageReaderReversed { buf: Box::new(buf) }
    }
}

fn reverse_seek<B: Seek + ReadBytesExt>(buf: &mut B) -> std::io::Result<()> {
    let reverse_feed = buf.read_u16::<LittleEndian>()?;
    // I'm seeking -4 for some reason. Have to remember why.
    buf.seek(SeekFrom::Current(-4 + (reverse_feed as i64) * -1))?;
    Ok(())
}

impl Iterator for FChatMessageReaderReversed {
    type Item = FChatMessageReaderResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buf.seek(SeekFrom::Current(0)) {
            Ok(pos) => {
                if pos == 0 {
                    return None;
                }
            }
            Err(err) => return Some(Err(Error::IOError(err)))
        }

        match reverse_seek(&mut self.buf) {
            Ok(_) => {}
            Err(err) => return Some(Err(Error::IOError(err)))
        };

        let return_value = match ChatMessage::read_from_buf(&mut self.buf) {
            Ok(message) => {Some(Ok(message))}
            Err(Error::EOF(_)) => { None }
            Err(err) => { Some(Err(err)) }
        };
        
        match reverse_seek(&mut self.buf) {
            Ok(_) => {}
            Err(err) => return Some(Err(Error::IOError(err)))
        };
        
        return_value
    }
}

pub struct FChatWriter {
    pub index: Index
}

impl FChatWriter {
    /// Using an existing idx, initialize the index with the idx file
    pub fn init_from_idx<A: Seek, B: ReadSeek>(log_buf: &mut A, idx_buf: &mut B) -> Result<Self, Error> {
        let index = Index::from_buf(idx_buf)?;
        log_buf.seek(SeekFrom::End(0))?;
        Ok(Self {
            index: index
        })
    }

    /// Using an existing log file and missing idx, initialize the index with the log and write to the idx file
    pub fn init_from_log<A: ReadSeek, B: SeekWrite>(log_buf: &mut A, idx_buf: &mut B, tab_name: String) -> Result<Self, Error> {
        let mut writer = Self::new(idx_buf, tab_name)?;
        loop {
            match FChatMessage::read_from_buf(log_buf) {
                Ok(message) => {
                    writer.update_idx_with_message(log_buf, idx_buf, message)?;
                }
                Err(Error::EOF(_)) => { break }
                Err(err) => { return Err(err); }
            }
        }
        Ok(writer)
    }

    pub fn new<B: SeekWrite>(idx_buf: &mut B, tab_name: String) -> Result<Self, Error>  {
        let writer = Self {
            index: Index::new(tab_name)
        };
        writer.index.write_header_to_buf(idx_buf)?;
        Ok(writer)
    }

    pub fn write_message<A: SeekWrite, B: SeekWrite>(&mut self, log_buf: &mut A, idx_buf: &mut B, message: FChatMessage) -> Result<(), Error> {
        message.write_to_buf(log_buf)?;
        self.update_idx_with_message(log_buf, idx_buf, message)?;
        Ok(())
    }

    fn update_idx_with_message<A: Seek, B: SeekWrite>(&mut self, log_buf: &mut A, idx_buf: &mut B, message: FChatMessage) -> Result<(), Error> {
        if match self.index.offsets.last() {
            Some(offset) => {
                different_day(message.datetime, offset.date)
            }
            None => { true }
        } {
            let offset_pos = log_buf.seek(SeekFrom::Current(0))? - (message.bytes_used() + 2);
            let offset = IndexOffset {
                date: message.datetime.date(),
                offset: offset_pos
            };
            offset.write_to_buf(idx_buf)?;
            self.index.offsets.push(offset);
            
        }
        Ok(())
    }
}

pub mod fchat_message;
pub mod error;
pub mod fchat_index;
use crate::fchat_message::FChatMessage;
use chrono::Datelike;
use std::{fs::File};
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

pub struct FChatWriter<'writer> {
    pub index: Index,
    pub log_buf: Box<dyn ReadSeekWrite + 'writer>,
    pub idx_buf: Box<dyn ReadSeekWrite + 'writer>,
}

impl FChatWriter<'_> {

    /// Using an existing idx file and log file, initialize the index with the idx file
    pub fn from_idx<'writer, A: 'writer + ReadSeekWrite, B: 'writer + ReadSeekWrite>(mut log_buf: A, mut idx_buf: B) -> Result<FChatWriter<'writer>, Error> {
        let index = Index::from_buf(&mut idx_buf)?;
        log_buf.seek(SeekFrom::End(0))?;
        Ok(FChatWriter {
            index: index,
            log_buf: Box::new(log_buf),
            idx_buf: Box::new(idx_buf),
        })
    }

    /// Using an existing log file and missing idx, initialize the index with the log and write to the idx file
    pub fn from_log<'writer, A: 'writer + ReadSeekWrite, B: 'writer + ReadSeekWrite>(log_buf: A, idx_buf: B, name: String) -> Result<FChatWriter<'writer>, Error> {
        let mut writer = Self::new(log_buf, idx_buf, name)?;
        writer.write_offsets_from_log()?;
        Ok(writer)
    }

    /// Using an existing log file and broken idx, repair the idx file.
    pub fn regenerate_idx(mut log_file: &File, mut idx_file: &File) -> Result<(), Error> {
        //idx_buf.set_len();
        let index = Index::read_header_from_buf(&mut idx_file)?;
        let new_size = idx_file.seek(SeekFrom::Current(0))?;
        idx_file.set_len(new_size)?;
        let mut writer = FChatWriter {
            index: index,
            log_buf: Box::new(log_file),
            idx_buf: Box::new(idx_file),
        };
        writer.write_offsets_from_log()?;
        log_file.seek(SeekFrom::Start(0))?;
        idx_file.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    pub fn new<'writer, A: 'writer + ReadSeekWrite, B: 'writer + ReadSeekWrite>(log_buf: A, idx_buf: B, name: String) -> Result<FChatWriter<'writer>, Error> {
        let mut writer = FChatWriter {
            index: Index {
                name: name,
                offsets: Vec::new()
            },
            log_buf: Box::new(log_buf),
            idx_buf: Box::new(idx_buf),
        };
        writer.index.write_header_to_buf(&mut writer.idx_buf)?;
        Ok(writer)
    }

    fn write_offsets_from_log(&mut self) -> Result<(), Error> {
        loop {
            match FChatMessage::read_from_buf(&mut self.log_buf) {
                Ok(message) => {
                    self.update_idx_with_message(message)?;
                }
                Err(Error::EOF(_)) => { break }
                Err(err) => { return Err(err); }
            }
        }
        Ok(())
    }

    /// Commit message to file and update the idx if needed.
    pub fn write_message(&mut self, message: FChatMessage) -> Result<(), Error> {
        //self.log_buf.seek(SeekFrom::End(0))?;
        message.write_to_buf(&mut self.log_buf)?;
        self.update_idx_with_message(message)?;
        Ok(())
    }

    /// This is typically reading with the reader or writing with the writer, so the seek location of the log_buf should be right after the read message.
    /// Aka, this function is run after reading a message or writing it from/to the log stream.
    fn update_idx_with_message(&mut self, message: FChatMessage) -> Result<(), Error> {
        if match self.index.offsets.last() {
            Some(offset) => {
                different_day(message.datetime, offset.date)
            }
            None => { true }
        } {
            let offset_pos = self.log_buf.seek(SeekFrom::Current(0))? - (message.bytes_used() + 2);
            let offset = IndexOffset {
                date: message.datetime.date(),
                offset: offset_pos
            };
            offset.write_to_buf(&mut self.idx_buf)?;
            self.index.offsets.push(offset);
            
        }
        Ok(())
    }
}

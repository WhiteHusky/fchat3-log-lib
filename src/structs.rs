use crate::structs::FChatMessageType::*;
pub mod errors;
use crate::structs::errors::{BadMessageLength, UnknownMessageType};
use byteorder;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{NaiveDateTime, NaiveDate, NaiveTime};
use std::convert::TryInto;
use std::error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
pub type ReaderResult = Result<FChatMessage, ParseError>;
pub type WriterResult = Result<(), ParseError>;
const SECONDS_IN_DAY: u32 = 86400;

#[derive(Clone)]
pub enum FChatMessageType {
    Message(String),
    Action(String),
    Ad(String),
    Roll(String),
    Warn(String),
    Event(String),
}

impl FChatMessageType {
    fn bytes_used(&self) -> usize {
        match self {
            Message(string) | Action(string) | Ad(string) | Roll(string) | Warn(string)
            | Event(string) => string.as_bytes().len(),
        }
    }

    fn as_byte(&self) -> u8 {
        match self {
            Message(_) => 0,
            Action(_) => 1,
            Ad(_) => 2,
            Roll(_) => 3,
            Warn(_) => 4,
            Event(_) => 5,
        }
    }

    fn from_byte(byte: u8, string: String) -> Result<FChatMessageType, UnknownMessageType> {
        match byte {
            0 => Ok(Message(string)),
            1 => Ok(Action(string)),
            2 => Ok(Ad(string)),
            3 => Ok(Roll(string)),
            4 => Ok(Warn(string)),
            5 => Ok(Event(string)),
            _ => Err(UnknownMessageType { found: byte }),
        }
    }
}

impl Display for FChatMessageType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let string: &String = match self {
            Message(this_string) => this_string,
            Action(this_string) => this_string,
            Ad(this_string) => this_string,
            Roll(this_string) => this_string,
            Warn(this_string) => this_string,
            Event(this_string) => this_string,
        };
        write!(f, "{}", string)
    }
}

impl Debug for FChatMessageType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "FChatMessageType::B!{} = {}", self.as_byte(), self)
    }
}

pub struct FChatMessage {
    pub datetime: NaiveDateTime,
    pub sender: String,
    pub body: FChatMessageType,
}

/*
    How FChat messages are stored on disk:
        epoch seconds:  u32:LE
        message type:   u8
        sender length:  u8
        sender:         str:utf8
        message length: u16:LE
        message:        str:utf8
        reverse feed:   u16:LE = (epoch seconds + message type + sender length + sender + message length + message)
          \_ This is used when the file is being read in reverse. Also used to verify the message was read properly.
*/

#[derive(Debug)]
pub enum ParseError {
    IOError(std::io::Error),
    EOF(std::io::Error),
    ConversionError(std::num::TryFromIntError),
    MessageLengthError(BadMessageLength),
    UTF8ConversionError(std::string::FromUtf8Error),
    UnknownMessageTypeError(UnknownMessageType)
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {
    fn description(&self) -> &str {
        "failed to write or read message"
    }
}

impl From<std::io::Error> for ParseError {
    fn from(item: std::io::Error) -> Self {
        match item.kind() {
            io::ErrorKind::UnexpectedEof => {Self::EOF(item)}
            _ => {Self::IOError(item)}
        }
    }
}

impl From<std::num::TryFromIntError> for ParseError {
    fn from(item: std::num::TryFromIntError) -> Self {
        Self::ConversionError(item)
    }
}

impl From<BadMessageLength> for ParseError {
    fn from(item: BadMessageLength) -> Self {
        Self::MessageLengthError(item)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(item: std::string::FromUtf8Error) -> Self {
        Self::UTF8ConversionError(item)
    }
}

impl From<UnknownMessageType> for ParseError {
    fn from(item: UnknownMessageType) -> Self {
        Self::UnknownMessageTypeError(item)
    }
}

impl FChatMessage {
    pub fn bytes_used(&self) -> usize {
        return 4 + 1 + 1 + self.sender.as_bytes().len() + 2 + self.body.bytes_used();
    }

    pub fn write_to_buf<B: io::Write + WriteBytesExt>(
        &self,
        buffer: &mut B,
    ) -> WriterResult {
        let epoch_seconds: u32 = self.datetime.timestamp().try_into()?;
        let sender_length: u8 = self.sender.as_bytes().len().try_into()?;
        let message_length: u16 = self.body.bytes_used().try_into()?;
        let log_length: u16 = self.bytes_used().try_into()?;
        buffer.write_u32::<LittleEndian>(epoch_seconds)?;
        buffer.write_u8(self.body.as_byte())?;
        buffer.write_u8(sender_length)?;
        buffer.write(self.sender.as_bytes())?;
        buffer.write_u16::<LittleEndian>(message_length)?;
        buffer.write(match &self.body {
            Message(string) | Action(string) | Ad(string) | Roll(string) | Warn(string)
            | Event(string) => string.as_bytes(),
        })?;
        buffer.write_u16::<LittleEndian>(log_length)?;
        Ok(())
    }

    pub fn read_from_buf<B: io::Read + ReadBytesExt>(
        buffer: &mut B,
    ) -> ReaderResult {
        let datetime_buf = buffer.read_u32::<LittleEndian>()?;
        let datetime: NaiveDateTime = NaiveDateTime::from_timestamp(datetime_buf as i64, 0);
        let message_type: u8 = buffer.read_u8()?;
        let sender_length: u8 = buffer.read_u8()?;
        let mut sender_raw = vec![0; sender_length as usize];
        buffer.read_exact(sender_raw.as_mut_slice())?;
        let sender = String::from_utf8(sender_raw)?;
        let message_length: u16 = buffer.read_u16::<LittleEndian>()?;
        let mut message_raw = vec![0; message_length as usize];
        buffer.read_exact(message_raw.as_mut_slice())?;
        let message = String::from_utf8(message_raw)?;
        let fchat_message = FChatMessage {
            datetime: datetime,
            sender: sender,
            body: FChatMessageType::from_byte(message_type, message)?,
        };
        let reverse_feed: u16 = buffer.read_u16::<LittleEndian>()?;
        let actual_length = fchat_message.bytes_used();
        if reverse_feed != actual_length.try_into()? {
            Err(ParseError::MessageLengthError(BadMessageLength {
                message: fchat_message,
                expected: reverse_feed as usize,
                found: actual_length,
            }))
        } else {
            Ok(fchat_message)
        }
    }
}

impl Debug for FChatMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "FChatMessage {{ datetime: {}, sender: {}, message: {}}}",
            self.datetime, self.sender, self.body
        )
    }
}

/* NOTE FOR FUTURE IDX PARSING:
    [
        name_len:u8,
        name_utf8:utf8_string * name_len,
        days_from_unix_epoch:u16, -- AKA unix_epoch/unix_epoch_seconds_in_day = day_epoch_midnight
        day_offset:[u8; 5], -- Offset in associated file (READ 5 BYTES AS LE)
        ...
    ]
*/

#[derive(Debug)]
pub enum IndexError {
    IOError(std::io::Error),
    UTF8ConversionError(std::string::FromUtf8Error),
    ConversionError(std::num::TryFromIntError),
}

impl Display for IndexError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Creating or parsing the index failed: {:?}",
            self
        )
    }
}

impl From<std::io::Error> for IndexError {
    fn from(item: std::io::Error) -> Self {
        Self::IOError(item)
    }
}

impl From<std::string::FromUtf8Error> for IndexError {
    fn from(item: std::string::FromUtf8Error) -> Self {
        Self::UTF8ConversionError(item)
    }
}

impl From<std::num::TryFromIntError> for IndexError {
    fn from(item: std::num::TryFromIntError) -> Self {
        Self::ConversionError(item)
    }
}


pub struct FChatIndexOffset {
    pub date: NaiveDate,
    pub offset: u64
}

pub struct FChatIndex {
    pub name: String,
    pub offsets: Vec<FChatIndexOffset>
}

impl FChatIndexOffset {
    pub fn write_to_buf<B: Write + WriteBytesExt>(
        &self,
        buffer: &mut B,
    ) -> Result<(), IndexError> {
        let unix_timestamp = self.date.and_time(NaiveTime::from_hms(0, 0, 0)).timestamp();
        let unix_days: u16 = (unix_timestamp / SECONDS_IN_DAY as i64).try_into()?;
        buffer.write_u16::<LittleEndian>(unix_days)?;
        let mut offset = self.offset.clone();
        for _ in 0..5 {
            let byte_to_write: u8 = (offset & 0xff).try_into()?;
            buffer.write_u8(byte_to_write)?;
            offset = offset >> 8;
        }
        Ok(())
    }

    pub fn read_from_buf<T: Read + ReadBytesExt>(buf: &mut T) -> Result<Self, IndexError> {
        let unix_days = buf.read_u16::<LittleEndian>()?;
        let unix_timestamp = (unix_days as u64 * SECONDS_IN_DAY as u64) as i64;
        let date = NaiveDateTime::from_timestamp(unix_timestamp, 0).date();
        let mut offset: u64 = 0;
        for n in 0..5 {
            offset |= (buf.read_u8()? as u64) << (n * 8);
        }
        Ok(Self {
            date: date,
            offset: offset
        })
    }
}

impl FChatIndex {
    pub fn new(name: String) -> Self {
        Self {
            name: name,
            offsets: Vec::new()
        }
    }

    pub fn write_header_to_buf<B: Write + WriteBytesExt>(
        &self,
        buffer: &mut B,
    ) -> Result<(), IndexError> {
        let name_len: u8 = self.name.len().try_into()?;
        buffer.write_u8(name_len)?;
        buffer.write(self.name.as_bytes())?;
        Ok(())
    }

    pub fn from_buf<T: Read + Seek + ReadBytesExt>(buf: &mut T) -> Result<Self, IndexError> {
        let name_length = buf.read_u8()?;
        let mut name_raw: Vec<u8> = vec![0; name_length as usize];
        buf.read_exact(name_raw.as_mut_slice())?;
        let name = String::from_utf8(name_raw)?;
        let mut offsets = Vec::<FChatIndexOffset>::new();
        //let size = buf.stream_len()?;
        loop {
            match FChatIndexOffset::read_from_buf(buf) {
                Ok(index_offset) => {
                    offsets.push(index_offset);
                }
                Err(IndexError::IOError(err)) => {
                    match err.kind() {
                        io::ErrorKind::UnexpectedEof => {
                            break
                        }
                        _ => { return Err(IndexError::IOError(err)); }
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        let index = FChatIndex {
            name: name,
            offsets: offsets,
        };
        Ok(index)
    }
}

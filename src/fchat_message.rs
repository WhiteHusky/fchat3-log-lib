use byteorder::ReadBytesExt;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use crate::error::Error;
use crate::error::{UnknownMessageType, BadMessageLength};
use crate::fchat_message::FChatMessageType::*;
use chrono::{NaiveDateTime};
use std::{io, fmt::{self, Debug, Display, Formatter}, convert::TryInto};
pub type FChatMessageReaderResult = Result<FChatMessage, Error>;
pub type FChatMessageWriterResult = Result<(), Error>;

/// Message types
#[derive(Clone, PartialEq, Eq)]
pub enum FChatMessageType {
    /// Chat message
    Message(String),
    /// Action message (/me)
    Action(String),
    /// Ad message
    Ad(String),
    /// Roll message
    Roll(String),
    /// Warn message
    Warn(String),
    /// Event message (status changes)
    Event(String),
}

impl FChatMessageType {
    fn bytes_used(&self) -> u64 {
        match self {
            Message(string) | Action(string) | Ad(string) | Roll(string) | Warn(string)
            | Event(string) => string.as_bytes().len() as u64,
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

/// Represents a chat message
#[derive(Clone, PartialEq, Eq)]
pub struct FChatMessage {
    /// Date of the [message](struct.FChatMessage.html)
    pub datetime: NaiveDateTime,
    /// Who sent the [message](struct.FChatMessage.html)
    pub sender: String,
    /// The body of the [message](struct.FChatMessage.html) as a [enum](enum.FChatMessageType.html)
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

impl FChatMessage {
    pub fn bytes_used(&self) -> u64 {
        return 4 + 1 + 1 + self.sender.as_bytes().len() as u64 + 2 + self.body.bytes_used();
    }

    pub fn write_to_buf<B: io::Write + WriteBytesExt>(
        &self,
        buffer: &mut B,
    ) -> FChatMessageWriterResult {
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
    ) -> FChatMessageReaderResult {
        let datetime_buf: u32;
        match buffer.read_u32::<LittleEndian>() {
            Ok(number) => { datetime_buf = number; }
            Err(err) => { return Err(Error::EOF(err)); }
        };
        let datetime: NaiveDateTime = NaiveDateTime::from_timestamp(datetime_buf as i64, 0);
        let message_type: u8 = buffer.read_u8()?;
        let sender_length: u8 = buffer.read_u8()?;
        let mut sender_raw: Vec<u8> = Vec::with_capacity(sender_length as usize);
        unsafe { sender_raw.set_len(sender_length as usize) }
        buffer.read_exact(&mut sender_raw)?;
        let sender = String::from_utf8(sender_raw)?;
        let message_length: u16 = buffer.read_u16::<LittleEndian>()?;
        let mut message_raw: Vec<u8> = Vec::with_capacity(message_length as usize);
        unsafe { message_raw.set_len(message_length as usize) }
        buffer.read_exact(&mut message_raw)?;
        let message = String::from_utf8(message_raw)?;
        let fchat_message = FChatMessage {
            datetime: datetime,
            sender: sender,
            body: FChatMessageType::from_byte(message_type, message)?,
        };
        let reverse_feed: u16 = buffer.read_u16::<LittleEndian>()?;
        let actual_length = fchat_message.bytes_used();
        if reverse_feed != actual_length.try_into()? {
            Err(Error::MessageLengthError(BadMessageLength {
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
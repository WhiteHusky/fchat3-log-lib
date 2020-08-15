use crate::fchat_message::FChatMessage;
use std::error;
use std::fmt;
use std::{io, fmt::{Debug, Display, Formatter}};

pub struct BadMessageLength {
    pub message: FChatMessage,
    pub expected: usize,
    pub found: usize,
}

impl Display for BadMessageLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "The message length was not correct. Expected {}, but read {}",
            self.expected, self.found
        )
    }
}

impl Debug for BadMessageLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "BadMessageLength {{ expected: {}, found: {} }}",
            self.expected, self.found
        )
    }
}

impl error::Error for BadMessageLength {
    fn description(&self) -> &str {
        "The message length was not correct and the message might be corrupted."
    }
}

pub struct UnknownMessageType {
    pub found: u8,
}

impl Display for UnknownMessageType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "The message type is unknown ({}). Did the log format change?",
            self.found
        )
    }
}

impl Debug for UnknownMessageType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "UnknownMessageType {{ found: {} }}", self.found)
    }
}

impl error::Error for UnknownMessageType {
    fn description(&self) -> &str {
        "The message type is unknown"
    }
}

pub struct ConformanceError {
    pub reason: String
}

impl Display for ConformanceError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "The information inputted does not conform to what is expected. Reason: {}",
            self.reason
        )
    }
}

impl Debug for ConformanceError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "ConformanceError {{ reason: {} }}",
            self.reason
        )
    }
}

impl error::Error for ConformanceError {
    fn description(&self) -> &str {
        "The information inputted does not conform to what is expected. The standard may have changed or there's a problem with a file."
    }
}

pub struct InadequateInformation {
    pub reason: String
}

impl Display for InadequateInformation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Not enough information specified. Reason: {}",
            self.reason
        )
    }
}

impl Debug for InadequateInformation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "InadequateInformation {{ reason: {} }}",
            self.reason
        )
    }
}

impl error::Error for InadequateInformation {
    fn description(&self) -> &str {
        "More information is needed to operate."
    }
}

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    EOF(std::io::Error),
    ConversionError(std::num::TryFromIntError),
    MessageLengthError(BadMessageLength),
    UTF8ConversionError(std::string::FromUtf8Error),
    UnknownMessageTypeError(UnknownMessageType),
    ConformanceError(ConformanceError),
    InadequateInformation(InadequateInformation)
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "failed to write or read message"
    }
}

impl From<std::io::Error> for Error {
    fn from(item: std::io::Error) -> Self {
        /*match item.kind() {
            io::ErrorKind::UnexpectedEof => {Self::EOF(item)}
            _ => {Self::IOError(item)}
        }*/
        Self::IOError(item)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(item: std::num::TryFromIntError) -> Self {
        Self::ConversionError(item)
    }
}

impl From<BadMessageLength> for Error {
    fn from(item: BadMessageLength) -> Self {
        Self::MessageLengthError(item)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(item: std::string::FromUtf8Error) -> Self {
        Self::UTF8ConversionError(item)
    }
}

impl From<UnknownMessageType> for Error {
    fn from(item: UnknownMessageType) -> Self {
        Self::UnknownMessageTypeError(item)
    }
}
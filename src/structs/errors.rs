use crate::structs::FChatMessage;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

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

impl Error for BadMessageLength {
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

impl Error for UnknownMessageType {
    fn description(&self) -> &str {
        "The message type is unknown"
    }
}

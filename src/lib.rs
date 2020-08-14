pub mod structs;
use crate::structs::errors::{ConformanceError, InadequateInformation};
use crate::structs::ReaderResult;
use crate::structs::{FChatIndex, FChatIndexOffset, FChatMessage, IndexError};
use byteorder;
use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono::Datelike;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use structs::ParseError;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
const SECONDS_IN_DAY: u32 = 86400;

// TODO: Look into dynamic dispatch
// https://discordapp.com/channels/442252698964721669/443150878111694848/742291981849460736

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub trait ReadSeekWrite: Read + Seek + Write {}
impl<T: Read + Seek + Write> ReadSeekWrite for T {}

pub struct FChatMessageReader<'a> {
    buf: Box<dyn Read + 'a>,
}

impl FChatMessageReader<'_> {
    pub fn new<'a, T: 'a +  Read>(buf: T) -> FChatMessageReader<'a> {
        FChatMessageReader { buf: Box::new(buf) }
    }
}

impl Iterator for FChatMessageReader<'_> {
    type Item = ReaderResult;

    fn next(&mut self) -> Option<Self::Item> {
        Some(FChatMessage::read_from_buf(&mut self.buf))
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
            Err(_) => return None,
        };
        let result = FChatMessage::read_from_buf(&mut self.buf);
        match reverse_seek(&mut self.buf) {
            Ok(_) => {}
            Err(_) => return None,
        };
        Some(result)
    }
}

#[derive(Debug)]
pub enum WriteError {
    IOError(std::io::Error),
    UTF8ConversionError(std::string::FromUtf8Error),
    ConformanceError(ConformanceError),
    IndexError(IndexError),
    InadequateInformation(InadequateInformation)
}

impl Display for WriteError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WriteError {
    fn description(&self) -> &str {
        "failed to write content"
    }
}

impl From<std::io::Error> for WriteError {
    fn from(item: std::io::Error) -> Self {
        Self::IOError(item)
    }
}

impl From<std::string::FromUtf8Error> for WriteError {
    fn from(item: std::string::FromUtf8Error) -> Self {
        Self::UTF8ConversionError(item)
    }
}

impl From<ConformanceError> for WriteError {
    fn from(item: ConformanceError) -> Self {
        Self::ConformanceError(item)
    }
}

impl From<IndexError> for WriteError {
    fn from(item: IndexError) -> Self {
        Self::IndexError(item)
    }
}

pub struct FChatWriter {
    log_path: PathBuf,
    idx_path: PathBuf,
    pub index: Option<FChatIndex>,
    fallback_name: Option<String>,
    log_fd: Option<File>,
    idx_fd: Option<File>,
}

// TODO: Probably should be replaced with something less...egregious.
fn get_message(result: ReaderResult) -> Option<FChatMessage> {
    // Thanks to @12Boti#0628 for showing that this is a thing
    match result {
        Ok(message) => Some(message),
        Err(ParseError::EOF(_)) => None,
        Err(err) => { eprintln!("{:?}", err); None},
        _ => None
    }
}

impl FChatWriter {
    pub fn new(
        log_path: PathBuf,
        idx_path: Option<PathBuf>,
        fallback_name: Option<String>,
    ) -> Result<Self, WriteError> {
        let idx_path = match idx_path {
            Some(path) => path,
            None => {
                let mut new_idx_path = log_path.clone();
                new_idx_path.set_extension("idx");
                new_idx_path
            }
        };
        let mut writer = FChatWriter {
            log_path: log_path,
            idx_path: idx_path,
            index: None,
            fallback_name: fallback_name,
            log_fd: None,
            idx_fd: None,
        };
        writer.parse_idx()?;
        Ok(writer)
    }
    fn parse_idx<'a>(&mut self) -> Result<(), WriteError> {
        let mut open_options = OpenOptions::new();
        open_options.read(true).write(true).create(true);
        self.idx_fd = Some(open_options.open(&self.idx_path)?);
        self.log_fd = Some(open_options.open(&self.log_path)?);
        let idx_fd = self.idx_fd.as_ref().unwrap();
        let idx_metadata = idx_fd.metadata()?;
        let idx_size = idx_metadata.len();
        if idx_size > 0 {
            let log_fd = self.log_fd.as_ref().unwrap();
            let mut idx_reader = BufReader::new(idx_fd);
            //let mut log_reader = BufReader::new(log_fd);
            eprintln!("{:?} has content, parsing...", self.idx_path);
            let index = FChatIndex::from_buf(&mut idx_reader)?;
            eprintln!("{} offsets registered", index.offsets.len());
            self.index = Some(index);
        } else {
            let log_fd = self.log_fd.as_ref().unwrap();
            let mut idx_writer = BufWriter::new(idx_fd);
            if self.fallback_name.is_none() {
                return Err(WriteError::InadequateInformation(InadequateInformation {
                    reason: "Need a name to initialize the index file with.".to_string()
                }))
            }
            let mut index = FChatIndex::new(self.fallback_name.as_ref().unwrap().clone());
            let log_size = log_fd.metadata()?.len();
            if log_size > 0 {
                eprintln!("{:?} has content, regenerating indexes...", self.log_path);
                let mut log_reader = BufReader::new(log_fd);
                let message_reader = FChatMessageReader::new(log_reader);
                let mut current_offset: u64 = 0;
                for result in message_reader {
                    match get_message(result) {
                        Some(message) => {
                            let mut do_write = false;
                            if index.offsets.is_empty() {
                                do_write = true;
                            } else {
                                let l_datetime = index.offsets.last().unwrap().date;
                                let m_datetime = message.datetime;
                                if m_datetime.year() != l_datetime.year() || m_datetime.month() != l_datetime.month() || m_datetime.day() != l_datetime.day() {
                                    do_write = true;
                                }
                            }
                            if do_write {
                                index.offsets.push(FChatIndexOffset {
                                    date: message.datetime.date(),
                                    offset: current_offset
                                })
                            }
                            current_offset += message.bytes_used() as u64 + 2;
                        }
                        None => {break}
                    }
                }
            }
            eprintln!("Created {} offsets", index.offsets.len());
            index.write_header_to_buf(&mut idx_writer)?;
            eprintln!("Writing to disk...");
            for offset in &index.offsets {
                offset.write_to_buf(&mut idx_writer)?;
            }
            eprintln!("Done");
            idx_writer.flush()?;
            self.index = Some(index);
        }
        Ok(())
    }
}

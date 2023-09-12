use std::io::Seek;
use byteorder::ReadBytesExt;
use std::io::Read;
use crate::error::Error;
use byteorder::LittleEndian;
use chrono::{NaiveTime, NaiveDate, NaiveDateTime};
use byteorder::WriteBytesExt;
use std::{convert::TryInto, io::Write};
pub type FChatIndexOffsetReaderResult = Result<FChatIndexOffset, Error>;
pub type FChatIndexOffsetWriterResult = Result<(), Error>;
pub type FChatIndexReaderResult = Result<FChatIndex, Error>;
pub type FChatIndexWriterResult = Result<(), Error>;

const SECONDS_IN_DAY: u32 = 86400;

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
    ) -> FChatIndexOffsetWriterResult {
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

    pub fn read_from_buf<T: Read + ReadBytesExt>(buf: &mut T) -> FChatIndexOffsetReaderResult {
        let unix_days: u16;
        match buf.read_u16::<LittleEndian>() {
            Ok(number) => { unix_days = number; }
            Err(err) => { return Err(Error::EOF(err)); }
        };
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
    ) -> FChatIndexWriterResult {
        let name_len: u8 = self.name.len().try_into()?;
        buffer.write_u8(name_len)?;
        buffer.write(self.name.as_bytes())?;
        Ok(())
    }

    pub fn read_header_from_buf<T: Read + ReadBytesExt>(buf: &mut T) -> FChatIndexReaderResult {
        let name_length = buf.read_u8()?;
        let mut name_raw: Vec<u8> = Vec::with_capacity(name_length as usize);
        unsafe { name_raw.set_len(name_length as usize) }
        buf.read_exact(&mut name_raw)?;
        let name = String::from_utf8(name_raw)?;
        let index = FChatIndex {
            name: name,
            offsets: Vec::new(),
        };
        Ok(index)
    }

    pub fn from_buf<T: Read + Seek + ReadBytesExt>(buf: &mut T) -> FChatIndexReaderResult {
        let mut index = Self::read_header_from_buf(buf)?;
        loop {
            match FChatIndexOffset::read_from_buf(buf) {
                Ok(index_offset) => {
                    index.offsets.push(index_offset);
                }
                Err(Error::EOF(_)) => {
                    break
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(index)
    }
}

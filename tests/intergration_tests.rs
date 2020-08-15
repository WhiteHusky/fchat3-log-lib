use chrono::{Local};
use std::error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;
use tempdir::TempDir;
use byteorder;
use byteorder::{ReadBytesExt};
use std::io::{BufReader};
const DIR_NAME: &str = "fchat3-log-lib-tests";
const TEST_CONTENTS: &[u8] = include_bytes!("carlen white");
const TEST_INDEX: &[u8] = include_bytes!("carlen white.idx");

use fchat3_log_lib::fchat_message::{FChatMessage, FChatMessageType};
use fchat3_log_lib::error::Error;
use fchat3_log_lib::{FChatMessageReader, FChatWriter};

type BoxedError = Box<dyn error::Error>;


fn create_dir() -> Result<TempDir, Box<dyn error::Error>> {
    Ok(TempDir::new(DIR_NAME)?)
}

fn create_test_file(dir: &TempDir, name: &str, contents: &[u8]) -> Result<std::fs::File, BoxedError> {
    let file_path_read = dir.path().join(name);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    let mut file = options.open(file_path_read)?;
    file.write(contents)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

#[test]
fn create() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let file_path = dir.path().join("write_me.log");
    let mut f = File::create(file_path)?;
    let message = FChatMessage {
        datetime: Local::now().naive_local(),
        body: FChatMessageType::Message(String::from("Hello World!")),
        sender: String::from("Someone"),
    };
    message.write_to_buf(&mut f)?;
    f.sync_all()?;
    assert_eq!(4, 2 + 2);
    dir.close()?;
    Ok(())
}
#[test]
fn create_and_read_basic() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let file_path = dir.path().join("write_me.log");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    let mut f = options.open(file_path)?;
    let temp_datetime = Local::now().naive_local();
    let temp_body = FChatMessageType::Message(String::from("Hello World!"));
    let temp_sender = String::from("Someone");
    let temp_message = FChatMessage {
        datetime: temp_datetime,
        body: temp_body.clone(),
        sender: temp_sender.clone(),
    };
    temp_message.write_to_buf(&mut f)?;
    f.sync_all()?;
    f.seek(SeekFrom::Start(0))?;
    let message = FChatMessage::read_from_buf(&mut f)?;
    println!("Read\n{:?}", message);
    assert_eq!(temp_datetime.timestamp(), message.datetime.timestamp());
    assert_eq!(temp_body.to_string(), message.body.to_string());
    assert_eq!(temp_sender, message.sender);
    dir.close()?;
    Ok(())
}
#[test]
fn can_create_1_to_1_from_native() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let mut f_r = create_test_file(&dir, "1.log", TEST_CONTENTS)?;
    let file_path_write = dir.path().join("2.log");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    let mut f_w = options.open(file_path_write)?;
    let size = f_r.metadata()?.len();
    while size > f_r.seek(SeekFrom::Current(0))? {
        let message = FChatMessage::read_from_buf(&mut f_r)?;
        message.write_to_buf(&mut f_w)?;
    }
    f_w.seek(SeekFrom::Start(0))?;
    assert_eq!(TEST_CONTENTS.len(), f_w.metadata()?.len() as usize);
    let mut i: u64 = 0;
    loop {
        if size <= f_w.seek(SeekFrom::Current(0))? {
            break;
        }
        let written_byte = f_w.read_u8()?;
        let source_byte = TEST_CONTENTS[i as usize];
        assert_eq!(written_byte, source_byte);
        //println!("Byte {} OK! ({})", i, written_byte);
        i = i + 1;
    }
    /*
    f_w.seek(SeekFrom::Start(0))?;
    let mut written_test_contents = Vec::new();
    f_w.read_to_end(&mut written_test_contents)?;
    assert_eq!(TEST_CONTENTS.len(), written_test_contents.len());
    for (i, byte) in written_test_contents.iter().enumerate() {
        assert_eq!(*byte, TEST_CONTENTS[i]);
        println!("Byte {} OK! ({})", i, byte)
    }*/
    dir.close()?;
    Ok(())
}

#[test]
fn read_using_reader() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let f_r = create_test_file(&dir, "1.log", TEST_CONTENTS)?;
    let reader = FChatMessageReader::new(f_r);
    for result in reader {
        let message = result?;
        println!("{:?}", message);
    }
    dir.close()?;
    Ok(())
}

fn check_index(log_fd: File, writer: FChatWriter) -> Result<(), BoxedError> {
    let index = writer.index.unwrap();
    let mut log_reader = BufReader::new(log_fd);
    let mut tested: u64 = 0;
    for offset in index.offsets {
        log_reader.seek(SeekFrom::Start(offset.offset))?;
        let message = FChatMessage::read_from_buf(&mut log_reader)?;
        eprintln!("{:?}", message);
        tested += 1;
    }
    eprintln!("Tested {} offsets", tested);
    Ok(())
}

#[test]
fn can_parse_index() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let log_fd = create_test_file(&dir, "1", TEST_CONTENTS)?;
    create_test_file(&dir, "1.idx", TEST_INDEX)?;
    //let idx_fd = create_test_file(&dir, "1.idx", TEST_INDEX)?;
    let writer = FChatWriter::new(dir.path().join("1"), Some(dir.path().join("1.idx")), None)?;
    check_index(log_fd, writer)?;
    dir.close()?;
    Ok(())
}

#[test]
fn can_create_index() -> Result<(), BoxedError> {
    let dir = create_dir()?;
    let log_fd = create_test_file(&dir, "1", TEST_CONTENTS)?;
    let writer = FChatWriter::new(dir.path().join("1"), Some(dir.path().join("1.idx")), Some("Carlen White".to_string()))?;
    check_index(log_fd, writer)?;
    dir.close()?;
    Ok(())
}
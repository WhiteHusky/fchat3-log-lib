use chrono::{Local};
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;
use tempdir::TempDir;
const DIR_NAME: &str = "fchat3-log-lib-tests";
const TEST_CONTENTS: &[u8] = include_bytes!("carlen white");

use fchat3_log_lib::structs::{FChatMessage, FChatMessageType, ParseError};
use fchat3_log_lib::{FChatMessageReader, FChatMessageReaderReversed, FChatWriter};

fn create_dir() -> Result<TempDir, Box<dyn Error>> {
    Ok(TempDir::new(DIR_NAME)?)
}

fn create_test_file(dir: &TempDir, name: &str) -> Result<std::fs::File, Box<dyn Error>> {
    let file_path_read = dir.path().join(name);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    let mut file = options.open(file_path_read)?;
    file.write(TEST_CONTENTS)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

#[test]
fn create() -> Result<(), Box<dyn Error>> {
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
fn create_and_read_basic() -> Result<(), Box<dyn Error>> {
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
fn can_create_1_to_1_from_native() -> Result<(), Box<dyn Error>> {
    let dir = create_dir()?;
    let mut f_r = create_test_file(&dir, "1.log")?;
    let file_path_write = dir.path().join("2.log");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    let mut f_w = options.open(file_path_write)?;
    let message_1 = FChatMessage::read_from_buf(&mut f_r)?;
    let message_2 = FChatMessage::read_from_buf(&mut f_r)?;
    println!("Message 1\n{:?}", message_1);
    println!("Message 2\n{:?}", message_2);
    message_1.write_to_buf(&mut f_w)?;
    message_2.write_to_buf(&mut f_w)?;
    f_w.seek(SeekFrom::Start(0))?;
    let mut written_test_contents = Vec::new();
    f_w.read_to_end(&mut written_test_contents)?;
    assert_eq!(TEST_CONTENTS.len(), written_test_contents.len());
    for (i, byte) in written_test_contents.iter().enumerate() {
        assert_eq!(*byte, TEST_CONTENTS[i]);
        println!("Byte {} OK! ({})", i, byte)
    }
    dir.close()?;
    Ok(())
}

#[test]
fn read_using_reader() -> Result<(), Box<dyn Error>> {
    let dir = create_dir()?;
    let f_r = create_test_file(&dir, "1.log")?;
    let reader = FChatMessageReader::new(f_r);
    for result in reader {
        match result {
            Ok(message) => {
                println!("{:?}", message);
            }
            Err(err) => {
                match err {
                    ParseError::EOF(_) => {
                        println!("Reached end of file!");
                    }
                    _ => {
                        println!("{:?}", err);
                        panic!()
                    }
                }
                break;
            }
        }
    }
    dir.close()?;
    Ok(())
}

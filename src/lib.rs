extern crate byteorder;
extern crate rustc_serialize;

use std::io;

#[derive(Debug)]
pub enum ProtoEncodeError {
    Io(io::Error),
    ByteOrder(byteorder::Error),
}

use std::{io, fmt, str};
use std::mem::size_of;
use std::slice::bytes;
use std::convert::From;
use byteorder;
use byteorder::{ByteOrder, NativeEndian};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ByteOrder(byteorder::Error),
    Utf8(str::Utf8Error),
    UnexpectedEOF,
    UnexpectedOctet { octet: u8, position: u64, },
}

pub trait ToBin {
    fn encode_len(&self) -> usize;
    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8];
}

pub trait FromBin: Sized {
    fn decode<'a>(area: &'a [u8]) -> Result<(Self, &'a [u8]), Error>;
}

macro_rules! try_get {
    ($data:ident, $ty:ty, $reader:ident) =>
        (if $data.len() < size_of::<$ty>() {
            return Err(Error::UnexpectedEOF)
        } else {
            (NativeEndian::$reader($data), &$data[size_of::<$ty>() ..])
        })
}

macro_rules! put_adv {
    ($area:expr, $ty:ty, $writer:ident, $value:expr) => ({
        let area = $area;
        NativeEndian::$writer(area, $value);
        &mut area[size_of::<$ty>() ..]
    })
}

trait U8Support {
    fn read_u8(buf: &[u8]) -> u8;
    fn write_u8(buf: &mut [u8], n: u8);
}

impl U8Support for NativeEndian {
    fn read_u8(buf: &[u8]) -> u8 {
        buf[0]
    }

    fn write_u8(buf: &mut [u8], n: u8) {
        buf[0] = n;
    }
}

macro_rules! try_get_str {
    ($buf:expr) => ({
        let buf = $buf;
        let (len, buf) = try_get!(buf, u32, read_u32);
        let len = len as usize;
        if buf.len() < len {
            return Err(Error::UnexpectedEOF)
        } else {
            (try!(str::from_utf8(&buf[0 .. len]).map_err(|e| Error::Utf8(e))).to_owned(), &buf[len ..])
        }
    })
}

macro_rules! put_str_adv {
    ($area:expr, $str:ident) => ({
        let src = $str.as_bytes();
        let dst = $area;
        let src_len_value = src.len() as u32;
        let area = put_adv!(dst, u32, write_u32, src_len_value);
        bytes::copy_memory(src, area);
        &mut area[src.len() ..]
    })
}

impl ToBin for String {
    fn encode_len(&self) -> usize {
        size_of::<u32>() + self.as_bytes().len()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        put_str_adv!(area, self)
    }
}

impl FromBin for String {
    fn decode<'a>(area: &'a [u8]) -> Result<(String, &'a [u8]), Error> {
        Ok(try_get_str!(area))
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::Io(ref err) => write!(f, "I/O error {}", err),
            &Error::ByteOrder(ref err) => write!(f, "byteorder related error: {}", err),
            &Error::Utf8(ref err) => write!(f, "utf8 related error: {}", err),
            &Error::UnexpectedEOF => f.write_str("unexpected EOF"),
            &Error::UnexpectedOctet { octet: o, position: p } => write!(f, "unexpected char {} at position {}", o, p),
        }
    }
}

impl From<byteorder::Error> for Error {
    fn from(err: byteorder::Error) -> Error {
        Error::ByteOrder(err)
    }
}

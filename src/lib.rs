extern crate byteorder;
extern crate rustc_serialize;

use std::{io, fmt, string};
use std::io::{Read, Write, Seek};
use std::fmt::Debug;
use std::convert::From;

#[derive(Clone, Debug)]
pub enum Req<UD> where UD: Clone + Debug {
    Init,
    Lookup(Workload<LookupTask<UD>>),
    Terminate,
}

#[derive(Clone, Debug)]
pub enum Workload<T> where T: Clone + Debug {
    Single(T),
    Many(Vec<T>),
}

#[derive(Clone, Debug)]
pub struct LookupTask<UD> where UD: Clone + Debug {
    pub text: String,
    pub result: LookupType,
    pub post_action: PostAction<UD>,
}

#[derive(Copy, Clone, Debug)]
pub enum LookupType { All, Best, BestOrMine }

#[derive(Clone, Debug)]
pub enum PostAction<UD> where UD: Clone + Debug {
    None,
    InsertNew { cond: InsertCond, assign: ClusterAssign, user_data: UD, },
}

#[derive(Copy, Clone, Debug)]
pub enum InsertCond {
    Always,
    BestSimLessThan(f64),
}

#[derive(Copy, Clone, Debug)]
pub enum ClusterAssign {
    ServerChoice,
    ClientChoice(u64),
}

#[derive(Clone, Debug)]
pub enum Rep<UD> where UD: Clone + Debug {
    InitAck,
    Result(Workload<LookupResult<UD>>),
    TerminateAck,
    Unexpected(Req<UD>),
    TooBusy,
    WantCrash,
}

#[derive(Clone, Debug)]
pub enum LookupResult<UD> where UD: Clone + Debug {
    EmptySet,
    Best(Match<UD>),
    Neighbours(Workload<Match<UD>>),
    Error(String),
}

#[derive(Clone, Debug)]
pub struct Match<UD> where UD: Clone + Debug {
    pub cluster_id: u64,
    pub similarity: f64,
    pub user_data: UD,
}

pub trait ProtoEncode {
    fn encode<W>(self, target: &mut W) -> Result<(), ProtoEncodeError> where W: Write;
}

pub trait ProtoDecode: Sized {
    fn decode<R>(source: &mut R) -> Result<Self, ProtoDecodeError> where R: Read + Seek;
}

#[derive(Debug)]
pub enum ProtoEncodeError {
    Io(io::Error),
    ByteOrder(byteorder::Error),
}

#[derive(Debug)]
pub enum ProtoDecodeError {
    Io(io::Error),
    ByteOrder(byteorder::Error),
    Utf8(string::FromUtf8Error),
    UnexpectedEOF,
    UnexpectedOctet { octet: u8, position: u64, },
}

impl fmt::Display for ProtoEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ProtoEncodeError::Io(ref err) => write!(f, "I/O error {}", err),
            &ProtoEncodeError::ByteOrder(ref err) => write!(f, "byteorder related error: {}", err),
        }
    }
}

impl fmt::Display for ProtoDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ProtoDecodeError::Io(ref err) => write!(f, "I/O error {}", err),
            &ProtoDecodeError::ByteOrder(ref err) => write!(f, "byteorder related error: {}", err),
            &ProtoDecodeError::Utf8(ref err) => write!(f, "utf8 related error: {}", err),
            &ProtoDecodeError::UnexpectedEOF => f.write_str("unexpected EOF"),
            &ProtoDecodeError::UnexpectedOctet { octet: o, position: p } => write!(f, "unexpected char {} at position {}", o, p),
        }
    }
}

impl From<io::Error> for ProtoEncodeError {
    fn from(err: io::Error) -> ProtoEncodeError {
        ProtoEncodeError::Io(err)
    }
}

impl From<io::Error> for ProtoDecodeError {
    fn from(err: io::Error) -> ProtoDecodeError {
        ProtoDecodeError::Io(err)
    }
}

impl From<byteorder::Error> for ProtoEncodeError {
    fn from(err: byteorder::Error) -> ProtoEncodeError {
        ProtoEncodeError::ByteOrder(err)
    }
}

impl From<byteorder::Error> for ProtoDecodeError {
    fn from(err: byteorder::Error) -> ProtoDecodeError {
        ProtoDecodeError::ByteOrder(err)
    }
}

impl From<string::FromUtf8Error> for ProtoDecodeError {
    fn from(err: string::FromUtf8Error) -> ProtoDecodeError {
        ProtoDecodeError::Utf8(err)
    }
}


use std::{io, fmt, str};
use std::sync::Arc;
use std::fmt::Debug;
use std::ops::Deref;
use std::mem::size_of;
use std::slice::bytes;
use std::convert::From;
use byteorder;
use byteorder::{ByteOrder, NativeEndian};
use super::{
    Workload,
    Trans, Req, LookupTask, PostAction, InsertCond, ClusterAssign, AssignCond, ClusterChoice, LookupType,
    Rep, LookupResult, Match
};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ByteOrder(byteorder::Error),
    Utf8(str::Utf8Error),
    UnexpectedEOF,
    InvalidTag(u8),
}

pub trait ToBin {
    fn encode_len(&self) -> usize;
    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8];
}

pub trait FromBin: Sized {
    fn decode<'a>(area: &'a [u8]) -> Result<(Self, &'a [u8]), Error>;
}

impl<T> ToBin for Arc<T> where T: ToBin {
    fn encode_len(&self) -> usize {
        self.deref().encode_len()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        self.deref().encode(area)
    }
}

impl<T> FromBin for Arc<T> where T: FromBin {
    fn decode<'a>(area: &'a [u8]) -> Result<(Arc<T>, &'a [u8]), Error> {
        let (obj, area) = try!(T::decode(area));
        Ok((Arc::new(obj), area))
    }
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
    fn read_i8(buf: &[u8]) -> i8;
    fn write_i8(buf: &mut [u8], n: i8);
    fn read_u8(buf: &[u8]) -> u8;
    fn write_u8(buf: &mut [u8], n: u8);
}

impl U8Support for NativeEndian {
    fn read_i8(buf: &[u8]) -> i8 { buf[0] as i8 }
    fn write_i8(buf: &mut [u8], n: i8) { buf[0] = n as u8; }
    fn read_u8(buf: &[u8]) -> u8 { buf[0] }
    fn write_u8(buf: &mut [u8], n: u8) { buf[0] = n; }
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

macro_rules! impl_bin {
    ($(($ty:ty, $reader:ident, $writer:ident)),*) => ($(
        impl ToBin for $ty {
            fn encode_len(&self) -> usize {
                size_of::<$ty>()
            }

            fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
                put_adv!(area, $ty, $writer, *self)
            }
        }

        impl FromBin for $ty {
            fn decode<'a>(area: &'a [u8]) -> Result<($ty, &'a [u8]), Error> {
                Ok(try_get!(area, $ty, $reader))
            }
        }
    )*)
}

impl_bin! {
    (i8, read_i8, write_i8),
    (u8, read_u8, write_u8),
    (i16, read_i16, write_i16),
    (u16, read_u16, write_u16),
    (i32, read_i32, write_i32),
    (u32, read_u32, write_u32),
    (i64, read_i64, write_i64),
    (u64, read_u64, write_u64),
    (f32, read_f32, write_f32),
    (f64, read_f64, write_f64)
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

impl<UD> ToBin for Trans<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &Trans::Async(ref req) => req.encode_len(),
            &Trans::Sync(ref req) => req.encode_len(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &Trans::Async(ref req) => {
                let area = put_adv!(area, u8, write_u8, 1);
                req.encode(area)
            },
            &Trans::Sync(ref req) => {
                let area = put_adv!(area, u8, write_u8, 2);
                req.encode(area)
            },
        }
    }
}

impl<UD> ToBin for Req<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &Req::Init | &Req::Terminate => 0,
            &Req::Lookup(ref workload) => workload.encode_len(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &Req::Init =>
                put_adv!(area, u8, write_u8, 1),
            &Req::Lookup(ref workload) => {
                let area = put_adv!(area, u8, write_u8, 2);
                workload.encode(area)
            },
            &Req::Terminate =>
                put_adv!(area, u8, write_u8, 3),
        }
    }
}

impl<UD> FromBin for Trans<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(Trans<UD>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) => {
                let (req, area) = try!(Req::decode(area));
                Ok((Trans::Async(req), area))
            },
            (2, area) => {
                let (req, area) = try!(Req::decode(area));
                Ok((Trans::Sync(req), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> FromBin for Req<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(Req<UD>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((Req::Init, area)),
            (2, area) => {
                let (workload, area) = try!(Workload::decode(area));
                Ok((Req::Lookup(workload), area))
            },
            (3, area) =>
                Ok((Req::Terminate, area)),
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<T> ToBin for Workload<T> where T: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &Workload::Single(ref value) => value.encode_len(),
            &Workload::Many(ref values) => size_of::<u32>() + values.iter().fold(0, |total, value| total + value.encode_len()),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &Workload::Single(ref value) => {
                let area = put_adv!(area, u8, write_u8, 1);
                value.encode(area)
            },
            &Workload::Many(ref values) => {
                let area = put_adv!(area, u8, write_u8, 2);
                let area = put_adv!(area, u32, write_u32, values.len() as u32);
                values.iter().fold(area, |area, value| value.encode(area))
            },
        }
    }
}

impl<T> FromBin for Workload<T> where T: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(Workload<T>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) => {
                let (value, area) = try!(T::decode(area));
                Ok((Workload::Single(value), area))
            },
            (2, area) => {
                let (len, mut area) = try_get!(area, u32, read_u32);
                let mut values = Vec::with_capacity(len as usize);
                for _ in 0 .. len {
                    let (value, next_area) = try!(T::decode(area));
                    values.push(value);
                    area = next_area;
                }
                Ok((Workload::Many(values), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> ToBin for LookupTask<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        self.text.encode_len() + self.result.encode_len() + self.post_action.encode_len()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        let area = self.text.encode(area);
        let area = self.result.encode(area);
        let area = self.post_action.encode(area);
        area
    }
}

impl<UD> FromBin for LookupTask<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(LookupTask<UD>, &'a [u8]), Error> {
        let (text, area) = try!(String::decode(area));
        let (result, area) = try!(LookupType::decode(area));
        let (post_action, area) = try!(PostAction::decode(area));
        Ok((LookupTask {
            text: text,
            result: result,
            post_action: post_action,
        }, area))
    }
}

impl ToBin for LookupType {
    fn encode_len(&self) -> usize {
        size_of::<u8>()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &LookupType::All => put_adv!(area, u8, write_u8, 1),
            &LookupType::Best => put_adv!(area, u8, write_u8, 2),
            &LookupType::BestOrMine => put_adv!(area, u8, write_u8, 3),
        }
    }
}

impl FromBin for LookupType {
    fn decode<'a>(area: &'a [u8]) -> Result<(LookupType, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) => Ok((LookupType::All, area)),
            (2, area) => Ok((LookupType::Best, area)),
            (3, area) => Ok((LookupType::BestOrMine, area)),
            (tag, _) => Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> ToBin for PostAction<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &PostAction::None =>
                0,
            &PostAction::InsertNew { cond: ref c, assign: ref a, user_data: ref u, } =>
                c.encode_len() + a.encode_len() + u.encode_len(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &PostAction::None =>
                put_adv!(area, u8, write_u8, 1),
            &PostAction::InsertNew { cond: ref c, assign: ref a, user_data: ref u, } => {
                let area = put_adv!(area, u8, write_u8, 2);
                let area = c.encode(area);
                let area = a.encode(area);
                let area = u.encode(area);
                area
            },
        }
    }
}

impl<UD> FromBin for PostAction<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(PostAction<UD>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((PostAction::None, area)),
            (2, area) => {
                let (cond, area) = try!(InsertCond::decode(area));
                let (assign, area) = try!(ClusterAssign::decode(area));
                let (user_data, area) = try!(UD::decode(area));
                Ok((PostAction::InsertNew { cond: cond, assign: assign, user_data: user_data, }, area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl ToBin for InsertCond {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &InsertCond::Always => 0,
            &InsertCond::BestSimLessThan(..) => size_of::<f64>(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &InsertCond::Always =>
                put_adv!(area, u8, write_u8, 1),
            &InsertCond::BestSimLessThan(sim) => {
                let area = put_adv!(area, u8, write_u8, 2);
                put_adv!(area, f64, write_f64, sim)
            },
        }
    }
}

impl FromBin for InsertCond {
    fn decode<'a>(area: &'a [u8]) -> Result<(InsertCond, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((InsertCond::Always, area)),
            (2, area) => {
                let (sim, area) = try_get!(area, f64, read_f64);
                Ok((InsertCond::BestSimLessThan(sim), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl ToBin for ClusterAssign {
    fn encode_len(&self) -> usize {
        self.cond.encode_len() + self.choice.encode_len()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        let area = self.cond.encode(area);
        let area = self.choice.encode(area);
        area
    }
}

impl FromBin for ClusterAssign {
    fn decode<'a>(area: &'a [u8]) -> Result<(ClusterAssign, &'a [u8]), Error> {
        let (cond, area) = try!(AssignCond::decode(area));
        let (choice, area) = try!(ClusterChoice::decode(area));
        Ok((ClusterAssign {
            cond: cond,
            choice: choice,
        }, area))
    }
}

impl ToBin for AssignCond {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &AssignCond::Always => 0,
            &AssignCond::BestSimLessThan(..) => size_of::<f64>(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &AssignCond::Always =>
                put_adv!(area, u8, write_u8, 1),
            &AssignCond::BestSimLessThan(sim) => {
                let area = put_adv!(area, u8, write_u8, 2);
                put_adv!(area, f64, write_f64, sim)
            },
        }
    }
}

impl FromBin for AssignCond {
    fn decode<'a>(area: &'a [u8]) -> Result<(AssignCond, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((AssignCond::Always, area)),
            (2, area) => {
                let (sim, area) = try_get!(area, f64, read_f64);
                Ok((AssignCond::BestSimLessThan(sim), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl ToBin for ClusterChoice {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &ClusterChoice::ServerChoice => 0,
            &ClusterChoice::ClientChoice(..) => size_of::<u64>(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &ClusterChoice::ServerChoice =>
                put_adv!(area, u8, write_u8, 1),
            &ClusterChoice::ClientChoice(cluster_id) => {
                let area = put_adv!(area, u8, write_u8, 2);
                put_adv!(area, u64, write_u64, cluster_id)
            },
        }
    }
}

impl FromBin for ClusterChoice {
    fn decode<'a>(area: &'a [u8]) -> Result<(ClusterChoice, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((ClusterChoice::ServerChoice, area)),
            (2, area) => {
                let (cluster_id, area) = try_get!(area, u64, read_u64);
                Ok((ClusterChoice::ClientChoice(cluster_id), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> ToBin for Rep<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &Rep::InitAck | &Rep::TerminateAck | &Rep::TooBusy | &Rep::WantCrash => 0,
            &Rep::Result(ref workload) => workload.encode_len(),
            &Rep::Unexpected(ref req) => req.encode_len(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &Rep::InitAck =>
                put_adv!(area, u8, write_u8, 1),
            &Rep::Result(ref workload) => {
                let area = put_adv!(area, u8, write_u8, 2);
                workload.encode(area)
            },
            &Rep::TerminateAck =>
                put_adv!(area, u8, write_u8, 3),
            &Rep::Unexpected(ref req) => {
                let area = put_adv!(area, u8, write_u8, 4);
                req.encode(area)
            },
            &Rep::TooBusy =>
                put_adv!(area, u8, write_u8, 5),
            &Rep::WantCrash =>
                put_adv!(area, u8, write_u8, 6),
        }
    }
}

impl<UD> FromBin for Rep<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(Rep<UD>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((Rep::InitAck, area)),
            (2, area) => {
                let (workload, area) = try!(Workload::decode(area));
                Ok((Rep::Result(workload), area))
            },
            (3, area) =>
                Ok((Rep::TerminateAck, area)),
            (4, area) => {
                let (req, area) = try!(Req::decode(area));
                Ok((Rep::Unexpected(req), area))
            },
            (5, area) =>
                Ok((Rep::TooBusy, area)),
            (6, area) =>
                Ok((Rep::WantCrash, area)),
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> ToBin for LookupResult<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u8>() + match self {
            &LookupResult::EmptySet => 0,
            &LookupResult::Best(ref m) => m.encode_len(),
            &LookupResult::Neighbours(ref workload) => workload.encode_len(),
            &LookupResult::Error(ref e) => e.encode_len(),
        }
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        match self {
            &LookupResult::EmptySet =>
                put_adv!(area, u8, write_u8, 1),
            &LookupResult::Best(ref m) => {
                let area = put_adv!(area, u8, write_u8, 2);
                m.encode(area)
            },
            &LookupResult::Neighbours(ref workload) => {
                let area = put_adv!(area, u8, write_u8, 3);
                workload.encode(area)
            },
            &LookupResult::Error(ref e) => {
                let area = put_adv!(area, u8, write_u8, 4);
                e.encode(area)
            },
        }
    }
}

impl<UD> FromBin for LookupResult<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(LookupResult<UD>, &'a [u8]), Error> {
        match try_get!(area, u8, read_u8) {
            (1, area) =>
                Ok((LookupResult::EmptySet, area)),
            (2, area) => {
                let (m, area) = try!(Match::decode(area));
                Ok((LookupResult::Best(m), area))
            },
            (3, area) => {
                let (workload, area) = try!(Workload::decode(area));
                Ok((LookupResult::Neighbours(workload), area))
            },
            (4, area) => {
                let (e, area) = try!(String::decode(area));
                Ok((LookupResult::Error(e), area))
            },
            (tag, _) =>
                Err(Error::InvalidTag(tag)),
        }
    }
}

impl<UD> ToBin for Match<UD> where UD: ToBin + Debug {
    fn encode_len(&self) -> usize {
        size_of::<u64>() + size_of::<f64>() + self.user_data.encode_len()
    }

    fn encode<'a>(&self, area: &'a mut [u8]) -> &'a mut [u8] {
        let area = put_adv!(area, u64, write_u64, self.cluster_id);
        let area = put_adv!(area, f64, write_f64, self.similarity);
        let area = self.user_data.encode(area);
        area
    }
}

impl<UD> FromBin for Match<UD> where UD: FromBin + Debug {
    fn decode<'a>(area: &'a [u8]) -> Result<(Match<UD>, &'a [u8]), Error> {
        let (cluster_id, area) = try_get!(area, u64, read_u64);
        let (similarity, area) = try_get!(area, f64, read_f64);
        let (user_data, area) = try!(UD::decode(area));
        Ok((Match {
            cluster_id: cluster_id,
            similarity: similarity,
            user_data: user_data,
        }, area))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::Io(ref err) => write!(f, "I/O error {}", err),
            &Error::ByteOrder(ref err) => write!(f, "byteorder related error: {}", err),
            &Error::Utf8(ref err) => write!(f, "utf8 related error: {}", err),
            &Error::UnexpectedEOF => f.write_str("unexpected EOF"),
            &Error::InvalidTag(tag) => write!(f, "invalid proto tag {}", tag),
        }
    }
}

impl From<byteorder::Error> for Error {
    fn from(err: byteorder::Error) -> Error {
        Error::ByteOrder(err)
    }
}

#[cfg(test)]
mod test {
    use super::{ToBin, FromBin};
    use super::super::{
        Workload,
        Trans, Req, LookupTask, PostAction, InsertCond, AssignCond, ClusterChoice, ClusterAssign, LookupType,
        Rep, LookupResult, Match
    };

    fn encode_decode<T>(value: T) -> T where T: ToBin + FromBin {
        let required = value.encode_len();
        let mut packet: Vec<_> = (0 .. required).map(|_| 0).collect();
        {
            let area = value.encode(&mut packet);
            assert_eq!(area.len(), 0);
        }
        let (decoded, area) = <T as FromBin>::decode(&packet).unwrap();
        assert_eq!(area.len(), 0);
        decoded
    }

    fn encode_decode_req(req: Trans<String>) -> Trans<String> { encode_decode(req) }
    fn encode_decode_rep(rep: Rep<String>) -> Rep<String> { encode_decode(rep) }

    #[test]
    fn req_00_async() {
        match encode_decode_req(Trans::Async(Req::Init)) {
            Trans::Async(Req::Init) => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_00_sync() {
        match encode_decode_req(Trans::Sync(Req::Init)) {
            Trans::Sync(Req::Init) => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_01() {
        match encode_decode_req(Trans::Async(Req::Lookup(Workload::Single(LookupTask {
            text: "hello world".to_owned(),
            result: LookupType::All,
            post_action: PostAction::None,
        })))) {
            Trans::Async(Req::Lookup(Workload::Single(LookupTask {
                text: ref lookup_text,
                result: LookupType::All,
                post_action: PostAction::None,
            }))) if lookup_text == "hello world" => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_02() {
        match encode_decode_req(Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
            text: "hello world".to_owned(),
            result: LookupType::BestOrMine,
            post_action: PostAction::InsertNew {
                cond: InsertCond::Always,
                assign: ClusterAssign {
                    cond: AssignCond::Always,
                    choice: ClusterChoice::ServerChoice,
                },
                user_data: "some data".to_owned(),
            },
        })))) {
            Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
                text: ref lookup_text,
                result: LookupType::BestOrMine,
                post_action: PostAction::InsertNew {
                    cond: InsertCond::Always,
                    assign: ClusterAssign {
                        cond: AssignCond::Always,
                        choice: ClusterChoice::ServerChoice,
                    },
                    user_data: ref lookup_user_data,
                },
            }))) if lookup_text == "hello world" && lookup_user_data == "some data" => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_03() {
        match encode_decode_req(Trans::Async(Req::Lookup(Workload::Single(LookupTask {
            text: "hello world".to_owned(),
            result: LookupType::Best,
            post_action: PostAction::InsertNew {
                cond: InsertCond::BestSimLessThan(0.5),
                assign: ClusterAssign {
                    cond: AssignCond::Always,
                    choice: ClusterChoice::ClientChoice(177),
                },
                user_data: "some data".to_owned(),
            },
        })))) {
            Trans::Async(Req::Lookup(Workload::Single(LookupTask {
                text: ref lookup_text,
                result: LookupType::Best,
                post_action: PostAction::InsertNew {
                    cond: InsertCond::BestSimLessThan(0.5),
                    assign: ClusterAssign {
                        cond: AssignCond::Always,
                        choice: ClusterChoice::ClientChoice(177),
                    },
                    user_data: ref lookup_user_data,
                },
            }))) if lookup_text == "hello world" && lookup_user_data == "some data" => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_04() {
        match encode_decode_req(Trans::Sync(Req::Terminate)) {
            Trans::Sync(Req::Terminate) => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_05() {
        match encode_decode_req(Trans::Async(Req::Lookup(Workload::Many(vec![LookupTask {
            text: "hello, world".to_owned(),
            result: LookupType::All,
            post_action: PostAction::None,
        }, LookupTask {
            text: "hello, cat".to_owned(),
            result: LookupType::Best,
            post_action: PostAction::None,
        }, LookupTask {
            text: "hello, dog".to_owned(),
            result: LookupType::BestOrMine,
            post_action: PostAction::None,
        }])))) {
            Trans::Async(Req::Lookup(Workload::Many(ref workloads))) => {
                match workloads.get(0) {
                    Some(&LookupTask { text: ref t, result: LookupType::All, post_action: PostAction::None, }) if t == "hello, world" => (),
                    other => panic!("bad workload 0: {:?}", other),
                }
                match workloads.get(1) {
                    Some(&LookupTask { text: ref t, result: LookupType::Best, post_action: PostAction::None, }) if t == "hello, cat" => (),
                    other => panic!("bad workload 1: {:?}", other),
                }
                match workloads.get(2) {
                    Some(&LookupTask { text: ref t, result: LookupType::BestOrMine, post_action: PostAction::None, }) if t == "hello, dog" => (),
                    other => panic!("bad workload 2: {:?}", other),
                }
            },
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_00() {
        match encode_decode_rep(Rep::InitAck) {
            Rep::InitAck => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_01() {
        match encode_decode_rep(Rep::TerminateAck) {
            Rep::TerminateAck => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_02() {
        match encode_decode_rep(Rep::TooBusy) {
            Rep::TooBusy => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_03() {
        match encode_decode_rep(Rep::WantCrash) {
            Rep::WantCrash => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_04() {
        match encode_decode_rep(Rep::Result(Workload::Single(LookupResult::EmptySet))) {
            Rep::Result(Workload::Single(LookupResult::EmptySet)) => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_05() {
        match encode_decode_rep(Rep::Result(Workload::Single(LookupResult::Best(Match {
            cluster_id: 177,
            similarity: 0.5,
            user_data: "some data".to_owned(),
        })))) {
            Rep::Result(Workload::Single(LookupResult::Best(Match {
                cluster_id: 177,
                similarity: 0.5,
                user_data: ref match_user_data,
            }))) if match_user_data == "some data" => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn rep_f64() {
        match encode_decode::<Rep<f64>>(Rep::Result(Workload::Single(LookupResult::Best(Match {
            cluster_id: 177,
            similarity: 0.5,
            user_data: 0.1,
        })))) {
            Rep::Result(Workload::Single(LookupResult::Best(Match {
                cluster_id: 177,
                similarity: 0.5,
                user_data: 0.1,
            }))) => (),
            other => panic!("bad result: {:?}", other),
        }
    }
}

#![feature(slice_bytes)]
extern crate byteorder;
extern crate rustc_serialize;

use std::fmt::Debug;

pub mod bin;
pub mod json;

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



extern crate byteorder;
extern crate rustc_serialize;

use std::fmt::Debug;

pub mod bin;
pub mod json;

#[derive(Debug)]
pub enum Trans<UD> where UD: Debug {
    Async(Req<UD>),
    Sync(Req<UD>),
}

#[derive(Debug)]
pub enum Req<UD> where UD: Debug {
    Init,
    Lookup(Workload<LookupTask<UD>>),
    Terminate,
}

#[derive(Debug)]
pub enum Workload<T> where T: Debug {
    Single(T),
    Many(Vec<T>),
}

#[derive(Debug)]
pub struct LookupTask<UD> where UD: Debug {
    pub text: String,
    pub result: LookupType,
    pub post_action: PostAction<UD>,
}

#[derive(Debug)]
pub enum LookupType { All, Best, BestOrMine }

#[derive(Debug)]
pub enum PostAction<UD> where UD: Debug {
    None,
    InsertNew { cond: InsertCond, assign: ClusterAssign, user_data: UD, },
}

#[derive(Debug)]
pub enum InsertCond {
    Always,
    BestSimLessThan(f64),
}

#[derive(Debug)]
pub struct ClusterAssign {
    pub cond: AssignCond,
    pub choice: ClusterChoice,
}

#[derive(Debug)]
pub enum AssignCond {
    Always,
    BestSimLessThan(f64),
}

#[derive(Debug)]
pub enum ClusterChoice {
    ServerChoice,
    ClientChoice(u64),
}

#[derive(Debug)]
pub enum Rep<UD> where UD: Debug {
    InitAck,
    Result(Workload<LookupResult<UD>>),
    TerminateAck,
    Unexpected(Req<UD>),
    TooBusy,
    WantCrash,
}

#[derive(Debug)]
pub enum LookupResult<UD> where UD: Debug {
    EmptySet,
    Best(Match<UD>),
    Neighbours(Workload<Match<UD>>),
    Error(String),
}

#[derive(Debug)]
pub struct Match<UD> where UD: Debug {
    pub cluster_id: u64,
    pub similarity: f64,
    pub user_data: UD,
}

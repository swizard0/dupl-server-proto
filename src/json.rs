use std::fmt::Debug;
use rustc_serialize::json::{Json, Object, ToJson};
use super::{
    Workload,
    Trans, Req, LookupTask, PostAction, InsertCond, ClusterAssign, AssignCond, ClusterChoice, LookupType,
    Rep, LookupResult, Match
};

pub fn req_to_json<UD>(trans: &Trans<UD>) -> Json where UD: Debug + ToJson { trans.to_json() }
pub fn rep_to_json<UD>(rep: &Rep<UD>) -> Json where UD: Debug + ToJson { rep.to_json() }

pub fn json_str_to_anything<T>(json_str: &str) -> Result<T, String> where T: Debug + FromJson {
    match Json::from_str(json_str) {
        Ok(ref json) => match <T as FromJson>::from_json(json) {
            Ok(value) =>
                Ok(value),
            Err(JsonDecodeError::MalformedObject(obj)) =>
                Err(format!("malformed json object: {}", obj)),
            Err(JsonDecodeError::UnexpectedToken(obj)) =>
                Err(format!("unexpected json token: {}", obj)),
        },
        Err(json_error) =>
            Err(format!("json parsing error: {}", json_error)),
    }
}

pub fn json_to_req<'a, UD>(json: &'a Json) -> Result<Req<UD>, JsonDecodeError<'a>> where UD: Debug + FromJson {
    <Req<UD> as FromJson>::from_json(json)
}

pub fn json_to_rep<'a, UD>(json: &'a Json) -> Result<Rep<UD>, JsonDecodeError<'a>> where UD: Debug + FromJson {
    <Rep<UD> as FromJson>::from_json(json)
}

impl ToJson for LookupType {
    fn to_json(&self) -> Json {
        match *self {
            LookupType::All => Json::String("all".to_string()),
            LookupType::Best => Json::String("best".to_string()),
            LookupType::BestOrMine => Json::String("best_or_mine".to_string()),
        }
    }
}

impl ToJson for AssignCond {
    fn to_json(&self) -> Json {
        match *self {
            AssignCond::Always =>
                Json::String("always".to_string()),
            AssignCond::BestSimLessThan(sim) => {
                let mut o = Object::new();
                o.insert("best_sim_less_than".to_string(), sim.to_json());
                Json::Object(o)
            },
        }
    }
}

impl ToJson for ClusterChoice {
    fn to_json(&self) -> Json {
        match *self {
            ClusterChoice::ServerChoice =>
                Json::String("server_choice".to_string()),
            ClusterChoice::ClientChoice(cluster_id) => {
                let mut o = Object::new();
                o.insert("client_choice".to_string(), cluster_id.to_json());
                Json::Object(o)
            },
        }
    }
}

impl ToJson for ClusterAssign {
    fn to_json(&self) -> Json {
        let mut o = Object::new();
        o.insert("cond".to_string(), self.cond.to_json());
        o.insert("choice".to_string(), self.choice.to_json());
        Json::Object(o)
    }
}

impl ToJson for InsertCond {
    fn to_json(&self) -> Json {
        match *self {
            InsertCond::Always =>
                Json::String("always".to_string()),
            InsertCond::BestSimLessThan(sim) => {
                let mut o = Object::new();
                o.insert("best_sim_less_than".to_string(), sim.to_json());
                Json::Object(o)
            },
        }
    }
}

impl<UD> ToJson for PostAction<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &PostAction::None =>
                Json::String("none".to_string()),
            &PostAction::InsertNew { cond: ref cond_value, assign: ref assign_value, user_data: ref user_data_value } => {
                let mut o = Object::new();
                o.insert("cond".to_string(), cond_value.to_json());
                o.insert("assign".to_string(), assign_value.to_json());
                o.insert("user_data".to_string(), user_data_value.to_json());
                Json::Object(o)
            },
        }
    }
}

impl<UD> ToJson for LookupTask<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        let mut o = Object::new();
        o.insert("text".to_string(), self.text.to_json());
        o.insert("result".to_string(), self.result.to_json());
        o.insert("post_action".to_string(), self.post_action.to_json());
        Json::Object(o)
    }
}

impl<T> ToJson for Workload<T> where T: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &Workload::Single(ref value) => value.to_json(),
            &Workload::Many(ref values) => values.to_json(),
        }
    }
}

impl<UD> ToJson for Req<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &Req::Init =>
                Json::String("init".to_string()),
            &Req::Lookup(ref workload) => {
                let mut o = Object::new();
                o.insert("lookup".to_string(), workload.to_json());
                Json::Object(o)
            },
            &Req::Terminate =>
                Json::String("terminate".to_string()),
        }
    }
}

impl<UD> ToJson for Trans<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &Trans::Async(ref req) => {
                let mut o = Object::new();
                o.insert("async".to_string(), req.to_json());
                Json::Object(o)
            },
            &Trans::Sync(ref req) => {
                let mut o = Object::new();
                o.insert("sync".to_string(), req.to_json());
                Json::Object(o)
            },
        }
    }
}

impl<UD> ToJson for Match<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        let mut o = Object::new();
        o.insert("cluster_id".to_string(), self.cluster_id.to_json());
        o.insert("similarity".to_string(), self.similarity.to_json());
        o.insert("user_data".to_string(), self.user_data.to_json());
        Json::Object(o)
    }
}

impl<UD> ToJson for LookupResult<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &LookupResult::EmptySet => Json::Null,
            &LookupResult::Best(ref m) => {
                let mut o = Object::new();
                o.insert("best".to_string(), m.to_json());
                Json::Object(o)
            },
            &LookupResult::Neighbours(ref neighbours) => {
                let mut o = Object::new();
                o.insert("neighbours".to_string(), neighbours.to_json());
                Json::Object(o)
            },
            &LookupResult::Error(ref message) => {
                let mut o = Object::new();
                o.insert("error".to_string(), message.to_json());
                Json::Object(o)
            }
        }
    }
}

impl<UD> ToJson for Rep<UD> where UD: Debug + ToJson {
    fn to_json(&self) -> Json {
        match self {
            &Rep::InitAck => Json::String("init_ack".to_string()),
            &Rep::Result(ref result) => {
                let mut o = Object::new();
                o.insert("result".to_string(), result.to_json());
                Json::Object(o)
            },
            &Rep::TerminateAck => Json::String("terminate_ack".to_string()),
            &Rep::Unexpected(ref req) => {
                let mut o = Object::new();
                o.insert("unexpected".to_string(), req.to_json());
                Json::Object(o)
            },
            &Rep::TooBusy => Json::String("too_busy".to_string()),
            &Rep::WantCrash => Json::String("want_crash".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum JsonDecodeError<'a> {
    UnexpectedToken(&'a Json),
    MalformedObject(&'a Json),
}

pub trait FromJson: Sized {
    fn from_json<'a>(json: &'a Json) -> Result<Self, JsonDecodeError<'a>>;
}

impl FromJson for String {
    fn from_json<'a>(json: &'a Json) -> Result<String, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref value) => Ok(value.clone()),
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl FromJson for LookupType {
    fn from_json<'a>(json: &'a Json) -> Result<LookupType, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "all" => Ok(LookupType::All),
            &Json::String(ref token) if *token == "best" => Ok(LookupType::Best),
            &Json::String(ref token) if *token == "best_or_mine" => Ok(LookupType::BestOrMine),
            token => Err(JsonDecodeError::UnexpectedToken(token)),
        }
    }
}

impl FromJson for InsertCond {
    fn from_json<'a>(json: &'a Json) -> Result<InsertCond, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "always" =>
                Ok(InsertCond::Always),
            &Json::Object(ref obj) => match obj.get("best_sim_less_than") {
                Some(&Json::F64(sim)) => Ok(InsertCond::BestSimLessThan(sim)),
                _ => Err(JsonDecodeError::MalformedObject(json)),
            },
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl FromJson for AssignCond {
    fn from_json<'a>(json: &'a Json) -> Result<AssignCond, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "always" =>
                Ok(AssignCond::Always),
            &Json::Object(ref obj) => match obj.get("best_sim_less_than") {
                Some(&Json::F64(sim)) => Ok(AssignCond::BestSimLessThan(sim)),
                _ => Err(JsonDecodeError::MalformedObject(json)),
            },
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl FromJson for ClusterChoice {
    fn from_json<'a>(json: &'a Json) -> Result<ClusterChoice, JsonDecodeError<'a>> {
        let decoded = match json {
            &Json::String(ref token) if *token == "server_choice" =>
                Some(ClusterChoice::ServerChoice),
            &Json::Object(ref obj) => match obj.get("client_choice") {
                Some(&Json::U64(cluster_id)) => Some(ClusterChoice::ClientChoice(cluster_id)),
                _ => None,
            },
            _ => None,
        };

        match decoded { Some(value) => Ok(value), None => Err(JsonDecodeError::UnexpectedToken(json)), }
    }
}

impl FromJson for ClusterAssign {
    fn from_json<'a>(json: &'a Json) -> Result<ClusterAssign, JsonDecodeError<'a>> {
        match json {
            &Json::Object(ref obj) => match (obj.get("cond"), obj.get("choice")) {
                (Some(cond), Some(choice)) =>
                    Ok(ClusterAssign {
                        cond: try!(<AssignCond as FromJson>::from_json(cond)),
                        choice: try!(<ClusterChoice as FromJson>::from_json(choice)),
                    }),
                _ => Err(JsonDecodeError::MalformedObject(json)),
            },
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for PostAction<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<PostAction<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "none" =>
                Ok(PostAction::None),
            &Json::Object(ref obj) => match (obj.get("cond"), obj.get("assign"), obj.get("user_data")) {
                (Some(cond), Some(assign), Some(user_data)) =>
                    Ok(PostAction::InsertNew {
                        cond: try!(<InsertCond as FromJson>::from_json(cond)),
                        assign: try!(<ClusterAssign as FromJson>::from_json(assign)),
                        user_data: try!(<UD as FromJson>::from_json(user_data)),
                    }),
                _ => Err(JsonDecodeError::MalformedObject(json)),
            },
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for LookupTask<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<LookupTask<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::Object(ref obj) => match (obj.get("text"), obj.get("result"), obj.get("post_action")) {
                (Some(text), Some(result), Some(post_action)) =>
                    Ok(LookupTask {
                        text: try!(<String as FromJson>::from_json(text)),
                        result: try!(<LookupType as FromJson>::from_json(result)),
                        post_action: try!(<PostAction<UD> as FromJson>::from_json(post_action)),
                    }),
                _ => Err(JsonDecodeError::MalformedObject(json)),
            },
            _ => Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<T> FromJson for Workload<T> where T: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<Workload<T>, JsonDecodeError<'a>> {
        match json {
            &Json::Array(ref obj) =>
                Ok(Workload::Many(try!(obj.iter().map(|o| <T as FromJson>::from_json(o)).collect()))),
            obj =>
                Ok(Workload::Single(try!(<T as FromJson>::from_json(obj)))),
        }
    }
}

impl<UD> FromJson for Req<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<Req<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "init" =>
                Ok(Req::Init),
            &Json::String(ref token) if *token == "terminate" =>
                Ok(Req::Terminate),
            &Json::Object(ref obj) => match obj.get("lookup") {
                Some(workload) =>
                    Ok(Req::Lookup(try!(<Workload<LookupTask<UD>> as FromJson>::from_json(workload)))),
                _ =>
                    Err(JsonDecodeError::MalformedObject(json)),
            },
            _ =>
                Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for Trans<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<Trans<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::Object(ref obj) => match (obj.get("async"), obj.get("sync")) {
                (Some(ref req), None) =>
                    Ok(Trans::Async(try!(<Req<UD> as FromJson>::from_json(req)))),
                (None, Some(ref req)) =>
                    Ok(Trans::Sync(try!(<Req<UD> as FromJson>::from_json(req)))),
                _ =>
                    Err(JsonDecodeError::MalformedObject(json)),
            },
            _ =>
                Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for Match<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<Match<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::Object(ref obj) => match (obj.get("cluster_id"), obj.get("similarity"), obj.get("user_data")) {
                (Some(&Json::U64(cluster_id)), Some(&Json::F64(similarity)), Some(user_data)) =>
                    Ok(Match {
                        cluster_id: cluster_id,
                        similarity: similarity,
                        user_data: try!(<UD as FromJson>::from_json(user_data)),
                    }),
                _ =>
                    Err(JsonDecodeError::MalformedObject(json)),
            },
            _ =>
                Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for LookupResult<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<LookupResult<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::Null => Ok(LookupResult::EmptySet),
            &Json::Object(ref obj) => match (obj.get("best"), obj.get("neighbours"), obj.get("error")) {
                (Some(result), None, None) =>
                    Ok(LookupResult::Best(try!(<Match<UD> as FromJson>::from_json(result)))),
                (None, Some(workload), None) =>
                    Ok(LookupResult::Neighbours(try!(<Workload<Match<UD>> as FromJson>::from_json(workload)))),
                (None, None, Some(message)) =>
                    Ok(LookupResult::Error(try!(<String as FromJson>::from_json(message)))),
                _ =>
                    Err(JsonDecodeError::MalformedObject(json)),
            },
            _ =>
                Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

impl<UD> FromJson for Rep<UD> where UD: Debug + FromJson {
    fn from_json<'a>(json: &'a Json) -> Result<Rep<UD>, JsonDecodeError<'a>> {
        match json {
            &Json::String(ref token) if *token == "init_ack" =>
                Ok(Rep::InitAck),
            &Json::String(ref token) if *token == "terminate_ack" =>
                Ok(Rep::TerminateAck),
            &Json::String(ref token) if *token == "too_busy" =>
                Ok(Rep::TooBusy),
            &Json::String(ref token) if *token == "want_crash" =>
                Ok(Rep::WantCrash),
            &Json::Object(ref obj) => match (obj.get("result"), obj.get("unexpected")) {
                (Some(workload), None) =>
                    Ok(Rep::Result(try!(<Workload<LookupResult<UD>> as FromJson>::from_json(workload)))),
                (None, Some(req)) =>
                    Ok(Rep::Unexpected(try!(<Req<UD> as FromJson>::from_json(req)))),
                _ =>
                    Err(JsonDecodeError::MalformedObject(json)),
            },
            _ =>
                Err(JsonDecodeError::UnexpectedToken(json)),
        }
    }
}

#[cfg(test)]
mod test {
    use rustc_serialize::json::{ToJson};
    use super::{FromJson};
    use super::super::{
        Workload,
        Trans, Req, LookupTask, PostAction, InsertCond, ClusterAssign, AssignCond, ClusterChoice, LookupType,
        Rep, LookupResult, Match
    };

    fn encode_decode<T>(value: T) -> T where T: ToJson + FromJson {
        let json = value.to_json();
        <T as FromJson>::from_json(&json).unwrap()
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
        match encode_decode_req(Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
            text: "hello world".to_owned(),
            result: LookupType::All,
            post_action: PostAction::None,
        })))) {
            Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
                text: ref lookup_text,
                result: LookupType::All,
                post_action: PostAction::None,
            }))) if lookup_text == "hello world" => (),
            other => panic!("bad result: {:?}", other),
        }
    }

    #[test]
    fn req_02() {
        match encode_decode_req(Trans::Async(Req::Lookup(Workload::Single(LookupTask {
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
            Trans::Async(Req::Lookup(Workload::Single(LookupTask {
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
        match encode_decode_req(Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
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
            Trans::Sync(Req::Lookup(Workload::Single(LookupTask {
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
}

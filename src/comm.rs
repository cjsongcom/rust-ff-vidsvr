use crate::error::Error;
use serde::{Deserialize, Serialize};
use serde_json;
use std::net::IpAddr;
use std::result::Result;
use std::time::Duration;
use uuid::Uuid;
//use chrono::{DateTime, Utc};

pub type EchoArc<T> = std::sync::Arc<T>;

// pub type EchoSyncMutex<T>  = std::sync::Mutex<T>;
// pub type EchoSyncRwLock<T> = std::sync::RwLock<T>;

// need to call .await in async function
pub type EchoAsyncMutex<T> = tokio::sync::Mutex<T>;
pub type EchoAsyncRwLock<T> = tokio::sync::RwLock<T>;

//
// time
//

pub type EchoTimeInstant = std::time::Instant;
pub type EchoTimeDuration = std::time::Duration;

#[macro_export]
macro_rules! ECHO_TIME_DURATION_SEC {
    ($sec:expr) => {{
        // EchoTimeDuration=std::time::Duration
        std::time::Duration::new($sec, 0)
    }};
}

#[macro_export]
macro_rules! ECHO_TIME_DURATION_MS {
    ($ms:expr) => {{
        std::time::Duration::from_millis($ms)
    }};
}

pub type EchoUtc = chrono::Utc;

pub struct EchoEpoch(i64);
pub struct EchoEpochMS(i64);

// number of seconds from 1970.1.1
pub fn get_echo_epoch() -> EchoEpoch {
    EchoEpoch(chrono::Utc::now().timestamp())
}

// epoch as milliseconds (second to milliseconds)
pub fn get_echo_epoch_as_ms() -> EchoEpochMS {
    EchoEpochMS(chrono::Utc::now().timestamp_millis())
}

// MIN: 600   = 60*10   (600, 10 min)
// MAX: 15000 = 60*60*4 (14400, 4 hour) + (600, 10 min)
pub const ECHO_PUBLISH_MIN_DURATION: Duration = Duration::new(60 * 10, 0);

//let in_ms = since_the_epoch.as_secs() * 1000 +
//since_the_epoch.subsec_nanos() as u64 / 1_000_000;

//
// uuid
//

pub type EchoUUID = Uuid;
pub fn EchoUUID_new() -> EchoUUID {
    Uuid::new_v4()
}

//
// json support
//
pub type PrmJsonValue = serde_json::Value;
pub type PrmJson = serde_json::Value;

pub type PrmJsonMap<K, V> = serde_json::Map<K, V>;

pub type PrmString = String;

pub type EchoJoinHandle<T, E> = tokio::task::JoinHandle<Result<T, E>>;

//
// result
//

pub type EchoRst<R, Error> = Result<R, Error>;
pub type EchoAsyncRst = Result<(), Error>;

//
// Echo Message
//

#[macro_export]
macro_rules! ECHO_ASYNC_SLEEP_MS {
    ($x:expr) => {{
        // EchoTimeDuration=std::time::Duration
        tokio::time::sleep(std::time::Duration::from_millis($x)).await
    }};
}

//
// Echo OneshotChannel
//

pub type EchoTryRecvError = tokio::sync::mpsc::error::TryRecvError;
pub type EchoOCTryRecvError = tokio::sync::oneshot::error::TryRecvError;

pub type EchoOCPayload<R, E> = Result<R, E>;

pub type EchoOCError = Error;

pub type EchoOCResponder<T> = tokio::sync::oneshot::Sender<T>;
pub type EchoOCResponderPrmJson = tokio::sync::oneshot::Sender<Result<PrmJson, Error>>;

pub type EchoOCRst<T> = tokio::sync::oneshot::Receiver<T>;
pub type EchoOCRstPrmJson = tokio::sync::oneshot::Receiver<Result<PrmJson, Error>>;

#[macro_export]
macro_rules! EchoOCCreatePrmJson {
    () => {
        || -> (EchoOCResponder<PrmJson, Error>, EchoOCRst<PrmJson, Error>) {
            tokio::sync::oneshot::channel()
        }()
    };
}

//
// echo publish oper mode
//

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EchoPublishOperMode {
    Audio,
    Video,
    AudioVideo,
}

// #[macro_export]
// macro_rules! EchoOCCreate {
//     () => (||->(EchoOCResponder<(),Error>,
//                 EchoOCRst<(),Error>){
//                     tokio::sync::oneshot::channel()
//                 }());
// }

//
// async
//

pub type EchoStrAry = Vec<String>;
pub type EchoVecAry<T> = Vec<Vec<T>>;

//
// async file read/write
//

pub type EchoAFile = tokio::fs::File;

//
// etc
//

pub type EchoPublishPort = u16;

pub type EchoPublishIp = Option<IpAddr>;

pub type EchoIpStr = String;

pub type EchoPortStr = String;
pub type EchoPort = u16;

// where the Err(err) branch expands to
// > an early return Err(From::from(err)),
// > and the Ok(ok) branch expands to an ok expression.
//let err_json = From::from(rst);

#[macro_export]
macro_rules! EchoPublishIpStr {
    // match like arm for macro
    //ty=type, item=function,struct,module,etc
    ($x:expr) => {{
        if let Option::<IpAddr>::Some(ip) = $x {
            ip.to_string()
        } else {
            String::from("0.0.0.0")
        }
    }};
}

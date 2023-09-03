// runner::mod.rs
pub mod driver;
pub mod error;
pub mod ffmpeg;
pub mod manager;
pub mod message;
pub mod worker;
use crate::service::vsvr::message::VSvrServMsgSend;
use crate::{comm::EchoJoinHandle, config::Config};
use crate::{
    comm::{EchoAsyncRwLock, EchoIpStr, EchoPublishPort},
    message::ServMsgSend,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

//
// runner
//

pub type RunnerError = error::RunnerError;

// runner process - spawned child process
pub type RunnerProcCmd = tokio::process::Command;
pub type RunnerProcChild = tokio::process::Child;
pub type RunnerProcChildStdErr = tokio::process::ChildStderr;

pub const AWAIT_TERMINATE_TIMEOUT_SEC: u64 = 10000;
pub const POLL_PERIOD_TERMINATE_MS: u64 = 1000;
pub const TERMINATE_TIMEOUT_MS: u64 = 2000;

use ffmpeg::RunnerFFMpeg;

use message::{
    RecvWorkerManagerMsgRecv, RecvWorkerManagerMsgSend, RecvWorkerMsgRecv, RecvWorkerMsgSend,
};

//
// driver
//

#[derive(Debug)]
pub enum DriverType {
    FFMPEG,
}

// pub trait Driver {
//     fn tick(&self) -> EchoAsyncRst;
//     fn begin(&self) -> EchoAsyncRst;
//     fn end(&self) -> EchoAsyncRst;
// }

#[derive(strum_macros::Display, Debug, Clone, PartialEq)]
enum DriverStatus {
    Init,
    Creating,
    ReceivingPublishStream, // ffmpeg is spawned and await connection or ffmpeg is receiving publish stream
    Ending,
    End,
    Restarting,
}

pub struct Driver {
    status: DriverStatus,
    runner_ffmpeg: Option<RunnerFFMpeg>,
}

#[derive(Debug, PartialEq)]
pub enum DriverRstOk {
    Ok,
    // bool=do_respawn
    Finished(bool),
}

pub type DriverRst = Result<DriverRstOk, RunnerError>;

//
// runner-worker
//

//
// Params
//

pub enum RecvWorkerType {
    FFMpegRTMP,
}

pub struct RecvWorkerSpawnPrm {}

//
// RecvWorkerManager
//

#[derive(Debug, Clone, PartialEq)]
pub enum RecvWorkerManagerStatus {
    Creating,
    Running,
    Destroying,
}

//
// RecvWorkerHandle
//

pub struct RecvWorkerHandle {
    //worker_data: Arc<RecvWorkerData>,
    //worker: Arc<EchoSyncRwLock<RecvWorker>>,
    //runner : JoinHandle<()>,
    uuid: String,
    worker_uuid: String,

    //worker_joinhandle: JoinHandle<Result<(), anyhow::Error>>,
    worker_joinhandle: Option<EchoJoinHandle<(), RunnerError>>,

    worker_msg_send: RecvWorkerMsgSend,

    app_name: String,
    sess_key: String,

    publish_ip: String,
    publish_port: u16,
}

//
// RecvWorker Manager
//

pub struct RecvWorkerManager {
    force_exit: bool,

    config: Config,
    status: RecvWorkerManagerStatus,

    // message queue/sender for recv-worker-manager
    msg_received: RecvWorkerManagerMsgRecv,
    msg_send: RecvWorkerManagerMsgSend,

    main_serv_msg_send: ServMsgSend,
    vsvr_serv_msg_send: VSvrServMsgSend,

    //worker_handles: HashMap<EchoUUID, Arc<RwLock<RecvWorkerHandle>>>,
    worker_handles: HashMap<String, Arc<EchoAsyncRwLock<RecvWorkerHandle>>>,

    is_destroyed: bool,

    publish_ip: EchoIpStr,
    //avail_publish_ports: Arc<RwLock<VecDeque<EchoPublishPort>>>,
    avail_publish_ports: Arc<EchoAsyncRwLock<VecDeque<EchoPublishPort>>>,
}

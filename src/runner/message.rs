use crate::comm::*;
use crate::service::api::reqres::publish::req::ReqPublishV3;
use anyhow::Result;
use strum::Display;
use tokio::sync::mpsc;
use PrmJsonValue;

use super::RunnerError;

//
// RecvWorkerManager Message
//

#[derive(Debug, Clone)]
pub enum RecvWorkerType {
    RecvWorkerTypeFFMpeg,
    RecvWorkerTypeRustRTMP,
}

//pub type RecvWorkerMsgParamJson = PrmJsonValue;

#[derive(Debug, Clone, Copy, Display)]
pub enum TerminateRecvWorkerPrmType {
    AppName,
    AppNameSessKey,
    WorkerUUID,
}

#[derive(Debug)]
pub enum RecvWorkerManagerMsg {
    SpawnRecvWorker(SpawnRecvWorkerMsgOCResponder, ReqPublishV3),

    // (responder, PrmType, AppName, SessionKey)
    TerminateRecvWorker(
        TerminateRecvWorkerMsgOCResponder,
        TerminateRecvWorkerPrmType,
        String,
        String,
    ),

    // (worker_uuid, app_name, sess_key)
    NotifyRecvWorkerIsExiting(String, String, String),
}
// QueryRecvWorkerManagerInstance,

pub type RecvWorkerManagerMsgSend = mpsc::UnboundedSender<RecvWorkerManagerMsg>;
pub type RecvWorkerManagerMsgRecv = mpsc::UnboundedReceiver<RecvWorkerManagerMsg>;

// Message: SpawnRecvWorker

// Result::<(publish_ip, publish_port, worker_uuid),Err>
pub type SpawnRecvWorkerMsgRstType = Result<(String, u16, String), RunnerError>;

pub type SpawnRecvWorkerMsgOCResponder = EchoOCResponder<SpawnRecvWorkerMsgRstType>;
pub type SpawnRecvWorkerMsgOCRst = EchoOCRst<SpawnRecvWorkerMsgRstType>;

// pub fn create_spawn_recv_worker_msg_oc_sender()->(
//     SpawnRecvWorkerMsgOCResponder,
//     SpawnRecvWorkerMsgOCRst) {
//     return tokio::sync::oneshot::channel::<SpawnRecvWorkerMsgRstType>();
// }

// Message: TerminateRecvWorker
pub type TerminateRecvWorkerMsgRstType = Result<PrmJson, RunnerError>;
pub type TerminateRecvWorkerMsgOCResponder = EchoOCResponder<TerminateRecvWorkerMsgRstType>;
pub type TerminateRecvWorkerMsgOCRst = EchoOCRst<TerminateRecvWorkerMsgRstType>;

///////////////////////////////////////////////////////////////////////////////
// RecvWorker Message
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RecvWorkerMsg {
    StartRecvWorker(StartRecvWorkerMsgOCResponder, PrmJsonValue),
    FinishRecvWorker(FinishRecvWorkerMsgOCResponder, PrmJsonValue),

    QueryRecvWorkerState,
}

//QueryManagerInstance,
//CmdSpawnReceiveRunner,
//CmdTerminateWorker,
//NotifyManagerIsDestroying,

pub type RecvWorkerMsgSend = tokio::sync::mpsc::UnboundedSender<RecvWorkerMsg>;
pub type RecvWorkerMsgRecv = tokio::sync::mpsc::UnboundedReceiver<RecvWorkerMsg>;

// StartRecvWorker
pub type StartRecvWorkerMsgRstType = Result<RecvWorkerMsgSend, RunnerError>;
pub type StartRecvWorkerMsgOCResponder = EchoOCResponder<StartRecvWorkerMsgRstType>;
pub type StartRecvWorkerMsgOCRst = EchoOCRst<StartRecvWorkerMsgRstType>;

// FinishRecvWorker
pub type FinishRecvWorkerMsgRstType = Result<PrmJson, RunnerError>;
pub type FinishRecvWorkerMsgOCResponder = EchoOCResponder<FinishRecvWorkerMsgRstType>;
pub type FinishRecvWorkerMsgOCRst = EchoOCRst<FinishRecvWorkerMsgRstType>;

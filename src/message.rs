use crate::comm::*;
use crate::error::Error;
use crate::runner::message::TerminateRecvWorkerPrmType;
use crate::service::api::reqres::publish::{req::ReqPublishV3, res::ResPublishV3};
use crate::service::vsvr::message::VSvrServMsgSend;
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};

//
// Service Channel Message
//

//pub type ServMsgError = Error;
pub type ServMsgReqJson = PrmJson;
pub type ServMsgResJson = PrmJson;

pub type ServMsgResponseJson<T> = oneshot::Sender<Result<T, Error>>;

//pub type SpawnRecvWorkerMsgRst = PrmJsonValue;

#[derive(Debug)]
pub enum ServMsg {
    SpawnRecvWorker(ServMsgResponseJson<ResPublishV3>, ReqPublishV3),

    // (rst_json, 'TerminateRecvWorkerPrmType::AppName' , app_name, sess_key)
    TerminateRecvWorker(
        ServMsgResponseJson<PrmJson>,
        TerminateRecvWorkerPrmType,
        String,
        String,
    ),

    GetVSvrServSender(GetVSvrServSenderMsgOCResponder),
}

pub type ServMsgSend = mpsc::UnboundedSender<ServMsg>;
pub type ServMsgRecv = mpsc::UnboundedReceiver<ServMsg>;

// SpawnRecvWorker
pub type SpawnRecvWorkerMsgRstType = Result<VSvrServMsgSend, Error>;
pub type SpawnRecvWorkerMsgOCResponder = EchoOCResponder<SpawnRecvWorkerMsgRstType>;
pub type SpawnRecvWorkerMsgOCRst = EchoOCRst<SpawnRecvWorkerMsgRstType>;

// TerminateRecvWorker
pub type TerminateRecvWorkerMsgRstType = Result<PrmJson, Error>;
pub type TerminateRecvWorkerMsgOCResponder = EchoOCResponder<TerminateRecvWorkerMsgRstType>;
pub type TerminateRecvWorkerMsgOCRst = EchoOCRst<TerminateRecvWorkerMsgRstType>;

// GetVSvrServSender
pub type GetVSvrServSenderMsgRstType = Result<VSvrServMsgSend, Error>;
pub type GetVSvrServSenderMsgOCResponder = EchoOCResponder<GetVSvrServSenderMsgRstType>;
pub type GetVSvrServSenderMsgOCRst = EchoOCRst<GetVSvrServSenderMsgRstType>;

pub fn create_get_vsvr_serv_sender_msg_oc_sender(
) -> (GetVSvrServSenderMsgOCResponder, GetVSvrServSenderMsgOCRst) {
    return tokio::sync::oneshot::channel::<GetVSvrServSenderMsgRstType>();
}

use super::RunnerFFMError;
use crate::comm::{EchoOCResponder, EchoOCRst};
use crate::runner::ffmpeg::RunnerFFMpegInnerExitRst;
use anyhow::Result;

//
// RunnerFFMpegMsg message
//

//pub type FFMpegInnerMsgResponseJson = tokio::sync::oneshot::Sender<Result<PrmJsonValue, Error>>;

#[derive(strum_macros::Display, Debug, Clone)]
pub enum RunnerFFMpegMsg {
    Spawning, // spawning ffmpeg process
    FailedSpawn,

    BeginRunning, // ffmpeg is spawned, and awaiting publish stream connection
    //ReceivingStream, // receiving publish stream

    // do_respawn  ;;exit_code, stdout, stderr
    Finished(bool),
}

pub type RunnerFFMpegMsgSend = tokio::sync::mpsc::UnboundedSender<RunnerFFMpegMsg>;
pub type RunnerFFMpegMsgRecv = tokio::sync::mpsc::UnboundedReceiver<RunnerFFMpegMsg>;

//
// FFMpegInner message
//

#[derive(strum_macros::Display, Debug)]
pub enum RunnerFFMpegInnerMsg {
    Terminate(TerminateMsgOCResponder),
}

pub type RunnerFFMpegInnerMsgSend = tokio::sync::mpsc::UnboundedSender<RunnerFFMpegInnerMsg>;
pub type RunnerFFMpegInnerMsgRecv = tokio::sync::mpsc::UnboundedReceiver<RunnerFFMpegInnerMsg>;

// FFMpegInnerMsg::Terminate
// (terminated, exit result for ffmpeg process)
pub type TerminateMsgRstType = Result<RunnerFFMpegInnerExitRst, RunnerFFMError>;
pub type TerminateMsgOCResponder = EchoOCResponder<TerminateMsgRstType>;
pub type TerminateMsgOCRst = EchoOCRst<TerminateMsgRstType>;

pub fn create_terminate_msg_oc_sender() -> (TerminateMsgOCResponder, TerminateMsgOCRst) {
    tokio::sync::oneshot::channel::<TerminateMsgRstType>()
}

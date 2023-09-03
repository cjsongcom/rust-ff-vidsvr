// RunnerError, runner::error
use crate::PrmJsonValue;
use thiserror::Error;

// Error=thiserror_impl proc_macro Error
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("error in runner operation={0}")]
    RunnerOperErr(String),

    #[error("error in runner msg channel={0},m={1},f={2},e={3}")]
    RunnerMsgChanErr(&'static str, &'static str, &'static str, String),

    #[error("{0}")]
    RecvDriverErr(String),

    #[error("invalid runner driver type={0}")]
    UnSupportedRunnerDriverType(String),

    #[error("runner driver operation error={0}")]
    RunnerDriverErr(String),

    #[error("failed to message send to channel: {0}")]
    MsgChanErrSendFail(String),

    #[error("internal system error: {0}")]
    InternalError(String),

    #[error("unsupported media receiver type, {0}")]
    UnSupportedMediaReciverType(String),

    #[error("receive worker manager operation error: {0}")]
    RecvWorkManagerOperErr(String),

    #[error("receive worker operation error: {0}")]
    RecvWorkerOperErr(String),

    #[error("receive worker handle error: {0}")]
    RecvWorkerHandleErr(String),

    #[error("failed to create receive worker: {0}")]
    FailedToCreateRecvWorker(String),

    #[error("failed to terminate receive worker: {0}")]
    FailedToTerminateRecvWorker(String),

    #[error("failed to begin receiver worker: {0}")]
    FailedToBeginRecvWorker(String),

    #[error("message channel is closed: {0}")]
    MsgChanErrChannelClosed(String), // (err_msg)

    #[error("failed to send message to manager: {0}")]
    SendingMsgToWorkerManagerFailed(String),

    #[error("manager status is invalid: {0}")]
    InvalidManagerStatus(PrmJsonValue),

    #[error("manager already created")]
    ManagerAlreadyCreated,
}

//pub type RunnerError = self::Error;

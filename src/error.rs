use thiserror::Error;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum Error {
    // for tokio::fs::metadata, create_dir_all
    #[error("{0}")]
    IoError(String),

    #[error("{0}")]
    InvalidPath(String),

    #[error("no available publish port")]
    PublishPortNotAvailable,

    #[error("parsing is failed, err={0}")]
    ParsingFailed(String),

    #[error("invalid calling parameter, {0}")]
    InvalidCallingParameter(String),

    #[error("invalid config parameter, {0}")]
    InvalidConfigParameter(String),

    #[error("message channel is closed: {0}")]
    MsgChanErrChannelClosed(String), // (err_msg)

    #[error("failed to message send to channel: {0}")]
    MsgChanErrSendFail(String),

    #[error("failed to message receive from channel: {0}")]
    MsgChanErrRecvFail(String),

    #[error("{0}")]
    MsgChanRstErrJson(String),
}

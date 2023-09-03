// RunnerFFMError, runner::ffmpeg::error
use thiserror::Error;

// Error=thiserror_impl proc_macro Error
#[derive(Error, Debug, Clone)]
pub enum RunnerFFMError {
    #[error("error in runnerffmpeg msg channel: {0},m={1},f={2},e={3}")]
    MsgChanErr(&'static str, &'static str, &'static str, String),

    #[error("error in runnerffmpeg operation: {0}")]
    OperErr(String),

    #[error("invalid config parameter: {0}")]
    InvalidConfigParameter(String),

    #[error("error in file operation: {0}")]
    FileOperErr(String),

    // (error msg, process is terminated, exit_rst
    //#[error("error occured on ffmpeginner exiting: {0}")]
    //FFMpegInnerErrOnExit(String, bool, Option<RunnerFFMpegInnerExitRst>),
    #[error("error on spawn ffmpeg={0}")]
    FFMpegSpawnErr(String),

    #[error("error on terminate ffmpeg={0}")]
    FFMpegTermErr(String),

    #[error("termination timeoutted")]
    FFMpegTermTimeout,

    #[error("error in runnerffmpeg inner operation: {0}")]
    InnerOperErr(String),
}

//pub type RunnerFFMError = self::Error;

// #[error("unsupported media type: {0}")]
// UnSupportedMediaType(String),

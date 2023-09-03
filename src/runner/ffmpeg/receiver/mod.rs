use super::RunnerFFMpeg;
use crate::runner::ffmpeg::RunnerFFMError;

pub mod rtmp;

pub trait IFFMReceiver {
    fn prepare_spawn(&self, runner_ctx: &RunnerFFMpeg) -> Result<(), RunnerFFMError>;
}

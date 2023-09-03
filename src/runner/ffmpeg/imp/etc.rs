use crate::runner::ffmpeg::*;
use crate::runner::{DriverRst, DriverRstOk};

impl RunnerFFMpeg {
    // protected

    pub(super) async fn flush_ffmpeg_ts(&mut self) -> DriverRst {
        log::debug!("[RunnerFFMpeg][flush_ffmpeg_ts]");

        Ok(DriverRstOk::Ok)
    }
}

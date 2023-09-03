use crate::runner::ffmpeg::*;
use crate::runner::*;

impl RunnerFFMpeg {
    pub(crate) async fn reset(&mut self) -> DriverRst {
        log::debug!("[RunnerFFMpeg][reset]");

        if self.inner_handle.is_some() {
            let em = format!("[RunnerFFMpeg][reset] self.inner_handle must be none..");
            log::debug!("{}", em);

            panic!("{}", em.as_str());
        }

        Ok(DriverRstOk::Ok)
    }
}

use crate::comm::{EchoOCTryRecvError, EchoTimeInstant};
use crate::runner::ffmpeg::message::*;
use crate::runner::ffmpeg::*;
use crate::runner::*;
use crate::ECHO_ASYNC_SLEEP_MS;
use crate::ECHO_TIME_DURATION_SEC;

impl RunnerFFMpeg {
    pub(crate) async fn end(&mut self) -> DriverRst {
        log::debug!("[RunnerFFMpeg::end] ");

        let mut inner_is_exited = false;
        let mut timeoutted_terminate = false;

        if self.inner_handle.is_some() {
            // Option<JoinHandle> to Option<&JoinHandle>
            let handle = self.inner_handle.as_ref().unwrap();

            if handle.is_finished() {
                inner_is_exited = true;
            } else {
                log::debug!("[RunnerFFMpeg::end] FFMpegInner is still running, sending msg({}) to FFMpegInner ..",
                "FFMpegInnerMsg::Terminate");

                let (terminate_responder, mut terminate_rst) = tokio::sync::oneshot::channel();

                let send_rst = self
                    .inner_msg_send
                    .as_ref()
                    .unwrap()
                    .send(RunnerFFMpegInnerMsg::Terminate(terminate_responder));

                if let Err(e) = send_rst {
                    // if channel is closed, FFMpegInner may be already exited

                    if handle.is_finished() {
                        inner_is_exited = true;
                    }

                    log::debug!("[RunnerFFMpeg::end] channel is closed, self.inner_msg_send.send({}), e={}, inner_is_exited={}",
                    "FFMpegInnerMsg::Terminate", e.to_string(), inner_is_exited);
                } else {
                    log::debug!(
                        "[RunnerFFMpeg::end] awaiting reply msg({}) from FFMpegInner, ..",
                        "FFMpegInnerMsg::Terminate"
                    );

                    // can't use terminate_rst.blocking_recv(), in asynchronous tasks(async fn)
                    //match terminate_rst.blocking_recv() ;   // terminate_rst.try_recv();
                    let expire = EchoTimeInstant::now()
                        + ECHO_TIME_DURATION_SEC!(AWAIT_TERMINATE_TIMEOUT_SEC);

                    loop {
                        //  give chance to FFMpegInner to process 'TERMINATE' message
                        ECHO_ASYNC_SLEEP_MS!(POLL_PERIOD_TERMINATE_MS);

                        let rst = terminate_rst.try_recv();

                        match rst {
                            Ok(r) => {
                                if let Err(e) = r.as_ref() {
                                    log::error!("[RunnerFFMpeg::end] received error exit result for msg({}) from FFMpegInner, e={}",                                    
                                    "FFMpegInnerMsg::Terminate", e.to_string());

                                    inner_is_exited = false;
                                } else {
                                    let r = r.unwrap();

                                    // log::debug!("[RunnerFFMpeg::end] received exit result for msg({}) from FFMpegInner, exit_code={}, exit_msg={} ..",
                                    // "FFMpegInnerMsg::Terminate", r.exit_code, r.message );

                                    log::debug!("[RunnerFFMpeg::end] received exit result for msg({}) from FFMpegInner..",
                                    "FFMpegInnerMsg::Terminate");

                                    inner_is_exited = true;
                                }

                                break;
                            }
                            Err(e) => match e {
                                EchoOCTryRecvError::Closed => {
                                    // if channel is closed, FFMpegInner may be already exited
                                    log::error!("[RunnerFFMpeg::end] recv channel is closed..for msg({}) from FFMpegInner, ..",
                                    "FFMpegInnerMsg::Terminate");

                                    inner_is_exited = true;

                                    break;
                                }
                                _ => { // mpsc::error::TryRecvError::Empty
                                }
                            },
                        };

                        let now = EchoTimeInstant::now();

                        if expire < now {
                            log::debug!("[RunnerFFMpeg::end] timeouted on awaiting reply msg({}) from FFMpegInner, ..",
                            "FFMpegInnerMsg::Terminate");

                            timeoutted_terminate = true;

                            break;
                        }
                    } // end of loop
                }
            }

            self.inner_handle = None;
        } else {
            // check FFMpegInner is exited
            inner_is_exited = true;
        }

        //
        // result
        //

        log::debug!(
            "[RunnerFFMpeg::end] termination result, inner_is_exited={}, timeoutted_terminated={} ",
            inner_is_exited,
            timeoutted_terminate
        );

        self.flush_ffmpeg_ts();

        Ok(DriverRstOk::Ok)
    }
}

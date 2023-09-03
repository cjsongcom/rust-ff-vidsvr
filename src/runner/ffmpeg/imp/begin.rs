use crate::runner::ffmpeg::message::*;
use crate::runner::ffmpeg::*;
use crate::{runner::*, ECHO_ASYNC_SLEEP_MS};

impl RunnerFFMpeg {
    pub(crate) async fn begin(&mut self) -> DriverRst {
        log::debug!("[RunnerFFMpeg][begin] ");

        {
            let mut create_ctx_ary: RunnerFFMCreateCtxAry = Vec::new();

            let receiver_ctx_ref = self.receiver_ctx.as_ref();
            let mut receiver_type: Option<RunnerFFMType> = None;

            let recorder_ctx_ref = self.recorder_ctx.as_ref();
            let mut recorder_type: Option<RunnerFFMType> = None;

            if receiver_ctx_ref.is_some() {
                receiver_type = Some(receiver_ctx_ref.unwrap().ffm_type);
                create_ctx_ary.push(receiver_ctx_ref.unwrap().clone());
            }

            if recorder_ctx_ref.is_some() {
                recorder_type = Some(recorder_ctx_ref.unwrap().ffm_type);
                create_ctx_ary.push(recorder_ctx_ref.unwrap().clone());
            }

            let responder = self.runner_msg_send.clone();

            let _inner = ffmpeg::create_runner_ffmpeg_inner(
                responder,
                receiver_type,
                recorder_type,
                create_ctx_ary,
            );

            if let Err(e) = _inner {
                return Err(RunnerError::RunnerOperErr(format!(
                    "failed to create runner ffmpeg inner, e={}",
                    e.to_string()
                )));
            }

            let inner = _inner.unwrap();

            self.inner_msg_send = Some(inner.inner_msg_send.clone());

            // ownership of 'inner' is moved to tokio
            let handle = tokio::spawn(inner.run());

            self.inner_handle = Some(handle);
        }

        //
        // await running
        //

        loop {
            let msg = self.runner_msg_recv.recv().await;

            match msg {
                Some(RunnerFFMpegMsg::BeginRunning) => {
                    log::info!("ffmpeg instance running..,");
                    break;
                }
                Some(RunnerFFMpegMsg::Finished(do_respawn)) => {
                    log::info!("ffmpeg instance finished.. do_respawn={}..", do_respawn);
                    break;
                }
                Some(RunnerFFMpegMsg::FailedSpawn) => {
                    // ex) /var/echo/record/d2572211, e=Permission denied (os error 13), desc=failed to spawn recorder ffmpeg process
                    let _em = format!("failed to spawn runner ffmpeg instance..");
                    log::error!("{}", _em);

                    return Err(RunnerError::RunnerDriverErr(_em));
                }
                Some(x) => {
                    log::info!("received msg, m={}", x.to_string());
                }
                _ => {
                    let _em = format!("can't receive ffmpeg start message(BeginRunning)");
                    log::error!("{}", _em);

                    return Err(RunnerError::RunnerDriverErr(_em));
                }
            }

            ECHO_ASYNC_SLEEP_MS!(100);
        }

        Ok(DriverRstOk::Ok)
    }
}

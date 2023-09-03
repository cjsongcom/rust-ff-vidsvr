use super::ffmpeg;
use super::DriverType;
use super::RunnerError;
use super::{Driver, DriverRst, DriverRstOk, DriverStatus};
use crate::comm::*;
use crate::comm_media::PropMedia;
use crate::config::Config;
use crate::runner::message::RecvWorkerMsgSend;
use crate::service::api::reqres::publish::ReqPropReceiverPrm;
use tokio::time::Duration;

impl Driver {
    pub async fn begin(&mut self) -> DriverRst {
        if self.status == DriverStatus::Creating {
            return Err(RunnerError::RecvDriverErr(format!("already creating")));
        }

        if self.status != DriverStatus::Init {
            return Err(RunnerError::RecvDriverErr(format!(
                "invalid staus, status={:?}",
                self.status
            )));
        }

        self.status = DriverStatus::Creating;

        if self.runner_ffmpeg.is_some() {
            self.runner_ffmpeg.as_mut().unwrap().begin().await?;
        }

        self.status = DriverStatus::ReceivingPublishStream;

        Ok(DriverRstOk::Ok)
    }

    pub async fn tick(&mut self) -> DriverRst {
        if self.status != DriverStatus::ReceivingPublishStream {
            let em = format!(
                "[Driver::tick] invalid calling state, status={}",
                self.status
            );
            tokio::time::sleep(Duration::from_millis(100)).await;

            return Err(RunnerError::RecvDriverErr(em));
        }

        //
        // DriverStatus::ReceivingPublishStream
        //

        if self.runner_ffmpeg.is_some() {
            let rst = self.runner_ffmpeg.as_mut().unwrap().tick().await?;

            match rst {
                DriverRstOk::Finished(do_respawn) => {
                    return Ok(rst);
                }
                _ => {}
            };
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(DriverRstOk::Ok)
    }

    pub async fn end(&mut self) -> DriverRst {
        match self.status {
            DriverStatus::End => {
                return Ok(DriverRstOk::Ok);
            }
            DriverStatus::Ending => {
                return Err(RunnerError::RecvDriverErr(format!("already ending..")));
            }
            _ => {}
        }

        //if self.status != DriverStatus::ReceivingPublishStream {
        if self.status == DriverStatus::Init {
            return Err(RunnerError::RecvDriverErr(format!(
                "invalid staus, status={}",
                self.status
            )));
        }

        self.status = DriverStatus::Ending;

        if self.runner_ffmpeg.is_some() {
            self.runner_ffmpeg.as_mut().unwrap().end().await?;
        }

        self.status = DriverStatus::End;

        Ok(DriverRstOk::Ok)
    }

    pub async fn restart(&mut self) -> DriverRst {
        log::debug!("[Driver::restart]");

        self.end().await?;

        self.status = DriverStatus::Restarting;

        if self.runner_ffmpeg.is_some() {
            self.runner_ffmpeg.as_mut().unwrap().reset().await?;
        }

        self.status = DriverStatus::Init;

        self.begin().await?;

        Ok(DriverRstOk::Ok)
    }
}

//
// runner driver
//

pub fn new(
    config: Config,
    worker_msg_send: RecvWorkerMsgSend,
    runner_type: DriverType,
    runner_prms: ReqPropReceiverPrm,
    media_prms: PropMedia,
    app_name: String,
    sess_key: String,
    publish_port: EchoPort,
) -> Result<Driver, RunnerError> {
    match runner_type {
        FFMpeg => {
            let runner = ffmpeg::create_runner_ffmpeg(
                config,
                worker_msg_send,
                runner_prms,
                media_prms,
                app_name,
                sess_key,
                publish_port,
            );

            if (runner.is_ok()) {
                return Ok(Driver {
                    status: DriverStatus::Init,
                    runner_ffmpeg: Some(runner.unwrap()),
                });
            } else {
                return Err(RunnerError::InternalError(format!("error")));
            }
        }

        e => return Err(RunnerError::UnSupportedRunnerDriverType(format!("{:?}", e))),
    }
}

use crate::comm::EchoTryRecvError;
use crate::runner::ffmpeg::message::*;
use crate::runner::ffmpeg::*;
use crate::runner::*;

impl RunnerFFMpeg {
    pub(in crate::runner) async fn tick(&mut self) -> DriverRst {
        if self.inner_handle.is_none() {
            return Ok(DriverRstOk::Finished(false));
        }

        //let msg = self.runner_msg_recv.recv().await;
        let recv = self.runner_msg_recv.try_recv();

        match recv {
            Ok(msg) => match msg {
                //RunnerFFMpegMsg::Finished(do_respawn, exit_code, stdout, stderr) => {
                RunnerFFMpegMsg::Finished(do_respawn) => {
                    log::debug!(
                        "[RunnerFFMpeg::tick] got msg 'FFMpegInnerMsg::RunIsFinished' \
                    do_respawn={}",
                        do_respawn //, exit_code={}, stdout={}, stderr={}",
                                   // do_respawn,
                                   // exit_code,
                                   // stdout,
                                   // stderr
                    );

                    // self.recv_worker_msg_send
                    //     .send(RecvWorkerMsg::RunnerIsExited(do_respawn)).unwrap();
                    return Ok(DriverRstOk::Finished(do_respawn));
                }
                _ => {}
            },
            Err(e) => match e {
                EchoTryRecvError::Empty => {}
                EchoTryRecvError::Disconnected => {
                    log::error!("[RunnerFFMpeg::tick] disconnected recv channel, self.runner_msg_recv.try_recv()");
                }
            },
        }

        Ok(DriverRstOk::Ok)
    }
}

use super::driver;
use super::DriverType;
use super::RunnerError;
use super::{Driver, DriverRstOk};
use crate::comm::*;
use crate::comm_media::MediaReceiver;
use crate::config::Config;
use crate::message::ServMsgSend;
use crate::service::api::reqres::publish::req::ReqPublishV3;
use crate::service::vsvr::{message::VSvrServMsgSend, util};
use anyhow::Result;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::Duration;
use PrmJson;

// vsvr service message
use super::message::{
    FinishRecvWorkerMsgOCResponder, RecvWorkerManagerMsg, RecvWorkerManagerMsgSend, RecvWorkerMsg,
    RecvWorkerMsgRecv, RecvWorkerMsgSend, StartRecvWorkerMsgOCResponder,
};

pub struct RecvWorker {
    //is_finished: bool,
    pub force_exit: bool,
    is_finished: bool, // got message teardown

    uuid: String,

    msg_recv: RecvWorkerMsgRecv,
    msg_send: RecvWorkerMsgSend,

    msg_send_to_manager: RecvWorkerManagerMsgSend,
    main_serv_msg_send: ServMsgSend,
    vsvr_serv_msg_send: VSvrServMsgSend,

    driver: Driver,

    config: Config,

    app_name: String,
    sess_key: String,

    publish_port: u16,

    expire_duration: EchoTimeDuration,
    expire_instant: EchoTimeInstant,

    start_epoch: EchoTimeEpoch,
    expire_epoch: EchoTimeEpoch,

    expired: bool,

    runner_respawn_cnt: i32,
}

impl RecvWorker {
    pub fn clone_msg_send(&self) -> RecvWorkerMsgSend {
        self.msg_send.clone()
    }

    pub fn get_main_serv_msg_sender_ref(&self) -> &ServMsgSend {
        &self.main_serv_msg_send
    }

    fn peek_message(&mut self) -> Result<RecvWorkerMsg, mpsc::error::TryRecvError> {
        self.msg_recv.try_recv()
    }

    async fn handle_msg_query_worker_state(&mut self) -> Result<(), RunnerError> {
        log::debug!("[RecvWorker] got msg 'QueryWorkerState'");
        Ok(())
    }

    async fn handle_msg_start_recv_worker(
        &mut self,
        responder: StartRecvWorkerMsgOCResponder,
        publish_prms: PrmJson,
    ) -> Result<(), RunnerError> {
        Ok(())
    }

    async fn handle_msg_finish_recv_worker(
        &mut self,
        responder: FinishRecvWorkerMsgOCResponder,
        finish_prms: PrmJson,
    ) -> Result<(), RunnerError> {
        if self.is_finished {
            let em = format!("[RecvWorker] already finishing..");
            log::error!("{}", em);

            return Err(RunnerError::RecvWorkerOperErr(em));
        }

        self.is_finished = true;
        log::debug!("[RecvWorker] finishing.., enqueue finish worker message");

        responder.send(Ok(json!({}))).map_err(|e| {
            RunnerError::MsgChanErrSendFail(format!("handle_msg_finish_recv_worker, : "))
        })?;

        Ok(())
    }

    async fn proc_msg(&mut self) -> Result<(), RunnerError> {
        match self.peek_message() {
            Ok(msg) => match msg {
                RecvWorkerMsg::QueryRecvWorkerState => self.handle_msg_query_worker_state().await?,

                RecvWorkerMsg::FinishRecvWorker(response, finish_prms) => {
                    self.handle_msg_finish_recv_worker(response, finish_prms)
                        .await?
                }

                RecvWorkerMsg::StartRecvWorker(response, publish_prms) => {
                    self.handle_msg_start_recv_worker(response, publish_prms)
                        .await?
                }

                // RecvWorkerMsg::RunnerIsExited(do_respawn ) => {}
                _ => {}
            },

            Err(e) => match e {
                mpsc::error::TryRecvError::Disconnected => {
                    //mpsc::error::TryRecvError::Closed => {
                    return Err(RunnerError::MsgChanErrChannelClosed(format!(
                        "[RecvWorker::proc_msg] e={}",
                        e.to_string()
                    )));
                }
                _ => {} // mpsc::error::TryRecvError::Empty
            },
        }

        Ok(())
    }

    fn send_msg_to_manager(&mut self, msg: RecvWorkerManagerMsg) -> Result<(), RunnerError> {
        if let Err(e) = self.msg_send_to_manager.send(msg) {
            // manager receiver channel, already closed ?
            return Err(RunnerError::SendingMsgToWorkerManagerFailed(format!(
                "worker-uuid={}, {:?}",
                self.uuid.to_string(),
                e
            )));
        }

        Ok(())
    }

    //msg_peek_timeout_ms: i16, // Duration::from_millis(500),

    pub fn new(
        config: Config,
        uuid: String,
        msg_send_to_manager: RecvWorkerManagerMsgSend,
        main_serv_msg_send: ServMsgSend,
        vsvr_serv_msg_send: VSvrServMsgSend,
        req_publish: ReqPublishV3,
        publish_port: u16,
        expire_duration: EchoTimeDuration,
    ) -> Result<RecvWorker, RunnerError> {
        let driverType = match req_publish.receiver {
            MediaReceiver::FFMPEG => DriverType::FFMPEG,
            e => return Err(RunnerError::UnSupportedMediaReciverType(format!("{:?}", e))),
        };

        let (sender, receiver) = mpsc::unbounded_channel();

        let driver = driver::new(
            config.clone(),
            sender.clone(),
            DriverType::FFMPEG,
            req_publish.receiver_prm.clone(),
            req_publish.media.clone(),
            req_publish.app_name.clone(),
            req_publish.sess_key.clone(),
            publish_port,
        )?;

        let start_epoch = get_echo_epoch();
        let expire_epoch = start_epoch + expire_duration.as_secs() as i64;

        let inst = Self {
            //is_finished: false,
            force_exit: false,
            is_finished: false,

            uuid: uuid.clone(),

            msg_recv: receiver,
            msg_send: sender.clone(),

            msg_send_to_manager,
            main_serv_msg_send,
            vsvr_serv_msg_send,

            driver: driver,

            config: config.clone(),

            app_name: req_publish.app_name.clone(),
            sess_key: req_publish.sess_key.clone(),

            publish_port,

            expire_duration,
            expire_instant: EchoTimeInstant::now() + expire_duration,

            start_epoch,
            expire_epoch,

            expired: false,

            runner_respawn_cnt: 0,
        };

        //Ok(Arc::new((inst, sender)))
        Ok(inst)
    }

    async fn inner_begin(&mut self) -> Result<(), RunnerError> {
        if let Err(e) = self.driver.begin().await {
            log::debug!("[RecvWorker::run] error on driver begin.., e={}", e);

            return Err(RunnerError::FailedToBeginRecvWorker(format!(
                "Failed to begin receive worker.., e={}",
                e
            )));
        }

        // hls provider svr addr: ECHO_HLS_PROVIDER_ADDR
        log::debug!(
            "[RecvWorker::begin] register hls provider addr to vsvr, app_name={}, hls_addr={}",
            self.app_name,
            self.config.echo_hls_ipaddr
        );

        util::register_hls_provider_addr_to_vsvr(
            self.vsvr_serv_msg_send.clone(),
            self.app_name.clone(),
            self.config.echo_hls_ipaddr,
        )
        .await
        .map_err(|e| {
            let em = format!("f=inner_begin, e={}", e.to_string());
            log::error!("{}", em);

            RunnerError::RunnerOperErr(em)
        })?;

        if self.config.vsvr_api_use_pub_unpub {
            log::debug!(
                "[RecvWorker::begin] update vsvr publish state to vsvr,\
             state=publish, app_name={}, sess_key={}",
                self.app_name,
                self.sess_key
            );

            util::update_vsvr_publish_state(
                self.vsvr_serv_msg_send.clone(),
                self.app_name.clone(),
                self.sess_key.clone(),
                crate::service::vsvr::message::VSvrLiveState::Publish,
            )
            .await
            .map_err(|e| {
                let em = format!(
                    "f=inner_begin:update_vsvr_publish_state, e={}",
                    e.to_string()
                );
                log::error!("{}", em);

                RunnerError::RunnerOperErr(em)
            })?;
        } else {
            log::debug!(
                "[RecvWorker::begin] skipped updating vsvr publish state\
             to vsvr by config, state=publish, app_name={}, sess_key={}",
                self.app_name,
                self.sess_key
            );
        }

        Ok(())
    }

    pub async fn begin(&mut self) -> Result<(), RunnerError> {
        log::debug!("[RecvWorker] begin.. , uuid={}", self.uuid.to_string());

        if let Err(e) = self.inner_begin().await {
            match self.driver.end().await {
                Ok(_) => {}
                Err(e2) => {
                    let em2 = format!(
                        "[RecvWorker] failed to end driver.. , uuid={}, e={}, e2={}",
                        self.uuid.to_string(),
                        e.to_string(),
                        e2.to_string()
                    );

                    log::error!("{}", em2);

                    return Err(RunnerError::FailedToBeginRecvWorker(em2));
                }
            }

            return Err(e);
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<(), RunnerError> {
        // log::debug!(
        //     "[RecvWorker] running.. ,
        //     worker_uuid={}, app_name={}, sess_key={}, \
        //     expire_duration_sec={}, start_epoch={}, expire_epoch={} \
        //     , publish_port={}",
        //     self.uuid.to_string(),
        //     self.app_name,
        //     self.sess_key,
        //     self.expire_duration.as_secs().to_string(),
        //     self.start_epoch,
        //     self.expire_epoch,
        //     self.publish_port
        // );

        crate::mlog::echo::session::event(
            "info",
            "session_created",
            &self.app_name,
            &self.uuid,
            json!({
                "uuid"         : self.uuid,
                "app_name"     : self.app_name,
                "sess_key"     : self.sess_key,
                "expire_dur"   : self.expire_duration.as_secs(),
                "start_epoch"  : self.start_epoch,
                "expire_epoch" : self.expire_epoch,
                "publish_port" : self.publish_port
            }),
        );

        //
        // loop
        //

        //while !self.is_finished {
        loop {
            match self.proc_msg().await {
                Err(RunnerError::MsgChanErrChannelClosed(e)) => {
                    log::error!(
                        "[RecvWorker::run] receive channel is closed.., force exiting.., e={}",
                        e
                    );
                    self.force_exit = true;

                    break;
                }

                Err(e) => {
                    log::error!(
                        "[RecvWorker::run] error occured, force exiting.., e={}",
                        e.to_string()
                    );
                    self.force_exit = true;

                    break;
                }

                Ok(_) => {
                    if self.is_finished {
                        log::info!("[RecvWorker::run] exiting by finished");
                        break;
                    }
                }
            }

            match self.driver.tick().await {
                Err(e) => {
                    log::debug!("[RecvWorker::run] error on driver tick.., e={}", e);
                    break;
                }

                Ok(r) => {
                    if let DriverRstOk::Finished(do_respawn) = r {
                        log::debug!(
                            "[RecvWorker::run] got msg 'respawn driver', do_respawn={}",
                            do_respawn
                        );

                        if do_respawn {
                            let rst_restart = self.driver.restart().await;

                            match rst_restart {
                                Ok(_) => {
                                    self.runner_respawn_cnt += 1;

                                    log::debug!(
                                        "[RecvWorker::run] driver is restarted.. respawn_count={}",
                                        self.runner_respawn_cnt
                                    );

                                    continue; // goto loop
                                }
                                Err(e) => {
                                    log::debug!(
                                        "[RecvWorker::run] error on restarting driver.., e={}",
                                        e
                                    );
                                }
                            }
                        }

                        //self.is_finished = true;
                        break;
                    }
                }
            }

            if self.force_exit {
                log::debug!("[RecvWorker::run] force exiting..");
                break;
            } else {
                // prevent blocking thread, do not block other task is beging drive
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Normal 2 hour, Maximum 4 hours(vsvr coupon)
            if self.check_expired() {
                let cur_epoch = get_echo_epoch();

                log::info!(
                    "[RecvWorker::run] echo worker duration is expired.. exiting.., \
                    cur_time_epoch={}, expire_duration={}, start_epoch={}, expire_epoch={}",
                    cur_epoch,
                    self.expire_duration.as_secs(),
                    self.start_epoch,
                    self.expire_epoch
                );

                self.expired = true;

                crate::mlog::echo::session::event(
                    "info",
                    "session_expired",
                    &self.app_name,
                    &self.uuid,
                    json!({
                       "cur_epoch" : cur_epoch,
                       "expire_dur_sec" : self.expire_duration.as_secs(),
                       "start_epoch=" : self.start_epoch,
                       "expire_epoch=" : self.expire_epoch,
                    }),
                );

                break;
            }
        } // end of loop

        //
        // exiting
        //

        if let Err(e) = self.driver.end().await {
            let em = format!("[RecvWorker::run::driver.end] failed to exit driver");
            log::error!("{}", em);

            crate::mlog::echo::session::event(
                "err",
                "session_exit",
                &self.app_name,
                &self.uuid,
                json!({
                    "exit_fail" : 1,
                    "cur_epoch" : get_echo_epoch(),
                    "start_epoch"  : self.start_epoch,
                    "expire_epoch" : self.expire_epoch,
                    "exit_fail_desc" : em
                }),
            );

            return Err(RunnerError::RecvDriverErr(em));
        }

        // notify to manager
        if let Err(_) = self.send_msg_to_manager(RecvWorkerManagerMsg::NotifyRecvWorkerIsExiting(
            self.uuid.to_string(),
            self.app_name.to_string(),
            self.sess_key.to_string(),
        )) {
            let em = format!(
                "[RecvWorker::run::send_msg_to_manager] \
             failed to send msg 'NotifyRecvWorkerIsExiting' to manager"
            );
            log::error!("{}", em);

            crate::mlog::echo::session::event(
                "err",
                "session_exit",
                &self.app_name,
                &self.uuid,
                json!({
                   "exit_fail" : 1,
                   "cur_epoch" : get_echo_epoch(),
                   "start_epoch"  : self.start_epoch,
                   "expire_epoch" : self.expire_epoch,
                   "exit_fail_desc" : em
                }),
            );

            return Err(RunnerError::MsgChanErrSendFail((em)));
        }

        log::debug!("[RecvWorker::run] exiting..");

        crate::mlog::echo::session::event(
            "info",
            "session_exit",
            &self.app_name,
            &self.uuid,
            json!({
                "exit_fail" : 0,
                "cur_epoch" : get_echo_epoch(),
                "start_epoch"  : self.start_epoch,
                "expire_epoch" : self.expire_epoch
            }),
        );

        Ok(())
    }

    pub fn check_expired(&mut self) -> bool {
        let now = EchoTimeInstant::now();

        if self.expire_instant < now {
            return true;
        }

        false
    }
}

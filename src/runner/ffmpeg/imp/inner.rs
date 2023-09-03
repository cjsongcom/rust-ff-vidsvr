use crate::comm::EchoTimeDuration;
use crate::comm_ps;
use crate::comm_ps::*;
use crate::runner::ffmpeg::RunnerFFMError;
use crate::runner::ffmpeg::RunnerFFMpegInner;
use crate::runner::TERMINATE_TIMEOUT_MS;
use crate::runner::{
    ffmpeg::message::{RunnerFFMpegInnerMsg, RunnerFFMpegMsg, RunnerFFMpegMsgSend},
    ffmpeg::{
        imp::spawn::spawn_ffmpeg_from_creation_ctx, RunnerFFMCreateCtxAry, RunnerFFMType,
        RunnerFFMpegInnerExitRst, RunnerFFMpegInnerProcCtx, RunnerFFMpegInnerProcCtxMap,
    },
};
use crate::ECHO_ASYNC_SLEEP_MS;
use crate::ECHO_TIME_DURATION_MS;
use crate::ECHO_TIME_DURATION_SEC;
use std::collections::LinkedList;
use tokio::sync::mpsc;

pub fn create_runner_ffmpeg_inner(
    responder: RunnerFFMpegMsgSend,

    receiver_type: Option<RunnerFFMType>,
    recorder_type: Option<RunnerFFMType>,

    create_ctxs: RunnerFFMCreateCtxAry,
) -> Result<RunnerFFMpegInner, RunnerFFMError> {
    //let mut proc_names = RunnerFFMpegInnerProcTypeAry::new();
    let mut proc_ctx_map = RunnerFFMpegInnerProcCtxMap::new();

    for x in create_ctxs.iter() {
        let info = RunnerFFMpegInnerProcCtx {
            create_ctx: x.clone(),

            no_need_termination: x.no_need_termination,
            auto_respawn: x.auto_respawn,

            is_spawned: false,
            spawn_err: None,

            proc: None,
            proc_cmd: None,

            term_exit_rst: None,
            term_err: None,
        };

        //proc_names.push(x.ffm_type);
        proc_ctx_map.insert(x.ffm_type, info);
    }

    let (inner_msg_send, inner_msg_recv) = tokio::sync::mpsc::unbounded_channel();

    Ok(RunnerFFMpegInner {
        responder,
        receiver_type,
        recorder_type,
        proc_ctx_map,
        output: LinkedList::new(),
        inner_msg_send,
        inner_msg_recv,
        force_terminating: false,
    })
}

async fn terminate_ffm(
    ffm_type: &RunnerFFMType,
    ctx_map: &mut RunnerFFMpegInnerProcCtxMap,
    poll_term_timeout_ms: EchoTimeDuration,
) -> Result<PollExitStRst, RunnerFFMError> {
    let ctx = ctx_map.get_mut(ffm_type).unwrap();
    let poll_exit_rst;

    if ctx.no_need_termination {
        poll_exit_rst = PollExitStRst {
            exit_code: 0,
            exit_desc: String::from("no need to termination"),
        };
        ctx.term_exit_rst = Some(poll_exit_rst.clone());

        return Ok(poll_exit_rst);
    } else {
        let proc = ctx.proc.as_mut().unwrap();
        let _term_rst = comm_ps::terminate(proc.id().unwrap());
        let mut _term_err = Option::<RunnerFFMError>::None;

        match _term_rst {
            Ok(mut _kill_proc) => {
                _kill_proc.wait().await.unwrap(); // give working chance to process 'kill SIGTERM <PID>'

                // polling ffmpeg process is exited
                let _poll_rst: Result<PollExitStRst, EchoPSError> =
                    comm_ps::poll_exit_status(proc, poll_term_timeout_ms).await;

                ECHO_ASYNC_SLEEP_MS!(100);

                match _poll_rst {
                    Ok(r) => return Ok(r),
                    Err(EchoPSError::PollingStatusTimeout) => {
                        _term_err = Some(RunnerFFMError::FFMpegTermTimeout)
                    }
                    Err(e) => _term_err = Some(RunnerFFMError::FFMpegTermErr(e.to_string())),
                }
            }
            Err(e) => {
                _term_err = Some(RunnerFFMError::FFMpegTermErr(e.to_string()));
            }
        }

        ctx.term_err = _term_err.clone();

        let poll_exit_rst = PollExitStRst {
            exit_code: 999,
            exit_desc: ctx.term_err.as_ref().unwrap().to_string(),
        };

        ctx.term_exit_rst = Some(poll_exit_rst.clone());

        Err(_term_err.unwrap())
    }
}

fn get_ffm_running_cnt(proc_ctx_map: &RunnerFFMpegInnerProcCtxMap) -> u32 {
    let mut running_count = 0;

    for (_, ctx) in proc_ctx_map.iter() {
        // not spawend or already exited
        if ctx.proc.is_none() || ctx.term_exit_rst.is_some() {
            continue;
        }
        running_count += 1;
    }

    running_count
}

async fn update_ffm_exit_status(
    proc_ctx_map: &mut RunnerFFMpegInnerProcCtxMap,
    poll_timeout_ms: EchoTimeDuration,
) -> Result<(), RunnerFFMError> {
    for (_, ctx) in proc_ctx_map.iter_mut() {
        if ctx.proc.is_none() {
            // not spawned
            continue;
        }

        if ctx.term_exit_rst.is_none() {
            let poll_rst =
                comm_ps::poll_exit_status(ctx.proc.as_mut().unwrap(), poll_timeout_ms).await;

            match poll_rst {
                Ok(poll_exit_rst) => {
                    log::debug!(
                        "ffm process is exited.., exit_code={}, desc={}",
                        poll_exit_rst.exit_code,
                        poll_exit_rst.exit_desc
                    );
                    ctx.term_exit_rst = Some(poll_exit_rst);
                    // process is just terminated
                }
                Err(EchoPSError::PollingStatusTimeout) => {}
                Err(e) => {
                    let _em = format!("failed to polling ffm, e={}", e.to_string());
                    log::error!("{}", _em);
                }
            };
        } else {
            // process is already terminated
        }

        ECHO_ASYNC_SLEEP_MS!(10);
    }

    Ok(())
}

pub async fn _spawn_ffmpegs_process(
    receiver_type: Option<&RunnerFFMType>,
    recorder_type: Option<&RunnerFFMType>,
    proc_ctx_map: &mut RunnerFFMpegInnerProcCtxMap,
) -> Result<(), RunnerFFMError> {
    let mut spawn_retry_max_time_ms = EchoTimeDuration::from_millis(10000);

    let mut err_no: i32 = 0;

    let mut last_err: Option<RunnerFFMError> = None;
    let mut last_err_msg = Option::<String>::None;
    let mut spawned_proc_ffm_types: Vec<RunnerFFMType> = Vec::new();

    //
    // spawn receiver
    //

    if last_err.is_none() {
        if receiver_type.is_some() {
            let mut proc_ctx = proc_ctx_map.get_mut(receiver_type.unwrap()).unwrap();

            let _spawn_rst =
                spawn_ffmpeg_from_creation_ctx(&mut proc_ctx.create_ctx, spawn_retry_max_time_ms)
                    .await;

            if let Err(e) = _spawn_rst {
                last_err = Some(e);

                let _em = format!(
                    "failed to spawn receiver ffmpeg process, ffm_type={}, app_name={}, cmd={}",
                    proc_ctx.create_ctx.ffm_type,
                    proc_ctx.create_ctx.owner_info.app_name,
                    proc_ctx.create_ctx.ffmpeg_cmd.args_to_string()
                );
                log::error!("{}", _em);

                proc_ctx.is_spawned = false;
                proc_ctx.spawn_err = Some(RunnerFFMError::FFMpegSpawnErr(_em.to_string()));

                last_err_msg = Some(_em);
            } else {
                let (ffmpeg_proc, ffmpeg_proc_cmd) = _spawn_rst.unwrap();

                proc_ctx.is_spawned = true;
                proc_ctx.proc = Some(ffmpeg_proc);
                proc_ctx.proc_cmd = Some(ffmpeg_proc_cmd);

                spawned_proc_ffm_types.push(proc_ctx.create_ctx.ffm_type);
            }
        }
    }

    //
    // spawn recorder
    //

    if last_err.is_none() {
        if recorder_type.is_some() {
            let mut proc_ctx = proc_ctx_map.get_mut(recorder_type.unwrap()).unwrap();

            let _spawn_rst =
                spawn_ffmpeg_from_creation_ctx(&mut proc_ctx.create_ctx, spawn_retry_max_time_ms)
                    .await;

            if let Err(e) = _spawn_rst {
                last_err = Some(e);

                let _em = format!(
                    "failed to spawn recorder ffmpeg process, ffm_type={}, app_name={}, cmd={}",
                    proc_ctx.create_ctx.ffm_type,
                    proc_ctx.create_ctx.owner_info.app_name,
                    proc_ctx.create_ctx.ffmpeg_cmd.args_to_string()
                );

                log::error!("{}", _em);

                last_err_msg = Some(_em);
            } else {
                let (ffmpeg_proc, ffmpeg_proc_cmd) = _spawn_rst.unwrap();

                proc_ctx.proc = Some(ffmpeg_proc);
                proc_ctx.proc_cmd = Some(ffmpeg_proc_cmd);
                spawned_proc_ffm_types.push(proc_ctx.create_ctx.ffm_type);
            }
        }
    }

    //
    // rollback if error occured, cleanup
    //

    if last_err.is_some() {
        //
        // terminate spawned ffmpeg
        //

        log::debug!("[cleanup] error occured in spawning process, cleanup  spawned ffmpeg process by reversed order, e={}, desc={}", 
            last_err.as_ref().unwrap().to_string(),
            last_err_msg.as_ref().unwrap().to_string());

        // terminate ffm process by reversed spawned order
        for ffm_type in spawned_proc_ffm_types.iter().rev() {
            // for (ffm_type, ctx) in proc_ctx_map.iter_mut() {
            let ctx = proc_ctx_map.get_mut(ffm_type).unwrap();
            let desc = String::new();

            log::debug!("[cleanup] cur_ffm_type={}", ctx.create_ctx.ffm_type);

            let _proc = ctx.proc.as_mut();

            if _proc.is_none() {
                // 1. does not spawned
                // 2. error on spawning
                // 3. already exited
                log::debug!(
                    "[cleanup] skip terminate , proc is empty, ffm_type={} ",
                    ctx.create_ctx.ffm_type
                );
                continue;
            }

            let proc = _proc.unwrap();

            let poll_timeout_ms = ECHO_TIME_DURATION_SEC!(50);

            log::debug!("[cleanup] terminate spawned ffmpeg process and waiting for process is exit, ffm={}, app_name={}, max_timeout_ms={}",
                ctx.create_ctx.ffm_type, ctx.create_ctx.owner_info.app_name, poll_timeout_ms.as_millis());

            let _pid = proc.id();

            if _pid.is_some() {
                let pid = _pid.unwrap();
                let _term_rst = comm_ps::terminate(pid);

                if let Err(e) = _term_rst {
                    log::debug!(
                        "[cleanup] pid is valid({}), but error is occured, e={}",
                        pid,
                        e.to_string()
                    );
                } else {
                    // polling exit status of termination process
                    let _term_poll_rst = comm_ps::poll_exit_status(proc, poll_timeout_ms).await;

                    if let Err(e) = _term_poll_rst {
                        log::error!("[cleanup] failed to terminate process, e={}", e.to_string());
                    } else {
                        let term_poll_rst = _term_poll_rst.unwrap();
                        log::debug!(
                            "terminated previous process, exit_code={}, desc={}",
                            term_poll_rst.exit_code,
                            term_poll_rst.exit_desc
                        );
                    }
                }
            } else {
                log::debug!("[cleanup] pid is empty, skip terminate");
            }
        }

        return Err(RunnerFFMError::FFMpegSpawnErr(format!(
            "failed to spawn receiver/recorder ffmpeg process, e={}, desc={}",
            last_err.as_ref().unwrap().to_string(),
            last_err_msg.as_ref().unwrap().to_string()
        )));
    }

    Ok(())
}

//
// RunnerFFMpegInner
//

impl RunnerFFMpegInner {
    //
    // run
    //
    pub async fn run(mut self) -> Result<RunnerFFMpegInnerExitRst, RunnerFFMError> {
        let run_rst: Result<RunnerFFMpegInnerExitRst, RunnerFFMError>;

        //
        // spawn ffmpeg process
        //
        self.responder.send(RunnerFFMpegMsg::Spawning).unwrap();

        if let Err(e) = _spawn_ffmpegs_process(
            self.receiver_type.as_ref(),
            self.recorder_type.as_ref(),
            &mut self.proc_ctx_map,
        )
        .await
        {
            run_rst = Err(e);
            return self.on_exit(run_rst).await;
        }

        self.responder.send(RunnerFFMpegMsg::BeginRunning).unwrap();

        let poll_timeout_ms = ECHO_TIME_DURATION_MS!(50);

        'entry: loop {
            // pump message until exiting
            match self.proc_message().await {
                Ok(inner_exit_rst) => {
                    if inner_exit_rst.is_some() {
                        run_rst = Ok(inner_exit_rst.unwrap());
                        // terminate message loop and exit function
                        log::debug!("[FFMpegInner::run] exited normally.");

                        break 'entry;
                    } else {
                        // continue pump message
                    }
                }
                Err(e) => {
                    log::error!("[FFMpegInner::run] exited with error. e={}", e.to_string());
                    run_rst = Err(e);

                    break 'entry;
                }
            };

            // update spawned ffmpeg process exit status
            let update_rst = update_ffm_exit_status(&mut self.proc_ctx_map, poll_timeout_ms).await;

            if let Err(e) = update_rst {
                run_rst = Err(e);
                break 'entry;
            }

            if get_ffm_running_cnt(&self.proc_ctx_map) == 0 {
                run_rst = Ok(RunnerFFMpegInnerExitRst {
                    all_terminated: true,
                    //do_respawn: self.do_respawn,
                    // exit_status,
                });
                break 'entry;
            }

            ECHO_ASYNC_SLEEP_MS!(10);
        } // end of 'entry:loop

        self.on_exit(run_rst).await
    }

    async fn on_exit(
        &mut self,
        run_rst: Result<RunnerFFMpegInnerExitRst, RunnerFFMError>,
    ) -> Result<RunnerFFMpegInnerExitRst, RunnerFFMError> {
        let mut do_respawn: bool = false;
        let mut response_msg: RunnerFFMpegMsg;

        // session time is over (>4hour), session closed by admin
        if run_rst.is_ok() {
            if self.force_terminating {
                log::info!("[FFMpegInner::run, on_exit] exiting.., self.force_terminating=true, disabled respawn..");
                response_msg = RunnerFFMpegMsg::Finished(do_respawn);
            } else {
                log::info!("[FFMpegInner::run, on_exit] exiting... but respawn agian..");
                do_respawn = true;
                response_msg = RunnerFFMpegMsg::Finished(do_respawn);
            }
        } else {
            log::info!(
                "[FFMpegInner::run, on_exit] exiting with error, disabled respawn.., e={}",
                run_rst.clone().err().unwrap().to_string()
            );
            response_msg = RunnerFFMpegMsg::FailedSpawn;
        }

        // send termination message
        if let Err(e) = self.responder.send(response_msg.clone()) {
            // responder channel is already closed ?
            let _em = format!(
                "[FFMpegInner::run] failed to send message, msg={}, do_respawn={}, e={}",
                response_msg.to_string(),
                do_respawn,
                e.to_string()
            );

            log::error!("{}", _em);
        }

        run_rst
    }

    //
    // proc_message
    //

    async fn proc_message(&mut self) -> Result<Option<RunnerFFMpegInnerExitRst>, RunnerFFMError> {
        // maximum 2 second
        let poll_term_timeout_ms = ECHO_TIME_DURATION_MS!(TERMINATE_TIMEOUT_MS);

        //tokio::time::sleep(ECHO_TIME_DURATION_MS!(10)).await;
        ECHO_ASYNC_SLEEP_MS!(10);

        match self.inner_msg_recv.try_recv() {
            Ok(msg) => {
                return match msg {
                    RunnerFFMpegInnerMsg::Terminate(responder) => {
                        // case #1: session time over, 4hour
                        // case #2: force closed session by admin
                        self.force_terminating = true; // do not respawn

                        log::debug!(
                            "[FFMpegInner::run_inner, proc_message] got msg ({}), starting terminate.. ",
                            "FFMpegInnerMsg::Terminate"
                        );

                        if self.receiver_type.is_some() {
                            // terminate receiver
                            let _term_rst = terminate_ffm(
                                self.receiver_type.as_ref().unwrap(),
                                &mut self.proc_ctx_map,
                                poll_term_timeout_ms,
                            )
                            .await;

                            match _term_rst {
                                Err(RunnerFFMError::FFMpegTermTimeout) => {}
                                Err(e) => {
                                    log::error!(
                                        "[FFMpegInner::run_inner, proc_message,\
                                     FFMpegInnerMsg::Terminate] failed to terminate, e={}",
                                        e.to_string()
                                    );
                                }
                                _ => {}
                            }
                        }

                        if self.recorder_type.is_some() {
                            // terminate recorder
                            let _term_rst = terminate_ffm(
                                self.recorder_type.as_ref().unwrap(),
                                &mut self.proc_ctx_map,
                                poll_term_timeout_ms,
                            )
                            .await;

                            match _term_rst {
                                Err(RunnerFFMError::FFMpegTermTimeout) => {}
                                Err(e) => {
                                    log::error!(
                                        "[FFMpegInner::run_inner, proc_message,\
                                     FFMpegInnerMsg::Terminate] failed to terminate, e={}",
                                        e.to_string()
                                    );
                                }
                                _ => {}
                            }
                        }

                        //
                        // check termination status and respond
                        //

                        let mut max_retry_count = 10;
                        let mut retry_cnt = 0;
                        let mut exit_rst: Result<RunnerFFMpegInnerExitRst, RunnerFFMError>;

                        'inner_chk_loop: loop {
                            let update_rst = update_ffm_exit_status(
                                &mut self.proc_ctx_map,
                                ECHO_TIME_DURATION_MS!(50),
                            )
                            .await;

                            if let Err(e) = update_rst {
                                exit_rst = Err(RunnerFFMError::FFMpegTermErr(e.to_string()));
                                break 'inner_chk_loop;
                            }

                            let running_cnt = get_ffm_running_cnt(&self.proc_ctx_map);

                            if running_cnt == 0 {
                                log::debug!("all ffm is terminated..");
                                exit_rst = Ok(RunnerFFMpegInnerExitRst {
                                    all_terminated: true,
                                });

                                break 'inner_chk_loop;
                            } else {
                                log::debug!(
                                    "remaining unterminated ffm process count={}, retry_cnt={}",
                                    running_cnt,
                                    retry_cnt
                                );
                            }

                            retry_cnt += 1;

                            if retry_cnt > max_retry_count {
                                let _em = format!("exceed max retrycount={}, aborting.. remain ffm process cnt={}",
                                max_retry_count, running_cnt);
                                log::error!("{}", _em);

                                exit_rst = Err(RunnerFFMError::FFMpegTermErr(_em));

                                break 'inner_chk_loop;
                            }

                            ECHO_ASYNC_SLEEP_MS!(10);
                        }

                        let _term_desc: String;

                        if let Err(e) = exit_rst.clone() {
                            _term_desc = format!("e={}", e.to_string());
                        } else {
                            _term_desc = format!("ok");
                        }

                        log::debug!(
                            "[RunnerFFMpegInner::proc_meesage, RunnerFFMpegInnerMsg::Terminate] {}",
                            _term_desc
                        );

                        //
                        // response message
                        //

                        let res_rst = responder.send(exit_rst.clone());

                        if let Err(e) = res_rst {
                            let _em = format!(
                                "[RunnerFFMpegInner::proc_meesage] failed to send response"
                            );
                            log::error!("{}", _em);

                            exit_rst = Err(RunnerFFMError::InnerOperErr(_em));
                        }

                        match exit_rst {
                            Ok(r) => Ok(Some(r)),
                            Err(e) => Err(e),
                        }
                    } // end of message (RunnerFFMpegInnerMsg::Terminate)
                    _ => Ok(None),
                };
            }
            // error on channel, may channel is closed ?
            Err(e) => match e {
                mpsc::error::TryRecvError::Disconnected => {
                    return Err(RunnerFFMError::MsgChanErr(
                        "disconnected",
                        "tryrecverror",
                        "FFMpegInner::run_inner",
                        e.to_string(),
                    ));
                }
                _ => { // mpsc::error::TryRecvError::Empty
                     // return Err(Error::MsgChanErrRecvMsgChannelIsEmpty(
                     //     "[FFMpegInner::run]".to_string(), "inner_msg_recv.try_recv()".to_string()));
                }
            },
        };

        Ok(None)
    }
}

// runner::ffmpeg::mod.rs
pub mod cmd;
pub mod error;
pub mod imp;
pub mod message;
pub mod receiver;
pub mod recorder;
use self::cmd::FFMpegCmd;
use self::message::{
    RunnerFFMpegInnerMsgRecv, RunnerFFMpegInnerMsgSend, RunnerFFMpegMsgRecv, RunnerFFMpegMsgSend,
};
use super::message::RecvWorkerMsgSend;
use super::{RunnerProcChild, RunnerProcCmd};
use crate::comm::{EchoArc, EchoAsyncRwLock};
use crate::comm_ps::PollExitStRst;
use crate::config::Config;
use crate::service::api::reqres::publish::ReqPropReceiverPrm;
use crate::EchoJoinHandle;
use crate::PropMedia;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use strum_macros;
use strum_macros::Display;

type RunnerFFMError = error::RunnerFFMError;
type RunnerFFMProc = RunnerProcChild;
type RunnerFFMProcShared = EchoArc<EchoAsyncRwLock<RunnerFFMProc>>;
type RunnerFFMProcCmd = RunnerProcCmd;
type RunnerFFMProcCmdShared = EchoArc<EchoAsyncRwLock<RunnerProcCmd>>;

//
// RunnerFFMEvtRst
//

type RstOnSpawnFFMpeg = (RunnerFFMProc, RunnerFFMProcCmd);
type RstPostSpawnFFMpeg = ();

pub type PFnOnSpawnFFMpeg = fn(&RunnerFFMCreateCtx) -> Result<RstOnSpawnFFMpeg, RunnerFFMError>;
pub type PFnPostSpawnFFMpeg = fn(&RunnerFFMCreateCtx) -> Result<RstPostSpawnFFMpeg, RunnerFFMError>;

#[derive(Display, Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RunnerFFMType {
    // RTMP, RTP
    RUNNER_FFM_RECEVIER(&'static str),

    // PLAYLIST_M3U8
    RUNNER_FFM_RECORDER(&'static str),
}

pub type RunnerFFMTypeAry = Vec<RunnerFFMType>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerFFMOwnerInfo {
    app_name: String,
    sess_key: String,
}

#[derive(Clone)]
pub struct RunnerFFMCreateCtx {
    ffm_type: RunnerFFMType,
    owner_info: RunnerFFMOwnerInfo,
    props: HashMap<String, String>,

    ffmpeg_cmd: FFMpegCmd,

    on_spawn: PFnOnSpawnFFMpeg,
    post_spawn: PFnPostSpawnFFMpeg,

    // no need to terminate explicitly
    // ex) ffmpeg playlist-m3u8 recorder
    //     if ffmpeg process which generate playlist.m3u8 periodic is exit,
    //        ffmpeg playlist-m3u8 recorder will exit automataically without terminate explicitly
    no_need_termination: bool,

    auto_respawn: bool,
}
// proc: Option<RunnerFFMProcShared>,
// proc_cmd: Option<RunnerFFMProcCmdShared>,

pub type RunnerFFMCreateCtxAry = Vec<RunnerFFMCreateCtx>;

impl RunnerFFMCreateCtx {
    pub fn new(
        ffm_type: RunnerFFMType,
        ffmpeg_cmd: FFMpegCmd,
        on_spawn: PFnOnSpawnFFMpeg,
        post_spawn: PFnPostSpawnFFMpeg,
        no_need_termination: bool,
        auto_respawn: bool,
    ) -> Self {
        RunnerFFMCreateCtx {
            ffm_type,
            owner_info: RunnerFFMOwnerInfo {
                app_name: String::new(),
                sess_key: String::new(),
            },
            props: HashMap::new(),

            ffmpeg_cmd,

            on_spawn,
            post_spawn,

            no_need_termination,
            auto_respawn,
        }
    }

    pub fn set_owner_info(&mut self, app_name: String, sess_key: String) {
        self.owner_info = RunnerFFMOwnerInfo { app_name, sess_key };
    }

    pub fn get_owner_info(&self) -> &RunnerFFMOwnerInfo {
        &self.owner_info
    }

    pub fn chk_prop_key(&self, key: &str) -> bool {
        if self.props.get(key).is_some() {
            return true;
        }
        false
    }

    pub fn get_prop(&self, key: &str) -> &str {
        let r = self.props.get_key_value(key);
        if r.is_some() {
            return r.unwrap().1;
        } else {
            return "";
        }
    }

    pub fn set_prop(&mut self, key: &str, val: &str) {
        self.props.insert(key.to_string(), val.to_string());
    }
}

//
// RunnerFFMpeg
//

pub struct RunnerFFMpeg {
    recv_worker_msg_send: RecvWorkerMsgSend,

    runner_msg_send: RunnerFFMpegMsgSend,
    runner_msg_recv: RunnerFFMpegMsgRecv,

    receiver_ctx: Option<RunnerFFMCreateCtx>,
    recorder_ctx: Option<RunnerFFMCreateCtx>,

    inner_handle: Option<EchoJoinHandle<RunnerFFMpegInnerExitRst, RunnerFFMError>>,
    inner_msg_send: Option<RunnerFFMpegInnerMsgSend>,
}

pub fn create_runner_ffmpeg(
    config: Config,
    recv_worker_msg_send: RecvWorkerMsgSend,
    runner_prms: ReqPropReceiverPrm,
    media_prms: PropMedia,
    app_name: String,
    sess_key: String,
    publish_port: u16,
) -> Result<RunnerFFMpeg, RunnerFFMError> {
    imp::instance::new(
        config,
        recv_worker_msg_send,
        runner_prms,
        media_prms,
        app_name,
        sess_key,
        publish_port,
    )
}

// pub async fn spawn_ffmpeg_process(create_ctx: &mut RunnerFFMCreateCtx)
//     -> Result<(), RunnerFFMError> {
//     use crate::runner::ffmpeg::imp::spawn::spawn_ffmpeg;

//     spawn_ffmpeg(create_ctx).await
// }

//
// ffmpeg inner
//

// NoClone or use RunnerFFMProcShared
pub struct RunnerFFMpegInnerProcCtx {
    // readonly
    create_ctx: RunnerFFMCreateCtx,

    no_need_termination: bool,
    auto_respawn: bool,

    is_spawned: bool,
    spawn_err: Option<RunnerFFMError>,

    proc: Option<RunnerFFMProc>,
    proc_cmd: Option<RunnerFFMProcCmd>,

    term_exit_rst: Option<PollExitStRst>,
    term_err: Option<RunnerFFMError>,
}

type RunnerFFMpegInnerProcCtxMap = HashMap<RunnerFFMType, RunnerFFMpegInnerProcCtx>;

pub struct RunnerFFMpegInner {
    responder: RunnerFFMpegMsgSend,

    receiver_type: Option<RunnerFFMType>,
    recorder_type: Option<RunnerFFMType>,

    proc_ctx_map: RunnerFFMpegInnerProcCtxMap,

    // ffmpeg_proc: RunnerProcChild,
    // command: RunnerProcCmd,
    output: LinkedList<String>,

    //    pub hls_out_path: PathBuf,
    // pub stdout: Vec<String>,
    // pub stderr: Vec<String>,
    pub inner_msg_send: RunnerFFMpegInnerMsgSend,
    pub inner_msg_recv: RunnerFFMpegInnerMsgRecv,

    pub force_terminating: bool,
}

pub fn create_runner_ffmpeg_inner(
    responder: RunnerFFMpegMsgSend,
    receiver_type: Option<RunnerFFMType>,
    recorder_type: Option<RunnerFFMType>,
    create_ctxs: RunnerFFMCreateCtxAry,
) -> Result<RunnerFFMpegInner, RunnerFFMError> {
    imp::inner::create_runner_ffmpeg_inner(responder, receiver_type, recorder_type, create_ctxs)
}

#[derive(Debug, Clone)]
pub struct RunnerFFMpegInnerExitRst {
    all_terminated: bool,
    //exit_status: Vec<PollExitStRst>,
    // pub exit_code: i32,
    // pub stdout: String,
    // pub stderr: String,
    // pub message: String,
}

impl RunnerFFMpegInnerExitRst {
    pub fn new() -> Self {
        RunnerFFMpegInnerExitRst {
            all_terminated: true,
            //exit_status: Vec::new(),
        }
    }
}

//
// util
//

pub fn create_ffmpeg_log_file(log_file_path: &str) -> Result<File, RunnerFFMError> {
    let log_file = std::fs::File::create(log_file_path).map_err(|e| {
        RunnerFFMError::FileOperErr(format!(
            "failed to create file {}, e={}",
            log_file_path,
            e.to_string()
        ))
    })?;

    // let stderr_file = std::fs::File::create(self.cmd.get_stderr_file_path()).map_err(|e| {
    //     Error::FileOperErr(format!(
    //         "failed to create file {}, e={}",
    //         self.cmd.get_stderr_file_path(),
    //         e.to_string()
    //     ))
    // })?;

    Ok(log_file)
}

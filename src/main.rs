use crate::comm_media::*;
use git_version::git_version;
use service::vsvr::{message::VSvrServMsgSend, VSvrServ};
use ver::*;

mod splash;
mod ver;

mod comm;
mod comm_fs;
mod comm_media;
mod comm_ps;

mod config;
mod error;
mod message;
//mod receiver;
mod kv;
mod mlog;
mod runner;
mod service;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn get_vsvr_user_agent() -> String {
    format!("VSvr/{} Echo/{}", VERSION.unwrap(), VERSION.unwrap())
}

use {
    crate::{
        comm::*,
        config::Config,
        error::Error,
        error::Error::*,
        message::*,
        runner::{message::*, RecvWorkerManager},
        service::api::serv::ApiServ,
    },
    anyhow::Result,

    public_ip::{dns, http, BoxToResolver, ToResolver},
    serde_json::json,

    std::borrow::Cow,

    // std::sync::Mutex/RwLock : sync lock
    std::{
        collections::HashMap,
        fmt,
        fmt::format,
        net::IpAddr,
        ops::Deref,
        rc::Rc,
        sync::Mutex,
        time::{Duration, Instant},
    },

    thiserror::Error,
    // tokio::sync::Mutex/RwLock : async lock, for async function, await
    tokio::{
        sync::{broadcast, mpsc, oneshot, RwLock},
        task,
        task::JoinHandle,
        time::timeout,
    },
};

use crate::runner::RunnerError;

//use crate::config::Config;
//const GIT_VERSION: &'static str = "test";
const GIT_VERSION: &str = git_version!();

//
// utility
//

async fn resolve_public_ip() -> Option<IpAddr> {
    // List of resolvers to try and get an IP address from
    let resolver = vec![
        BoxToResolver::new(dns::OPENDNS_RESOLVER),
        BoxToResolver::new(http::HTTP_IPIFY_ORG_RESOLVER),
    ]
    .to_resolver();
    public_ip::resolve_address(resolver).await
}

struct MainContext {
    force_exit: bool,

    config: Config,

    //    recv_worker_manager: Arc<RecvWorkerManager>,

    //   main_msg_send : ServMsgSend,   // send message to main
    // main_msg_recv : ServMsgRecv,   // received messages for main

    // api_serv: Arc<ApiServ>,
    async_task_handles: Vec<JoinHandle<()>>,
}

#[actix_rt::main]
async fn main() -> Result<()> {

    let config = match Config::load() {
        Ok(c) => c,
        Err(err) => return Err(err.into()),
    };

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        if args[1] == "-v" {
            println!("{}", GIT_VERSION);
            println!("{:?}", config);
            std::process::exit(0);
        } else {
            println!("usage: echo [-v]");
            std::process::exit(1);
        }
    }

    log4rs::init_file(config.echo_log4rs_file.clone(), Default::default()).unwrap();

    splash::print_splash(ECHO_VER, ECHO_DATE, GIT_VERSION);

    log::info!("config= {:?}", config);

    //
    // create main message channel
    //

    let (main_msg_send, main_msg_recv) = mpsc::unbounded_channel();

    //
    // create managers
    //

    let svr_public_ip: EchoPublishIp = resolve_public_ip().await;
    let svr_publish_ip_str = EchoPublishIpStr!(svr_public_ip);

    //
    // vsvr service
    //

    let vsvr_serv_inst = VSvrServ::new(main_msg_send.clone(), config.clone()).unwrap();

    let vsvr_serv_msg_send = vsvr_serv_inst.get_msg_sender_ref().clone();

    tokio::spawn(vsvr_serv_inst.run());

    //
    // RecvWorkerManager
    //

    let worker_man_inst = RecvWorkerManager::new(
        main_msg_send.clone(),
        vsvr_serv_msg_send.clone(),
        config.clone(),
        svr_publish_ip_str.clone(),
    );

    //let worker_man_msg_send = worker_man_inst.get_msg_sender_ref().clone();
    let worker_man_msg_send = worker_man_inst.get_msg_send_ref().clone();

    // ownership is moved to RecvWorkerManager::run()
    tokio::spawn(worker_man_inst.run());

    //
    // build main context
    //

    let mut main_ctx = MainContext {
        force_exit: false,

        config,

        //  recv_worker_manager,

        //main_msg_send,
        //main_msg_recv,

        // api_serv,
        async_task_handles: Vec::new(),
    };

    //
    // running
    //

    //main_ctx.async_task_handles.push(
    tokio::spawn(run_message_handler(
        main_msg_recv, // passover ownership,
        worker_man_msg_send.clone(),
        svr_publish_ip_str,
        vsvr_serv_msg_send.clone(),
    ));
    //);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut api_serv = ApiServ::new(main_msg_send.clone(), main_ctx.config.clone())
        .run()
        .await?;

    //
    // cleanup
    //

    // await handles
    // XXX graceful shutdown
    // wait for all spawned processes to complete
    for handle in main_ctx.async_task_handles {
        handle.await?;
    }

    Ok(())
}

pub async fn run_message_handler(
    mut main_msg_recv: ServMsgRecv,
    worker_man_msg_send: RecvWorkerManagerMsgSend,
    svr_publish_ip_str: String,
    vsvr_serv_msg_send: VSvrServMsgSend,
) -> Result<()> {
    let mut worker_man_msg_send = worker_man_msg_send.clone();

    log::debug!("[ServMsgHandler] starting..");

    loop {
        let msg = main_msg_recv.recv().await;

        let msg_proc_status: Result<(), Error> = match msg {
            Some(ServMsg::TerminateRecvWorker(responder, prm_type, app_name, sess_key)) => {
                log::debug!(
                    "[ServMsg::TerminateWorker] terminating receive worker..,\
                    prm_type={}, app_name={}, sess_key={}",
                    prm_type,
                    app_name,
                    sess_key
                );

                let (man_term_responder, man_term_recv) = tokio::sync::oneshot::channel();

                worker_man_msg_send
                    .send(RecvWorkerManagerMsg::TerminateRecvWorker(
                        man_term_responder,
                        prm_type,
                        app_name.clone(),
                        sess_key,
                    ))
                    .map_err(|e| {
                        MsgChanErrSendFail(format!(
                            "{},e={}",
                            "RecvWorkerManager,TerminateRecvWorker", e
                        ))
                    })?;

                let man_term_rst = man_term_recv.await.map_err(|e| {
                    MsgChanErrRecvFail(format!(
                        "{},e={:?}",
                        "RecvWorkerManager,TerminateRecvWorker", e
                    ))
                })?;

                let resp = match man_term_rst {
                    Err(RunnerError::FailedToTerminateRecvWorker(e)) => {
                        Err(Error::MsgChanRstErrJson(e.to_string()))
                    }
                    Err(e) => Err(error::Error::MsgChanRstErrJson(e.to_string())),
                    Ok(_) => {
                        // successfully, enqueued terminate worker message
                        Ok(json!({ "name": app_name }))
                    }
                };

                responder.send(resp).map_err(|e| {
                    MsgChanErrSendFail(format!("{},e={:?}", "MainServ,TerminateRecvWorker", e))
                })?;

                Ok(())
            }

            Some(ServMsg::SpawnRecvWorker(responder, req_publish)) => {
                log::debug!(
                    "[ServMsg][SpawnRecvWorker] creating receive worker.., {:?}",
                    req_publish
                );

                let (spawn_responder, spawn_rst) = tokio::sync::oneshot::channel();

                worker_man_msg_send
                    .send(RecvWorkerManagerMsg::SpawnRecvWorker(
                        spawn_responder,
                        req_publish.clone(),
                    ))
                    .map_err(|e| MsgChanErrSendFail(format!("{},e={}", "SpawnRecvWorker", e)))?;

                let worker_info = spawn_rst
                    .await
                    .map_err(|e| MsgChanErrRecvFail(format!("{},e={:?}", "SpawnRecvWorker", e)))?;

                let spawn_rst = match worker_info {
                    Err(RunnerError::FailedToCreateRecvWorker(e)) => {
                        Err(Error::MsgChanRstErrJson(e))
                    }
                    Err(e) => Err(error::Error::MsgChanRstErrJson(e.to_string())),
                    Ok((publish_ip, publish_port, _)) => {
                        // successfully created receive worker
                        let app_name = req_publish.app_name.clone();
                        let echo_api_sess_key = req_publish.sess_key.clone();

                        //let publish_ip = svr_publish_ip_str.clone();
                        //let publish_port : u16 = 30000;

                        let prop_transport = PropTransport {
                            ip_type: format!("IPv4"),
                            address: publish_ip.clone(),
                            port: publish_port,
                        };

                        let propReciver = PropReceiver {
                            receiver_type: MediaReceiver::FFMPEG,
                        };

                        let prop_rtmp = PropRtmp {
                            url: format!(
                                "rtmp://{}:{}/{}",
                                publish_ip.clone(),
                                publish_port,
                                app_name
                            ),
                            name: echo_api_sess_key.clone(),
                        };

                        let mut response =
                            crate::service::api::reqres::publish::res::gen_response_ok(
                                // format!("success"),
                                app_name,
                                // echo_api_sess_key.clone(),
                                prop_transport,
                                req_publish.media,
                                prop_rtmp,
                                //  propReciver,
                            );

                        Ok(response)
                    }
                };

                responder
                    .send(spawn_rst)
                    .map_err(|e| MsgChanErrSendFail(format!("{},e={:?}", "SpawnRecvWorker", e)))?;

                Ok(())
                //Err(Error::MsgChanRstErr(format!("error-test")))
            }

            Some(ServMsg::GetVSvrServSender(responder)) => {
                responder
                    .send(Ok(vsvr_serv_msg_send.clone()))
                    .map_err(|e| {
                        MsgChanErrSendFail(format!("{},e={:?}", "GetVSvrServSender", e))
                    })?;

                Ok(())
            }

            Some(_) => Ok(()),
            None => {
                log::error!("internal error: main_event");
                Ok(())
            }
        };

        if let Err(e) = msg_proc_status {
            log::error!("internal error: {:?}", e);
            panic!("internal error: {:?}", e);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        //tokio::task::yield_now().await;
    }

    Ok(())
}

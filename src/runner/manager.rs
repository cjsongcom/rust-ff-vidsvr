use super::RunnerError;
use execute::Execute;
use anyhow::Result;
use crate::service::vsvr::util;
use crate::service::{api::reqres::publish::req::ReqPublishV3, vsvr::message::VSvrServMsgSend};

use {
    super::{ message::*, worker::RecvWorker},

    crate::{
        comm::*, 
        error::Error, 
        message::ServMsgSend,
        config::Config,
    },

    serde::{Deserialize, Serialize},
    serde_json::json,

    // std::sync::Mutex/RwLock : no await
    std::{
        collections::{ HashMap, VecDeque },
        sync::{Arc},
        time::{Duration, Instant},
    },

    // tokio::sync::Mutex/RwLock : for async function, await
    tokio::{
        sync::mpsc,
        task::JoinHandle,
    },

    
};

use crate::ECHO_ASYNC_SLEEP_MS;

use super::RecvWorkerType;

use super::{
    RecvWorkerManager,
    RecvWorkerHandle,
    RecvWorkerManagerStatus,    
};

impl RecvWorkerManager {

    fn peek_message(&mut self) 
        -> Result<RecvWorkerManagerMsg, mpsc::error::TryRecvError> {
        self.msg_received.try_recv()
    }

    async fn handle_msg_spawn_recv_worker(&mut self,
        responder:SpawnRecvWorkerMsgOCResponder,
        req_publish:ReqPublishV3) -> Result<(), RunnerError> {

        log::debug!("[RecvWorkerManager] creating receiver worker, {:?}", req_publish);

        if self.status != RecvWorkerManagerStatus::Running {
            return Err(RunnerError::InvalidManagerStatus(
                json!({ "error" : format!("RecvWorker Manager not ready.., status={:?}", 
                    self.status) })));
        }

        //
        // find previous worker by app_name 
        //

        let chk_app_name = req_publish.app_name.as_str();

        let prev_recv_worker 
            = self.worker_handles.get(chk_app_name);
        
        if prev_recv_worker.is_some() {
            log::debug!("[RecvWorkerManager] found previous receiver worker,\
                respawning worker.. app_name={}", chk_app_name);                

            let _prev = prev_recv_worker.unwrap();

            let mut prev_publish_info = None;

            // lock - read
            { 
                let prev_worker_handle 
                    = _prev.read().await;

                    prev_publish_info = Some((
                        prev_worker_handle.publish_ip.clone(),
                        prev_worker_handle.publish_port,
                        prev_worker_handle.uuid.clone()));
            }

            if let Some(resp) = prev_publish_info {                
                responder
                    .send(Ok(resp))
                    .unwrap();

                // send terminate signal to ffmpeg

                // await , give chance to ffmpeg to respawn
                ECHO_ASYNC_SLEEP_MS!(1000);

                return Ok(());

            } else {
                let _em = format!("previos worker founded, but worker handle\
                 info is invalid, app_name={}", chk_app_name);

                return Err(RunnerError::RecvWorkerHandleErr(_em));
            }
        }

        ////////////////////////////////////////////////////////////////////////////

        let publish_ip
            = self.get_publish_ip().to_string();

        let publish_port 
            = self.pick_publish_port()
                .await
                .map_err(|e| {
                    let _em = format!("can't pick publish port,\
                     app_name={}\
                     f=handle_msg_spawn_recv_worker,
                     e={}", 
                     chk_app_name,
                     e.to_string());

                    RunnerError::RunnerOperErr(_em)
                })?;

        let worker_uuid = EchoUUID_new().to_string();

        match self.spawn_worker(
            self.config.clone(),
            worker_uuid.clone(),
            req_publish,
            publish_ip.as_str(), 
            publish_port).await {

            Ok((handle, mut worker)) => {
                let rst = worker.begin().await;
                
                let response = match rst {
                    Ok(r) => {
                        let worker_joinhandle 
                            = tokio::spawn(worker.run());

                        handle.write().await.worker_joinhandle = Some(worker_joinhandle);
                        Ok((publish_ip, publish_port, worker_uuid))
                    },

                    Err(e) => {
                        Err(RunnerError::FailedToCreateRecvWorker(
                            format!("{}", e.to_string())))
                    }
                };

                responder
                    .send(response)
                    .unwrap();                
            },

            Err(e) =>{
                responder
                    .send(Err(e))
                    .unwrap();
            }
        }

        Ok(())    
    }


    async fn handle_msg_terminate_recv_worker(&mut self, 
        prm_type:TerminateRecvWorkerPrmType,
        app_name:String,
        sess_key:String) 
        -> Result<PrmJson, RunnerError> {
        
         // find worker handle by app_name
         let handle 
            = self.worker_handles.get(&app_name);

        if handle.is_none() {
            return Err(RunnerError::RunnerOperErr(
                format!("inner-error-route, f={},e={}",
                    "handle_msg_terminate_recv_worker", 
                    "can't find worker handle by given appname")));
        }

        // send finish message to receive worker
        let mut worker_msg_send: Option::<RecvWorkerMsgSend> = None;

        { // rwlock.write
            worker_msg_send 
                = Some(handle.unwrap()
                    .write()
                    .await
                    .worker_msg_send.clone());
        }
        
        // let _handle2 
        //     = &handle.unwrap().as_ref().write().unwrap();

        let (fin_responder,
             fin_recv)
            = tokio::sync::oneshot::channel();

        worker_msg_send
            .unwrap()
            .send(RecvWorkerMsg::FinishRecvWorker(fin_responder, json!({})))
            .map_err(|e|
                RunnerError::RunnerMsgChanErr(
                    "send_fail", 
                    "RecvWorkerMsg::FinishRecvWorker",
                    "handle_msg_terminate_recv_worker",
                    e.to_string()))?;

        let fin_rst 
            = fin_recv
                .await
                .map_err(|e|
                    RunnerError::RunnerMsgChanErr(
                        "recv_rst_fail", 
                        "RecvWorkerMsg::FinishRecvWorker",
                        "handle_msg_terminate_recv_worker",
                        e.to_string())
                )?;

        // restore publish port

        Ok(fin_rst.unwrap())

    }


    async fn proc_msg(&mut self) -> Result<(), RunnerError> {

        match self.peek_message() {

            Ok(worker_manager_msg) => match worker_manager_msg {

                /////////////////////////////////////////////////
                RecvWorkerManagerMsg::NotifyRecvWorkerIsExiting(
                    worker_uuid,
                    app_name,
                    sess_key ) => {
                    log::debug!("[RecvWorkerManager] got msg 'NotifyWorkerIsExiting', worker_uuid={:?}", worker_uuid);


                    if self.config.vsvr_api_use_pub_unpub {

                        log::debug!("[RecvWorkerManager::proc_msg] changing vsvr state to unpublish, \
                        app_name={}, sess_key={}", app_name, sess_key);
        
                        // don't wait response
                        util::update_vsvr_publish_state(
                            self.vsvr_serv_msg_send.clone(),
                            app_name,
                            sess_key,
                            crate::service::vsvr::message::VSvrLiveState::UnPublish)
                        .await
                        .map_err(|e| {
                            let em=format!("f=RecvWorkerManager::proc_msg, e={}", e.to_string());
                            log::error!("{}", em);
                            RunnerError::RunnerOperErr(em)                            
                        })?;

                    } else {
                        log::debug!("[RecvWorkerManager::proc_msg] skipped changing vsvr \
                        state to unpublish by config, app_name={}, sess_key={}",
                            app_name, sess_key);

                    }
                },

                /////////////////////////////////////////////////
                RecvWorkerManagerMsg::SpawnRecvWorker(                    
                    responder, 
                    req_publish) => {

                    self.handle_msg_spawn_recv_worker(responder, req_publish).await?
                },

                /////////////////////////////////////////////////
                RecvWorkerManagerMsg::TerminateRecvWorker(
                    responder,
                    prm_type, app_name, sess_key) => {

                    let resp 
                        = self.handle_msg_terminate_recv_worker(
                            prm_type, app_name, sess_key).await;                                               
                    
                    responder
                        .send(resp)
                        .map_err(|e|{
                            RunnerError::RunnerMsgChanErr(
                                "response", 
                                "RecvWorkerManagerMsg::TerminateRecvWorker",
                                "RecvWorkerManager::proc_msg",
                                format!("{:?}",e))
                        })?;
                },

                _ => {}
            },

            Err(e) => match e {
                mpsc::error::TryRecvError::Disconnected => {
                //mpsc::error::TryRecvError::Closed => {                    
                    return Err(
                        RunnerError::MsgChanErrChannelClosed(format!("[RecvWorkerManager::proc_msg] e={}", e.to_string())));
                    // message channel is closed
                    //log::debug!("[RecvWorkerManagerMsg] message channel to manager is closed.. exiting..");
                    //break;
                }

                _ => {
                    // mpsc::error::TryRecvError::Empty
                    // return Err(Error::MsgChanErrRecvMsgChannelIsEmpty(
                    //     "self.peek_message()".to_string(), "[RecvWorkerManager::proc_msg]".to_string()));
                }
            },
        }

        Ok(())
    }

    pub fn new(
        main_serv_msg_send: ServMsgSend, 
        vsvr_serv_msg_send: VSvrServMsgSend,
        config: Config,
        publish_ip: EchoIpStr,                
        ) -> Self {
                
        let (sender, 
            receiver) = mpsc::unbounded_channel();

        let avail_publish_ports 
            = Arc::new(EchoAsyncRwLock::new(
                (config.echo_publish_min_port..= 
                    config.echo_publish_max_port).collect()));

        let inst = Self {
            force_exit: false,

            config: config.clone(),

            status: RecvWorkerManagerStatus::Creating,

            msg_received: receiver,
            msg_send: sender,

            main_serv_msg_send,
            vsvr_serv_msg_send,

            // key: app_name, value: worker
            worker_handles: HashMap::new(),

            is_destroyed: false,
            
            publish_ip: publish_ip,

            // config.echo_publish_min_port ~ // config.echo_publish_max_port
            avail_publish_ports : avail_publish_ports,
        };

        log::info!("Created RecvWorkerManager..");

        inst
    }

    pub fn destroy(&mut self) {
        if self.status == RecvWorkerManagerStatus::Destroying {
            return;
        }

        log::info!("Destroying RecvWorkerManager..");

        self.is_destroyed = true;
    }

    async fn pop_publish_port(&mut self)->Result<EchoPublishPort, Error> {

        let mut ports 
                //= self.avail_publish_ports.write().await;
                = self.avail_publish_ports.write().await;

        // if ports.is
        //             .map_err(|e| 
        //                 Error::LockAcquireFailed(format!("avail_publish_ports,{:?}",e)))?;
                                
        if ports.is_empty() {
            log::error!("no available publish port");
            return Err(Error::PublishPortNotAvailable);
        }

        let port = ports.pop_front();
            
        if port.is_none() {
            return Err(Error::PublishPortNotAvailable);
        }

        Ok(port.unwrap())
    }

    fn get_publish_ip(&self) -> &str {
        self.publish_ip.as_str()        
    }

    async fn pick_publish_port(&mut self) 
        ->Result<EchoPublishPort, Error> {

        let mut try_count = 0;
        let mut publish_port = 0;

        let max_try_count = 100;

        while try_count < max_try_count {

            publish_port 
                = self.pop_publish_port().await?;

            // check port is already in use            
            {
                let rst = std::process::Command::new("bash")
                        .arg("-c")
                        .arg(format!("lsof -n -i TCP:{} | grep LISTEN", publish_port))
                        .execute_output();

                if rst.is_err() {
                    // can't execute 'lsof', skip port check
                    return Ok(publish_port);
                }

                if let Some(r) = rst.unwrap().status.code() {

                    if r != 0 {  // port not in use
                        return Ok(publish_port); 
                    }
                    
                    // port is already in use, continue 

                } else {
                    // error in execute 'lsof', skip port check
                    return Ok(publish_port);
                }
            }

            try_count += 1;
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        log::error!("too many check port in use, exceed {} times, last finding port={}", 
            max_try_count, publish_port);

        // too many port check, skip port check
        return Ok(publish_port);
        
    }

    pub fn get_msg_send_ref(&self) -> &RecvWorkerManagerMsgSend {
        &self.msg_send
    }
   
    pub async fn spawn_worker(&mut self,         
        config: Config,
        worker_uuid: String,
        req_publish: ReqPublishV3,        
        publish_ip : &str,
        publish_port: u16,
    )->Result<(Arc<EchoAsyncRwLock<RecvWorkerHandle>>, RecvWorker), RunnerError> {

        match RecvWorker::new(
            config.clone(),
            worker_uuid.clone(), 
            self.msg_send.clone(),
            self.main_serv_msg_send.clone(),
            self.vsvr_serv_msg_send.clone(),
            req_publish.clone(),
            publish_port,
            config.echo_publish_max_duration
        ) {
            Ok(worker) => {
                let worker_handle_uuid = EchoUUID_new();
                let worker_handle
                     = Arc::new(EchoAsyncRwLock::new(
                        RecvWorkerHandle {
                            uuid: worker_handle_uuid.to_string(),
                            worker_uuid: worker_uuid.clone(),
                            worker_msg_send: worker.clone_msg_send(),                        
                            worker_joinhandle: None,   // option<JoinHandle..>
                            app_name: req_publish.app_name.clone(),
                            sess_key: req_publish.sess_key.clone(),
                            publish_ip: publish_ip.to_string(),
                            publish_port: publish_port,                        
                        }));

                self.worker_handles.insert(
                    req_publish.app_name.clone(), 
                    worker_handle.clone());
                
                Ok((worker_handle.clone(), worker))
            },

            Err(e) => {
                let _em = format!("failed to create recv worker,\
                    f=RecvWorkerManager::spawn_worker, e={}", e.to_string());

                Err(RunnerError::RunnerOperErr(_em.to_string()))
            },
        }

    }

    pub async fn run(mut self) {
        log::debug!("[RecvWorkerManager::run] starting..");

        self.status = RecvWorkerManagerStatus::Running;

        loop {
            match self.proc_msg().await {
                Err(RunnerError::RunnerMsgChanErr(_e0, _e1, _e2, _e3)) => {
                    log::error!("[RecvWorkerManager::run] receive msg channel err.., force exiting..: e={0},{1},{2},{3}",
                        _e0, _e1, _e2, _e3);
                    self.force_exit = true;

                    break;
                },
                Err(e) => {
                    log::error!("[RecvWorkerManager::run] error occured., force exiting..: e={:?}", e);
                    self.force_exit = true;

                    break;
                },
                Ok(_) => {}               
            }
            
            if (self.force_exit) {
                log::debug!("[RecvWorkerManager::run] force exiting..");
                break;

            } else {
                // prevent blocking thread, do not block other task is beging drive
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

        }
    }

}

impl Drop for RecvWorkerManager {
    fn drop(&mut self) {
        log::debug!("[RecvWorkerManager] drop called");
        self.destroy()
    }
}

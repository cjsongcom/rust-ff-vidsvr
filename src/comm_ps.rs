// comm_ps (process)
// > process utility
use crate::{
    comm::{EchoTimeDuration, EchoTimeInstant},
    ECHO_ASYNC_SLEEP_MS,
};
use thiserror::Error;
pub type EchoPSCmd = tokio::process::Command;
pub type EchoPSChild = tokio::process::Child;
pub type EchoPSChildStdErr = tokio::process::ChildStderr;
pub type EchoPSExitCode = i32;
pub const PROC_EXIT_CODE_INTERRUPTED: i32 = 999;

//
// error
//
// Error=thiserror_impl proc_macro Error
#[derive(Error, Debug)]
pub enum EchoPSError {
    //
    // spawn process
    //
    #[error("error on spawn process={0}")]
    SpawnErr(String),

    //
    // pollng process status
    //
    #[error("error on polling process status={0}")]
    PollingStatusErr(String),

    #[error("")]
    PollingStatusTimeout,

    //
    // terminate process
    //
    #[error("error on terminate process={0}")]
    TerminateErr(String),
}

//
// spawn process
//

//
// terminate
//

pub type EchoPSID = u32;

pub fn terminate(pid: EchoPSID) -> Result<EchoPSChild, EchoPSError> {
    // SIGHUP	1	Hangup
    // SIGINT	2	Interrupt from keyboard
    // SIGKILL	9	Kill signal (never graceful)
    // SIGTERM	15	gracefully termination, equal to kill -s SIGTERM or kill -15
    // SIGSTOP	17,19,23  Stop the process (pause running process)
    // SIGCONT <PID>: resume paused ffmpeg again

    // SIGKILL: $ kill -9 {PID}  or kill -s SIGKILL
    // SIGTERM: $ kill {PID}
    let cmd_kill = EchoPSCmd::new("kill")
        .args([pid.to_string().as_str()])
        .spawn()
        .map_err(|e| {
            // proc.start_kill()  // non-block, kill -9
            EchoPSError::TerminateErr(format!(
                "failed to execute 'kill', pid={}, e={}",
                pid,
                e.to_string()
            ))
        })?;

    // cmd_kill.try_wait()
    // cmd_kill.wait()          ; async wait
    // cmd_kill.try_wait()      ; nonblock wait

    Ok(cmd_kill)
}

//
// polling process
//

#[derive(Clone)]
pub struct PollExitStRst {
    pub exit_code: EchoPSExitCode,
    pub exit_desc: String,
}

pub async fn poll_exit_status(
    proc: &mut EchoPSChild,
    timeout_ms: EchoTimeDuration,
) -> Result<PollExitStRst, EchoPSError> {
    let mut exit_rst = Option::<PollExitStRst>::None;

    let en_time_ms = EchoTimeInstant::now() + timeout_ms;

    loop {
        let mut _wait_rst = proc.try_wait();

        match _wait_rst {
            Ok(wait_rst) => {
                if let Some(exit_status) = wait_rst {
                    let mut exit_code = 0;

                    // process is terminated
                    let msg = match exit_status.code() {
                        Some(0) => {
                            exit_code = 0;
                            format!("exit with normally")
                        }
                        Some(1) => {
                            exit_code = 1;
                            format!(
                                "exit with errno=1, may port already in use or SIGINT(ctrl+c) ?"
                            )
                        }
                        Some(e) => {
                            exit_code = e;
                            format!("exit with errno={}", e)
                        }
                        None => {
                            exit_code = PROC_EXIT_CODE_INTERRUPTED;
                            let em = format!(
                                "process is terminated by external \
                             ,(ex: kill -9 pid, killall -9 ffmpeg)"
                            );
                            log::error!("{}", em);
                            em
                        }
                    };

                    exit_rst = Some(PollExitStRst {
                        exit_code,
                        exit_desc: msg,
                    });
                }
            } //-end-of-Ok(wait_rst),

            Err(e) => {
                // try_wait() error
                //exit_rst = Some(self.on_exit(false, PROC_EXIT_CODE_INTERRUPTED, e.to_string()));
                return Err(EchoPSError::PollingStatusErr(format!(
                    "error on polling, e={}",
                    e.to_string()
                )));
            }
        }

        if exit_rst.is_some() {
            break;
        }

        let now_ms = EchoTimeInstant::now();

        if now_ms > en_time_ms {
            // termination signal was sented, but process is still running
            return Err(EchoPSError::PollingStatusTimeout);
        }

        ECHO_ASYNC_SLEEP_MS!(10);
    }

    Ok(exit_rst.unwrap())
}

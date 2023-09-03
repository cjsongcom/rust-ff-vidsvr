use crate::comm::{EchoTimeDuration, EchoTimeInstant};
use crate::comm_ps;
use crate::runner::ffmpeg::error::RunnerFFMError;
use crate::runner::ffmpeg::*;
use crate::ECHO_ASYNC_SLEEP_MS;

pub async fn spawn_ffmpeg_from_creation_ctx(
    creation_ctx: &mut RunnerFFMCreateCtx,
    spawn_retry_max_time_ms: EchoTimeDuration, // 10 second
) -> Result<(RunnerFFMProc, RunnerFFMProcCmd), RunnerFFMError> {
    //let mut spawn_retry_max_time_ms = EchoTimeDuration::from_millis(10000); // 10 second
    let mut spawn_start_time_ms = EchoTimeInstant::now();

    // let (PFnOnSpawnFFMpeg, PFnPostSpawnFFMpeg) =
    //     ffmpeg::get_spawn_callback_by_ffm_type(creation_ctx.runner_ffm_name)?;

    //
    // on_spawn_ffmpeg
    //

    let _spawn_rst = (creation_ctx.on_spawn)(&creation_ctx); // sync

    ECHO_ASYNC_SLEEP_MS!(100);

    if let Err(e) = _spawn_rst {
        return Err(RunnerFFMError::FFMpegSpawnErr(format!(
            "fail on OnSpawnFFMepg, e={}",
            e.to_string()
        )));
    }

    let (mut ffmpeg_proc, mut ffmpeg_proc_cmd) = _spawn_rst.unwrap();

    loop {
        let wait_rst = ffmpeg_proc.try_wait().unwrap();

        if wait_rst.is_none() {
            //
            // ffmpeg is spawned, and running
            //

            // match proc.stdout.as_mut() {
            //     Some(out) => {
            //         let r = read_from_std(out);
            //         if let Ok(o) = r.await {
            //             log::debug!("[ffmpeg][stdout-1] {}", o);
            //         }
            //     }
            //     _ => {}
            // };

            // check.fix
            // match proc.stderr.as_mut() {
            //     Some(out) => {
            //         let r = read_from_std(out);

            //         if let Ok(o) = r.await {
            //             log::debug!("[ffmpeg][stderr-1] {}", o);
            //         }
            //     }
            //     _ => {}
            // };

            log::debug!("[RunnerFFMpeg::spawn_ffmpeg] continue with 'None' ");

            break;
        } else {
            //
            // 1. ffmpeg is exited in spawning by error
            // >  port is already used / invalid ffmpeg option
            //
            // 2. ffmpeg is still spawning
            //

            let exit_status = wait_rst.unwrap();

            // program exited with normally(0), or error(1~)
            let mut output_err = vec![String::from("")];
            let mut exit_code = comm_ps::PROC_EXIT_CODE_INTERRUPTED;

            if (exit_status.code().is_some()) {
                exit_code = exit_status.code().unwrap();
            } else {
                // still spawning, so exit_code is not avaialble
                if EchoTimeInstant::now().duration_since(spawn_start_time_ms)
                    > spawn_retry_max_time_ms
                {
                    let err_msg = format!("exceed max ffmpeg spawning time..").to_string();
                    log::error!("{}", err_msg);

                    return Err(RunnerFFMError::FFMpegSpawnErr(
                        format!("{}", err_msg).to_string(),
                    ));
                }

                // give chance to poll spawning
                //tokio::time::sleep(EchoTimeDuration::from_millis(100)).await;
                ECHO_ASYNC_SLEEP_MS!(100);

                continue;
            }

            // match proc.stdout.as_mut() {
            //     Some(out) => {
            //         let r = read_from_std(out);

            //         if let Ok(o) = r.await {
            //             log::debug!("[ffmpeg][stdout-2] {}", o);
            //         }
            //     }
            //     _ => {}
            // };

            let _em_spawn = format!("just exited, code={},{}", exit_code, output_err.join("\n"));
            log::error!("{}", _em_spawn);

            return Err(RunnerFFMError::FFMpegSpawnErr(_em_spawn));
            // return Err(Error::FFMpegSpawnErr(
            //     format!("error attempting to wait,{e}")));
        }
    } // end of loop

    //proc = Some(EchoArc::new(EchoAsyncRwLock::new(ffmpeg_proc)));
    //proc_cmd = Some(EchoArc::new(EchoAsyncRwLock::new(ffmpeg_proc_cmd)));

    //
    // after spawn ffmpeg
    //

    let _post_spawn_rst = (creation_ctx.post_spawn)(&creation_ctx);

    if let Err(e) = _post_spawn_rst {
        return Err(RunnerFFMError::FFMpegSpawnErr(format!(
            "fail on OnPostSpawnFFMepg, e={}",
            e.to_string()
        )));
    }

    //
    // fin
    //

    Ok((ffmpeg_proc, ffmpeg_proc_cmd))
}

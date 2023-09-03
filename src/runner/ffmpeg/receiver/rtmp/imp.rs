//
// runner::ffmpeg::receiver::rtmp
//
use crate::comm_fs;
use crate::comm_media::PropMedia;
use crate::runner::ffmpeg::imp::prerole;
use crate::runner::ffmpeg::{
    create_ffmpeg_log_file, FFMpegCmd, RstOnSpawnFFMpeg, RstPostSpawnFFMpeg, RunnerFFMCreateCtx,
    RunnerFFMError, RunnerFFMType,
};
use crate::runner::RunnerProcCmd;
use std::result::Result;

pub fn new_ffm_cmd(
    media_props: PropMedia,

    app_name: String,
    sess_key: String,
    listen_ip: String,
    publish_port: u16,

    hls_out_path: String,

    ffmpeg_path: String,
    ffmpeg_args_given: String,
    ffmpeg_log_file_path: String,
    ffmpeg_verbose: String,

    ffmpeg_overwrite: bool,
    ffmpeg_rw_timeout: i64,
    ffmpeg_vcodec: String,
    ffmpeg_acodec: String,

    ffmpeg_hls_init_time: String,
    ffmpeg_hls_time: String,
    ffmpeg_hls_list_size: i32,
) -> Result<FFMpegCmd, RunnerFFMError> {
    let rtmp_receiver_cmd = FFMpegCmd::new_as_rtmp_receiver(
        media_props,
        app_name,
        sess_key,
        listen_ip,
        publish_port,
        hls_out_path,
        ffmpeg_path,
        ffmpeg_args_given,
        ffmpeg_log_file_path,
        ffmpeg_verbose,
        ffmpeg_overwrite,
        ffmpeg_rw_timeout,
        ffmpeg_vcodec,
        ffmpeg_acodec,
        ffmpeg_hls_init_time,
        ffmpeg_hls_time,
        ffmpeg_hls_list_size,
    )
    .map_err(|e| {
        RunnerFFMError::OperErr(format!(
            "failed to create ffmpeg cmd for rtmp receiver, \
                    f=ffmpeg:imp::new, e={}",
            e.to_string()
        ))
    })?;

    Ok(rtmp_receiver_cmd)
}

//Box::new(move |prm| Box::pin(on_spawn_ffmpeg(prm))),
//Box::new(move |prm| Box::pin(post_spawning_ffmpeg(prm))),
pub fn new_ffm_create_ctx(cmd: FFMpegCmd) -> RunnerFFMCreateCtx {
    RunnerFFMCreateCtx::new(
        RunnerFFMType::RUNNER_FFM_RECEVIER("RTMP"),
        cmd,
        on_spawn_ffmpeg,
        post_spawn_ffmpeg,
        false,
        true,
    )
}

// pub fn get_spawn_callbacks() -> (PFnOnSpawnFFMpeg, PFnPostSpawnFFMpeg) {
//     return (
//         Box::new(move |prm| Box::pin(on_spawn_ffmpeg(prm))),
//         Box::new(move |prm| Box::pin(post_spawning_ffmpeg(prm))),
//     );
// }

//
// spawn event callbacks (do not use async function)
//

fn on_spawn_ffmpeg(ctx: &RunnerFFMCreateCtx) -> Result<RstOnSpawnFFMpeg, RunnerFFMError> {
    assert!(ctx.chk_prop_key("hls_out_path"));
    assert!(ctx.chk_prop_key("hls_prerole_path"));
    assert!(ctx.chk_prop_key("log_file_path"));

    log::debug!(
        "[ffmpeg::receiver::rtmp] spawn receiver ffmpeg  cmd={}",
        ctx.ffmpeg_cmd.get_ffmpeg_args().clone().join(" ")
    );

    let mut spawner = RunnerProcCmd::new(ctx.ffmpeg_cmd.get_program().clone());

    let hls_out_path = ctx.get_prop("hls_out_path");

    if let Err(e) = comm_fs::create_dir_sync(&hls_out_path) {
        let _em = format!(
            "[ffmpeg::receiver::rtmp] failed to create hls out root path={}, e={}",
            hls_out_path,
            e.to_string()
        );
        return Err(RunnerFFMError::FFMpegSpawnErr(_em));
    }

    //self.command.env(key, val) // env value
    let log_file_path = ctx.get_prop("log_file_path");

    match create_ffmpeg_log_file(log_file_path) {
        Ok(ffmpeg_log_file) => {
            // command.stdout(Stdio::piped());
            // command.stderr(Stdio::piped());
            //command.stdout(stdout_file);
            // ffmpeg generate log via stderr
            spawner.stderr(ffmpeg_log_file);
        }
        Err(e) => {
            let _em = format!(
                "[ffmpeg::receiver::rtmp] failed to create receiver ffmpeg log files.., path={}, e={}",
                log_file_path,
                e.to_string()
            );
            return Err(RunnerFFMError::FFMpegSpawnErr(_em));
        }
    }

    //
    // spawn ffmpeg rtmp receiver process
    //

    spawner.current_dir(hls_out_path.clone());
    spawner.args(ctx.ffmpeg_cmd.get_ffmpeg_args().clone());
    //self.cmd.print_ffmpeg_args();

    /*
        let ff = command.execute_output();

        let output = ff.unwrap();

        if let Some(exit_code) = output.status.code() {
            if exit_code == 0 {
                log::debug!("Ok.");
            } else {
                log::debug!("Failed.");
            }
        } else {
            log::debug!("Interrupted!");
        }

        log::debug!("{}", String::from_utf8(output.stdout).unwrap());
        log::debug!("{}", String::from_utf8(output.stderr).unwrap());
    */

    //let ffmpeg_proc = Box::new(command.spawn());
    let ffmpeg_proc = spawner.spawn();

    // give spawning time, 100ms
    //tokio::time::sleep(Duration::from_millis(100)).await;
    //ECHO_ASYNC_SLEEP_MS!(100);

    if let Err(e) = ffmpeg_proc {
        return Err(RunnerFFMError::FFMpegSpawnErr(e.to_string()));
    }

    //cjfix, unwrap -> handle Error
    let mut proc = ffmpeg_proc.unwrap();

    // for std::process::command
    // match proc.try_wait() {
    //     Ok(Some(status)) => {
    //         // ownership(stderr) is moved into 'get_child_stderr_output'
    //         let (stderr, output)
    //             = get_child_stderr_output(proc.stderr);

    //         proc.stderr = stderr; // restore ownership (stderr)

    //         // program exited with normally(0), or error(1~)
    //         return Err(Error::FFMpegSpawnErr(
    //             format!("just exited, status={status},{output}")));
    //     },
    //     Ok(None) => { // running
    //     },
    //     Err(e) => {
    //       return Err(Error::FFMpegSpawnErr(
    //           format!("error attempting to wait,{e}")));
    //     }
    // }

    Ok((proc, spawner))
}

pub fn post_spawn_ffmpeg(ctx: &RunnerFFMCreateCtx) -> Result<RstPostSpawnFFMpeg, RunnerFFMError> {
    let prerole_src_path = ctx.get_prop("hls_prerole_path");
    let prerole_out_path = format!("{}", ctx.get_prop("hls_out_path"));

    // copy prerole file
    let rst = prerole::generate_prerole_file_sync(
        prerole_src_path.to_string(),
        prerole_out_path.to_string(),
    );

    Ok(())
}

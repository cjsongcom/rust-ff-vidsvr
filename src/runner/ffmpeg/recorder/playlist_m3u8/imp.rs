//
// runner::ffmpeg::recorder::playlist_m3u8
//
use crate::comm_media::MediaType;
use crate::runner::ffmpeg::{
    create_ffmpeg_log_file, FFMpegCmd, RstOnSpawnFFMpeg, RstPostSpawnFFMpeg, RunnerFFMCreateCtx,
    RunnerFFMError, RunnerFFMType,
};
use crate::runner::RunnerProcCmd;
use crate::{comm_fs, EchoPathBufToString, EchoStringToPathBuf};

pub fn new_ffm_cmd(
    ffmpeg_path: String,

    //Audio="-v quiet -i playlist.m3u8 -vn -acodec copy"
    //Video="-v quiet -i playlist.m3u8 -vcodec copy -acodec copy"
    ffmpeg_args_given: String,

    // default: 1M
    // > 1M=1Mega
    // > ffmpeg -fs limit_size
    ffmpeg_out_split_file_siz: String,

    ffmpeg_log_file_path: String,
    ffmpeg_verbose: String,

    media_type: MediaType,

    playlist_file_path: String,

    out_root_path: String,
    out_file_name: String,
) -> Result<FFMpegCmd, RunnerFFMError> {
    let cmd = FFMpegCmd::new_as_playlist_m3u8_recorder(
        ffmpeg_path.clone(),
        ffmpeg_args_given,
        ffmpeg_log_file_path,
        ffmpeg_verbose,
        ffmpeg_out_split_file_siz,
        media_type,
        playlist_file_path,
        out_root_path,
        out_file_name,
    )
    .map_err(|e| RunnerFFMError::OperErr(e.to_string()))?;

    Ok(cmd)
}

pub fn new_ffm_create_ctx(cmd: FFMpegCmd) -> RunnerFFMCreateCtx {
    RunnerFFMCreateCtx::new(
        RunnerFFMType::RUNNER_FFM_RECORDER("PLAYLIST_M3U8"),
        cmd,
        on_spawn_ffmpeg,
        post_spawn_ffmpeg,
        // ex) if ffmpeg process which generate playlist.m3u8 periodic is exit,
        // ffmpeg playlist-m3u8 recorder will exit automataically without terminate explicitly
        true,
        true,
    )
}

// pub fn get_spawn_callbacks_async() -> (PFnOnSpawnFFMpeg, PFnPostSpawnFFMpeg) {
//     return (
//         // Box::pin(pfn), guarantee that the pointee is not moved are very
//         // > specific and involve raw pointers and thus unsafe code
//         // > (for instance, when wanting to soundly implement a self-referential struct).
//         Box::new(move |prm| Box::pin(on_spawn_ffmpeg(prm))),
//         Box::new(move |prm| Box::pin(post_spawning_ffmpeg(prm))),
//     );
// }

//
// spawn event callbacks (do not use async function)
//

fn on_spawn_ffmpeg(ctx: &RunnerFFMCreateCtx) -> Result<RstOnSpawnFFMpeg, RunnerFFMError> {
    //Err(RunnerFFMError::OperErr("impl not yet".to_string()))
    assert!(ctx.chk_prop_key("playlist_file_path"));
    assert!(ctx.chk_prop_key("ffmpeg_verbose"));
    assert!(ctx.chk_prop_key("log_file_path"));
    assert!(ctx.chk_prop_key("rec_file_name"));
    assert!(ctx.chk_prop_key("rec_out_path"));

    log::debug!(
        "[ffmpeg::recorder::playlist_m3u8] spawn recorder ffmpeg, cmd={}",
        ctx.ffmpeg_cmd.get_ffmpeg_args().clone().join(" ")
    );

    let mut spawner = RunnerProcCmd::new(ctx.ffmpeg_cmd.get_program().clone());

    //
    // create record out path
    //

    let rec_out_path = EchoStringToPathBuf!(ctx.get_prop("rec_out_path"));

    if let Err(e) = comm_fs::create_dir_sync(&rec_out_path) {
        let _em = format!(
            "failed to create record out path={}, e={}",
            EchoPathBufToString!(rec_out_path),
            e.to_string()
        );
        return Err(RunnerFFMError::FFMpegSpawnErr(_em));
    }

    //
    // ffmpeg log file
    //

    let log_file_path = ctx.get_prop("log_file_path");

    match create_ffmpeg_log_file(log_file_path) {
        Ok(ffmpeg_log_file) => {
            spawner.stderr(ffmpeg_log_file);
        }
        Err(e) => {
            let _em = format!(
                "failed to create recorder ffmpeg log files.., path={}, e={}",
                log_file_path,
                e.to_string()
            );
            return Err(RunnerFFMError::FFMpegSpawnErr(_em));
        }
    }

    //
    // spawn ffmpeg recorder process
    //

    spawner.current_dir(rec_out_path.clone());
    spawner.args(ctx.ffmpeg_cmd.get_ffmpeg_args().clone());

    //self.cmd.print_ffmpeg_args();
    //let playlist_file_path = ctx.get_prop("playlist_file_path");

    let ffmpeg_proc = spawner.spawn();

    if let Err(e) = ffmpeg_proc {
        return Err(RunnerFFMError::FFMpegSpawnErr(e.to_string()));
    }

    let mut proc = ffmpeg_proc.unwrap();

    Ok((proc, spawner))
}

fn post_spawn_ffmpeg(ctx: &RunnerFFMCreateCtx) -> Result<RstPostSpawnFFMpeg, RunnerFFMError> {
    Ok(())
}

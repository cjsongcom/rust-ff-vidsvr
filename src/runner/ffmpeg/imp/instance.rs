use crate::comm_media::MediaType;
use crate::config::Config;
use crate::runner::ffmpeg::receiver::rtmp as FFMpegRTMPReceiver;
use crate::runner::ffmpeg::recorder::playlist_m3u8 as FFMpegM3u8Recorder;
use crate::runner::{
    ffmpeg::{FFMpegCmd, RunnerFFMCreateCtx, RunnerFFMError, RunnerFFMpeg},
    message::RecvWorkerMsgSend,
};
use crate::service::api::reqres::publish::ReqPropReceiverPrm;
use crate::{EchoPathBufToString, PropMedia};
use std::path::PathBuf;

pub fn new(
    config: Config,
    recv_worker_msg_send: RecvWorkerMsgSend,
    runner_prms: ReqPropReceiverPrm,
    media_prms: PropMedia,
    app_name: String,
    sess_key: String,
    publish_port: u16,
) -> Result<RunnerFFMpeg, RunnerFFMError> {
    let hls_root_dir = config.echo_hls_root_dir.to_str();

    if (hls_root_dir.is_none()) {
        return Err(RunnerFFMError::InvalidConfigParameter(format!(
            "ECHO_HLS_ROOT_DIR"
        )));
    }

    // {hls_out_root}/{app_name}
    let mut hls_out_path = PathBuf::from(format!("{}/{}", hls_root_dir.clone().unwrap(), app_name));

    //log::debug!("hls_out_path: {:?}", fs::canonicalize(&hls_out_path));
    // >  Err(Os { code: 2, kind: NotFound, message: "No such file or directory" })
    //let hls_out_path
    //    = fs::canonicalize(&hls_out_path).unwrap().to_str().unwrap().to_string();

    let ffmpeg_path = config.echo_ffmpeg_path.clone();
    let listen_ip = format!("0.0.0.0");

    let hls_out_root_path = config
        .echo_hls_root_dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap();

    let ffmpeg_log_root_path = config
        .echo_ffmpeg_log_root_dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap();

    let ffmpeg_hls_time = match media_prms.media_type {
        MediaType::Audio => config.echo_ffmpeg_hls_time_aud,
        MediaType::Video => config.echo_ffmpeg_hls_time_vid,
        _ => config.echo_ffmpeg_hls_time_aud,
    };

    //
    // receiver
    //

    let mut receiver_cmd: Option<FFMpegCmd> = None;
    let mut receiver_ctx: Option<RunnerFFMCreateCtx> = None;

    {
        let hls_out_path = format!("{}/{}", hls_out_root_path, app_name);
        let ffmpeg_log_file_path =
            format!("{}/{}_receiver.log", ffmpeg_log_root_path, app_name.clone());

        let receiver_cmd_rst = FFMpegRTMPReceiver::new_ffm_cmd(
            media_prms.clone(),
            app_name.clone(),
            sess_key.clone(),
            listen_ip.clone(),
            publish_port,
            hls_out_path.clone(),
            ffmpeg_path.clone(),
            runner_prms.args.clone(),
            ffmpeg_log_file_path.clone(),
            config.echo_ffmpeg_verbose.clone(),
            config.echo_ffmpeg_overwrite.clone(),
            0,
            config.echo_ffmpeg_vcodec.clone(),
            config.echo_ffmpeg_acodec.clone(),
            config.echo_ffmpeg_hls_init_time.clone(),
            ffmpeg_hls_time.clone(),
            config.echo_ffmpeg_hls_list_size.clone(),
        );

        receiver_cmd = match receiver_cmd_rst {
            Ok(c) => Some(c),
            Err(e) => {
                log::error!(
                    "failed to create {} receiver ffmpeg cmd, e={}",
                    "rtmp",
                    e.to_string()
                );
                None
            }
        };

        let mut _receiver_ctx = FFMpegRTMPReceiver::new_ffm_create_ctx(receiver_cmd.unwrap());

        _receiver_ctx.set_prop("hls_out_path", &hls_out_path);
        _receiver_ctx.set_prop(
            "hls_prerole_path",
            &EchoPathBufToString!(&config.echo_hls_prerole_dir.clone()),
        );
        _receiver_ctx.set_prop("log_file_path", &ffmpeg_log_file_path);

        receiver_ctx = Some(_receiver_ctx);
    }

    //
    // recorder
    //

    let mut recorder_cmd: Option<FFMpegCmd> = None;
    let mut recorder_ctx: Option<RunnerFFMCreateCtx> = None;

    {
        if config.echo_rec_enabled {
            let rec_ffmpeg_given = match media_prms.media_type {
                MediaType::Audio => config.echo_rec_ffmpeg_opt_aud.clone(),
                MediaType::Video => config.echo_rec_ffmpeg_opt_vid.clone(),
                _ => config.echo_rec_ffmpeg_opt_aud.clone(),
            };

            let src_playlist_file_path =
                format!("{}/{}/playlist.m3u8", hls_out_root_path, app_name);

            let rec_ffmpeg_verbose = config.echo_rec_ffmpeg_verbose.clone();

            let rec_ffmpeg_log_file_path =
                format!("{}/{}_record.log", ffmpeg_log_root_path, app_name);

            let rec_file_name = app_name.clone();
            let rec_out_path = format!(
                "{}/{}",
                EchoPathBufToString!(config.echo_rec_root_dir),
                app_name
            );

            let rec_cmd_rst = FFMpegM3u8Recorder::new_ffm_cmd(
                ffmpeg_path.clone(),
                rec_ffmpeg_given.clone(),
                config.echo_rec_ffmpeg_split_size.clone(),
                rec_ffmpeg_log_file_path.clone(),
                rec_ffmpeg_verbose.clone(),
                media_prms.media_type.clone(),
                src_playlist_file_path.clone(),
                EchoPathBufToString!(config.echo_rec_root_dir.clone()),
                rec_file_name.clone(),
            );

            recorder_cmd = match rec_cmd_rst {
                Ok(c) => Some(c),
                Err(e) => {
                    log::error!(
                        "failed to create {} recorder ffmpeg cmd, e={}",
                        "playlistm3u8",
                        e.to_string()
                    );
                    None
                }
            };

            let mut _recorder_ctx = FFMpegM3u8Recorder::new_ffm_create_ctx(recorder_cmd.unwrap());

            _recorder_ctx.set_prop("playlist_file_path", &src_playlist_file_path.clone());
            _recorder_ctx.set_prop("ffmpeg_verbose", &rec_ffmpeg_verbose.clone());
            _recorder_ctx.set_prop("log_file_path", &rec_ffmpeg_log_file_path.clone());
            _recorder_ctx.set_prop("rec_file_name", &rec_file_name.clone());
            _recorder_ctx.set_prop("rec_out_path", &rec_out_path.clone());

            recorder_ctx = Some(_recorder_ctx);
        }
    }

    //
    // RunnerFFMpeg
    //

    let (runner_msg_send, runner_msg_recv) = tokio::sync::mpsc::unbounded_channel();

    let inst = RunnerFFMpeg {
        recv_worker_msg_send,

        runner_msg_send,
        runner_msg_recv,

        receiver_ctx,
        recorder_ctx,

        inner_handle: None,
        inner_msg_send: None,
    };

    Ok(inst)
}

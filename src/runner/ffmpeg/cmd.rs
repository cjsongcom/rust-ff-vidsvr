use crate::comm_media::*;
use serde::{Deserialize, Serialize};

//let ffarg1 = String::from("-listen 1 -i \"rtmp://${SVR_IP}:${SVR_PORT}/${APP_NAME}/${STREAM_NAME}/key=xx\" -c:v:0 libx264 -x264-params \"nal-hrd=cbr:force-cfr=1\" -b:v:0 3M -maxrate:v:0 3M -minrate:v:0 3M -bufsize:v:0 3M -preset slow -g 48 -sc_threshold 0 -keyint_min 48 -c:a aac -strict -2 -crf 18 -flags -global_header -f hls -hls_time 1 -hls_list_size 5 -hls_flags \"delete_segments+independent_segments+discont_start\" -start_number 1 -hls_segment_type mpegts -master_pl_publish_rate 1 \"demo-m3u8/demo.m3u8\"");
//let ffarg2 = String::from("-v quiet -y -listen 1 -i rtmp://0.0.0.0:2280/demo -vcodec copy -acodec copy -flags -global_header -hls_time 1 -hls_list_size 10 -start_number 1 -hls_flags delete_segments playlist.m3u8");
/*
    $ ffmpeg
        -v quiet \       ; quite | panic | fatal | error | warning | info | verbose | debug | trace
        -y \             ; overwriting
        -listen 1 \
        -i rtmp://0.0.0.0:2280/demo \
        -vcodec copy \
        -acodec copy \
        -flags \
        -global_header \
        -hls_time 1 \
        -hls_list_size 10 \
        -start_number 1 \
        -hls_flags delete_segments \
        playlist.m3u8
*/

pub type FFMpegArgs = Vec<String>;

type FFMepgArgsInner = Vec<Vec<String>>;

/*
https://trac.ffmpeg.org/ticket/2294
The option from 0678c38 is -stimeout, which works. But note the units are microseconds.
Currently -timeout infers a listen mode. But this behavior will change after the version bump, after which it will be the same as -stimeout option.

‘-timeout’  -> ffmpeg 5.1.2 , working
    Set maximum timeout (in seconds) to wait for incoming connections.
    A value of -1 mean infinite (default). This option implies the
    ‘rtsp_flags’ set to ‘listen’. ‘reorder_queue_size’
    Set number of packets to buffer for handling of reordered packets.

‘-stimeout’  -> ffmpeg 5.1.2 Unrecognized option 'stimeout'
    Set socket TCP I/O timeout in micro seconds.


[rtmp]
Found the answer myself.
https://ffmpeg.org/ffmpeg-all.html#Protocols

All protocols accept the following options:
rw_timeout   -> ffmpeg 5.1.2 , working

Maximum time to wait for (network) read/write operations to complete, in
microseconds.

Eugene Medvedev

timeout=

*/

// https://ffmpeg.org/ffmpeg-protocols.html
// -rw_timeout: network timeout
// > int64, microsecond, 1sec=1000000 micorsecond)
// > 0 for infinite

// const DEF_FFMARG_AUDIO: &str = r#"-v quiet -y -listen 1 -rw_timeout 10000000 -acodec copy -flags -global_header -hls_time 1 -hls_list_size 10 -start_number 1 -hls_flags delete_segments -strftime 1 -hls_segment_filename %Y%m%d-%s.ts playlist.m3u8"#;
// const DEF_FFMARG_AUDIO_VIDEO: &str = r#"-v quiet -y -listen 1 -rw_timeout 10000000 -vcodec copy -acodec copy -flags -global_header -hls_time 1 -hls_list_size 10 -start_number 1 -hls_flags delete_segments -strftime 1 -hls_segment_filename %Y%m%d-%s.ts playlist.m3u8"#;

const DEF_FFMARG_AUDIO: &str = r#"-v quiet -y -listen 1 -rw_timeout 10000000 -vn -acodec copy -flags -global_header -hls_time 1 -hls_list_size 10 -start_number 1 -hls_flags delete_segments -strftime 1 playlist.m3u8"#;
const DEF_FFMARG_VIDEO: &str = r#"-v quiet -y -listen 1 -rw_timeout 10000000 -vcodec copy -acodec copy -flags -global_header -hls_time 5 -hls_list_size 10 -start_number 1 -hls_flags delete_segments -strftime 1 playlist.m3u8"#;

use thiserror::Error;

// Error=thiserror_impl proc_macro Error
#[derive(Error, Debug)]
pub enum FFMpegCmdError {
    #[error("error in ffmpeg cmd operation: {0}")]
    CmdOperErr(String),

    #[error("invalid ffmpeg cmd generation param: {0}")]
    InvalidCmdGenParameter(String),
}

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FFMpegCmdType {
    // RECEIVER("rtmp")
    RECEIVER(&'static str),

    // RECORDER("playlist")
    RECORDER(&'static str),
}

#[derive(Debug, Clone)]
pub struct FFMpegCmd {
    cmd_type: FFMpegCmdType,

    app_name: String,
    sess_key: String,

    playlist_file_path: String,

    out_path: String,
    out_file_name: String,

    ffmpeg_path: String,
    ffmpeg_args: FFMpegArgs,

    ffmpeg_log_file_path: String,
    ffmpeg_verbose: String,
    //stdout_file_path: String,
    //stderr_file_path: String,
}

impl FFMpegCmd {
    pub fn get_app_name(&self) -> String {
        self.app_name.clone()
    }

    pub fn args_to_string(&self) -> String {
        self.get_ffmpeg_args().join(" ")
    }

    pub fn print_ffmpeg_args(&self) {
        for x in self.get_ffmpeg_args().iter() {
            log::debug!("[FFMpegCmd::print_args] {x}");
        }
    }

    pub fn get_parsed_args(&self) -> &FFMpegArgs {
        &self.ffmpeg_args
    }

    pub fn get_default_args(prop_media: PropMedia) -> Result<String, String> {
        let rst = match prop_media.media_type {
            MediaType::Audio => DEF_FFMARG_AUDIO,
            MediaType::Video => DEF_FFMARG_VIDEO,
            e => return Err(format!("unsupported media={:?}", e)),
        };

        Ok(rst.to_string())
    }

    // pub fn get_stdout_file_path(&self) -> &String {
    //     &self.stdout_file_path
    // }

    // pub fn get_stderr_file_path(&self) -> &String {
    //     &self.stderr_file_path
    // }

    //
    // generate *.m4a/*.mp4 from playlist.m3u8
    //

    pub fn new_as_playlist_m3u8_recorder(
        ffmpeg_path: String,
        mut ffmpeg_args_given: String,
        ffmpeg_log_file_path: String,
        ffmpeg_verbose: String,
        ffmpeg_out_split_file_siz: String,
        media_type: MediaType,
        playlist_file_path: String,
        rec_root_path: String,
        rec_file_name: String,
        rec_meta_info: String,
    ) -> Result<Self, FFMpegCmdError> {
        //
        // -correct_ts_overflow 0
        //
        // -metadata rec_st_epoch="1687914818"
        // -metadata rec_arc_uuid="{UUIDV4}"
        // -metadata rec_file_name="36f39cd9"
        //
        // $ ffmpeg -v debug  -y
        //   -correct_ts_overflow 0
        //   -i ${SRC_M3U8_FILE_PATH}
        //   -vn -acodec copy
        //   ${OUT_M4A_FILE_PATH}
        //
        // $ ffmpeg -v verbose -i playlist.m3u8 -vn -acodec copy out1.m4a
        // $ ffmpeg -v verbose -i ${playlist_file_path} ${ffmpeg_args_given} ${rec_root_path}/${rec_file_name}
        //

        if ffmpeg_args_given.len() == 0 {
            ffmpeg_args_given = FFMpegCmd::get_record_default_args(media_props.clone())
                .map_err(|e| FFMpegCmdError::CmdOperErr(e))?;
        }

        let (args_given, playlist_file_path) = FFMpegCmd::parse_args(ffmpeg_args_given.clone())
            .map_err(|e| FFMpegCmdError::CmdOperErr(e))?;

        //
        let mut rec_file_path = format!("{}/{}", rec_file_name.clone());
        let mut args = FFMpegArgs::new();

        Ok(Self {
            cmd_type: FFMpegCmdType::RECORDER("playlist_m3u8"),

            app_name: String::new(),
            sess_key: String::new(),

            playlist_file_path: playlist_file_path,

            out_path: rec_root_path,
            out_file_name: rec_file_name,

            ffmpeg_path,
            ffmpeg_args: args,

            ffmpeg_log_file_path,
            ffmpeg_verbose,
        })
    }

    //
    // generate *.ts from rtmp input
    //
    pub fn new_as_rtmp_receiver(
        media_props: PropMedia,

        app_name: String,
        sess_key: String,
        listen_ip: String,
        publish_port: u16,

        hls_out_path: String,

        ffmpeg_path: String,
        mut ffmpeg_args_given: String,
        ffmpeg_log_file_path: String,
        ffmpeg_verbose: String,

        ffmpeg_overwrite: bool,
        ffmpeg_rw_timeout: i64,
        ffmpeg_vcodec: String,
        ffmpeg_acodec: String,

        ffmpeg_hls_init_time: String,
        ffmpeg_hls_time: String,
        ffmpeg_hls_list_size: i32,
    ) -> Result<Self, FFMpegCmdError> {
        // if ffmpeg_log_file_path.len() == 0 {
        //     return Err(FFMpegCmdError::InvalidCmdGenParameter(format!(
        //         "ffmpeg_log_file_path is empty"
        //     )));
        // }
        //let stderr_file_path = format!("{ffmpeg_log_file_path}/{app_name}.log");

        if ffmpeg_args_given.len() == 0 {
            ffmpeg_args_given = FFMpegCmd::get_default_args(media_props.clone())
                .map_err(|e| FFMpegCmdError::CmdOperErr(e))?;
        }

        let (args_given, playlist_file_path) = FFMpegCmd::parse_args(ffmpeg_args_given.clone())
            .map_err(|e| FFMpegCmdError::CmdOperErr(e))?;

        let mut path = format!("{}/{}", app_name.clone(), sess_key.clone());
        let mut args = FFMpegArgs::new();

        // -v , -verobose : quite | panic | fatal | error | warning | info | verbose | debug | trace
        args.push(format!("-v {}", ffmpeg_verbose));

        // -xerror (exit on error)

        // -y: overwrtting
        if ffmpeg_overwrite {
            args.push(format!("-y"));
        }

        //args.push(format!("-rw_timeout {}", ffmpeg_rw_timeout));

        //
        // add listen addr
        //

        // keep order
        args.push(format!("-listen 1"));
        args.push(format!(
            "-i {}://{}:{}/{}",
            "rtmp", //media_prms.protocol.to_string().to_lowercase(),
            listen_ip,
            publish_port,
            path
        ));

        //
        // refine arguments
        //

        for x in args_given.iter() {
            if x.is_empty() {
                continue;
            }
            //log::debug!("[args_intl-iter] {}", x.join("__"));

            if x.len() > 1 {
                // skip from given argument
                match x[0].as_str() {
                    "-listen" => continue, // -listen 1
                    "-i" => continue,      // -i rtmp://0.0.0.0:port/app_name
                    "-v" => continue,      // -v quite|debug|trace ..
                    "-y" => continue,      // -y      ; overwritting
                    "-rw_timeout" => continue,
                    "-vcodec" => continue,
                    "-acodec" => continue,
                    "-hls_time" => continue,
                    "-hls_list_size" => continue,
                    v => {
                        args.push(format!("{}", x.join(" ")));
                    }
                };
            }
        }

        //
        // add ffmpeg hls options
        //

        match media_props.media_type {
            MediaType::Video => {
                args.push(format!("-vcodec {}", ffmpeg_vcodec));
                args.push(format!("-acodec {}", ffmpeg_acodec));
            }
            MediaType::Audio => {
                // discard video stream
                args.push(format!("-vn "));

                args.push(format!("-acodec {}", ffmpeg_acodec));
            }
        };

        args.push(format!("-hls_init_time {}", ffmpeg_hls_init_time));

        // Segments will be cut at keyframes, so unless a keyframe exists each second,
        // > hls_time will not get honoured.
        // 비디오 스트림일경우 1초안에 키프레임을 생성이 힘드므로 hls_time(TARGET_DURATION) 을 정확히 반영하기어려움
        // > hls_time 유효값: 2, 4, 6, 8 ..
        args.push(format!("-hls_time {}", ffmpeg_hls_time));
        args.push(format!("-hls_list_size {}", ffmpeg_hls_list_size));

        //
        // add playlist
        //

        //args.push(format!("-hls_base_url {}/", app_name));
        args.push(format!("-hls_segment_filename %Y%m%d-%s.ts"));

        args.push(playlist_file_path.clone());
        //log::debug!("[ffmpeg-args] {}", args.join(" "));

        //
        // logging
        //

        Ok(Self {
            cmd_type: FFMpegCmdType::RECEIVER("rtmp"),

            app_name,
            sess_key,
            playlist_file_path,

            out_path: hls_out_path,
            out_file_name: String::new(),

            ffmpeg_path: ffmpeg_path,
            ffmpeg_args: args,
            ffmpeg_log_file_path,
            ffmpeg_verbose,
            //stdout_file_path: stdout_file_path,
            //stderr_file_path: stderr_file_path,
        })
    }

    pub fn get_program(&self) -> &str {
        &self.ffmpeg_path
    }

    pub fn get_out_path(&self) -> &str {
        &self.out_path
    }

    pub fn get_out_filename(&self) -> &str {
        &self.out_file_name
    }

    pub fn get_ffmpeg_log_file_path(&self) -> &str {
        &self.ffmpeg_log_file_path
    }

    pub fn parse_args(mut args_str: String) -> Result<(FFMepgArgsInner, String), String> {
        //
        // parse playlist m3u8
        //

        let tmp = args_str.clone();

        let mut rsp = tmp.rsplitn(2, ' ');

        if rsp.clone().count() < 2 {
            return Err(format!("parsing failed, argument count, {}", tmp.clone()));
        }

        let mut is_parsed_playlist = false;
        let mut playlist_path = "";

        if let Some(x) = rsp.next() {
            playlist_path = x;

            if let Some(y) = rsp.next() {
                args_str = y.to_string();
                is_parsed_playlist = true
            }
        }

        if !is_parsed_playlist {
            return Err(format!("parsing failed, playlist, {}", args_str));
        }

        //
        // parse ffmpeg params
        //

        let mut args = FFMepgArgsInner::new();

        for mut x in args_str.split(" -") {
            let mut y = String::from("");

            if !x.starts_with('-') {
                y = format!("-{}", x);
            }

            x = &y;

            let mut kv = x.split(' ');
            let cnt = kv.clone().count();

            if cnt == 1 {
                let mut item = vec![String::from(kv.next().unwrap())];

                // if item[0].starts_with('-') {
                //     item[0].remove(0);
                //     item[0].
                // }

                args.push(item);
            } else {
                let mut item = vec![
                    String::from(kv.next().unwrap()),
                    String::from(kv.next().unwrap()),
                ];

                // if item[0].starts_with('-') {
                //     item[0].remove(0);
                // }

                args.push(item);
            }
        }

        Ok((args, playlist_path.to_string()))
    }

    pub fn debug_print(&self) {
        let r = self.ffmpeg_args.join("");

        log::debug!("[arg-str] {}", r);
        log::debug!("[playlist_file_path] {}", self.playlist_file_path);

        for z in self.ffmpeg_args.clone() {
            log::debug!("val={}", z);
        }
    }

    // ["-v debug", "-y", "-hls_duration 1" ]
    // => [ "-v", "debug", "-y", "-hls_duration", "1" ]
    pub fn get_ffmpeg_args(&self) -> FFMpegArgs {
        let mut rst = FFMpegArgs::new();

        for x in &self.ffmpeg_args {
            let sp = x.splitn(2, ' ');

            for y in sp.into_iter() {
                rst.push(y.to_string());
            }
        }

        rst
    }
}

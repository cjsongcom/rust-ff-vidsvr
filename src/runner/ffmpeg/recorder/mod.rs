use serde::{Deserialize, Serialize};
use crate::{EchoEpoch, }

pub mod playlist_m3u8;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
pub enum RecFileStatus {
    NEW, // newly recording, on recording
    FIN, // recording is finished, but not archived
    ARC, // recording was archived on permanent storage(ex: s3). (safe to delete this file)
}

pub struct RecordFileDesc {
    file_name_prefix: String,
    st_epoch: EchoEpoch,   // epoch second
    file_status: RecFileStatus,

    file_name: String,
}

pub fn gen_record_file_name(file_name_prefix: String) -> (String, RecordFileDesc) {
    //
    // ex) <AppName>_<EPOCH>.<RecFileStatus>
    //      36f39cd9_1687914818.tmp
    //
    assert!(!rec_prefix.is_empty());

    let st_epoch = create::comm::echo_get_epoch();
    let file_status = RecFileStatus::NEW;

    let file_name = format!("{}_{}_{}", file_name_prefix, st_epoch, file_status);

    (
        file_name.clone(), 
        RecordFileDesc { 
            file_name_prefix,
            st_epoch,
            file_status,
            file_name,        
        }
    )
}

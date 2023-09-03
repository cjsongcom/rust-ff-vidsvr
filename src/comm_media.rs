use {
    serde::{Deserialize, Serialize},
    strum_macros,
};

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Audio,
    Video,
}

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    SRT,
    RTMP,
}

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaFormat {
    AAC,
    MP2T,
    FLV,
}

#[derive(strum_macros::Display, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaReceiver {
    FFMPEG,
    RUSTRTMP,
    RUSTSRT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct PropMedia {
    // 'type' is reserved identifier for rust
    // > rename 'media_type' to 'type' on serialize
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub protocol: Protocol,
    pub format: MediaFormat,
}

//pub type PropRtmp = HashMap<String, String>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropRtmp {
    pub url: String,  // rtmp://xx.yy.zz.public-ip:{port}/{app_name}
    pub name: String, // SessionKey
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropTransport {
    #[serde(rename = "type")]
    pub ip_type: String, // "IPv4",
    pub address: String, // public ip
    pub port: u16,
}

//pub type PropReceiver = HashMap<String, String>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropReceiver {
    #[serde(rename = "type")]
    pub receiver_type: MediaReceiver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropPublish {
    pub name: String, // app_name,
    pub transports: Vec<PropTransport>,
    pub media: PropMedia,
    pub rtmp: PropRtmp,
}

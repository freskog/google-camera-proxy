use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceList {
    #[serde(rename = "devices")]
    pub cameras: Vec<Camera>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Camera {
    pub name: String,
    #[serde(rename = "traits")]
    pub details: CameraDetails,
    #[serde(rename = "parentRelations")]
    pub locations: Vec<Location>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CameraDetails {
    #[serde(rename = "sdm.devices.traits.CameraLiveStream")]
    pub camera_live_stream: CameraLiveStream,
    #[serde(rename = "sdm.devices.traits.Info")]
    pub info: DeviceInfo,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CameraLiveStream {
    pub max_video_resolution: Option<Resolution>,
    pub video_codecs: Vec<String>,
    pub audio_codecs: Vec<String>,
    pub supported_protocols: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resolution {
    pub width: u16,
    pub height: u16,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub custom_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamResponse<T> {
    pub results: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtspStreamGenerated {
    pub stream_urls: StreamUrl,
    pub stream_extension_token: String,
    pub stream_token: String,
    pub expires_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum StreamUrl {
    RtspUrl(String),
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtspStreamExtended {}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCommandBody {
    pub command: String,
    pub params: HashMap<String, String>,
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn parse_device_list_response() {
        let response = include_str!("test_data/device-list-response.json");
        let devices = serde_json::from_str::<DeviceList>(response).unwrap();

        println!("{:#?}", devices);

        assert!(devices.cameras.len() > 0);
    }
}

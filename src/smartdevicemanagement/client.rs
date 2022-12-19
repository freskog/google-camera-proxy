use crate::smartdevicemanagement::api::StreamUrl::RtspUrl;
use crate::smartdevicemanagement::api::*;
use anyhow::Result;
use dirs;
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use reqwest::Url;
use yup_oauth2 as oauth2;
use yup_oauth2::authenticator::*;

async fn auth() -> Authenticator<HttpsConnector<HttpConnector>> {
    let home_dir = dirs::home_dir().expect("Can't locate users home directory!");
    let client_secret_path = home_dir.join("client_secret.json");

    let config_secret = oauth2::read_application_secret(&client_secret_path)
        .await
        .expect(&format!(
            "Expected application secret at {:?}",
            &client_secret_path
        ));

    let sdm_token_path = home_dir.join("sdm-tokens.json");

    oauth2::InstalledFlowAuthenticator::builder(
        config_secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(sdm_token_path)
    .build()
    .await
    .expect("Can't construct authenticator")
}

pub struct SmartDeviceMgmtApi {
    auth: Authenticator<HttpsConnector<HttpConnector>>,
    scopes: Vec<String>,
    base_url: Url,
    client: reqwest::Client,
}

impl SmartDeviceMgmtApi {
    pub async fn new() -> SmartDeviceMgmtApi {
        let auth = auth().await;
        let client = reqwest::Client::new();
        let scopes = vec!["https://www.googleapis.com/auth/sdm.service".to_string()];
        let base_url: Url = Url::parse("https://smartdevicemanagement.googleapis.com/v1/").unwrap();
        SmartDeviceMgmtApi {
            auth,
            scopes,
            base_url,
            client,
        }
    }

    pub async fn device_list(&self) -> Result<DeviceList, anyhow::Error> {
        let device_list = self
            .base_url
            .join("enterprises/92046848-4dc7-465d-948e-3060efad9fe9/devices")?;
        let token = self.auth.token(&self.scopes).await?;
        let token_str = token.token().expect("Token can not be None!");
        let response = self
            .client
            .get(device_list)
            .bearer_auth(token_str)
            .send()
            .await?
            .text()
            .await?;

        let devices: Result<DeviceList, anyhow::Error> =
            serde_json::from_str::<DeviceList>(&response).map_err(|err| err.into());

        devices
    }

    pub async fn generate_rtsp_stream(
        &self,
        device_id: &String,
    ) -> Result<StreamResponse<RtspStreamGenerated>, anyhow::Error> {
        let token = self.auth.token(&self.scopes).await?;
        let token_str = token.token().expect("Token can not be None!");
        let command_url = format!("{:}:executeCommand", device_id);
        let get_rtsp_stream_url: Url = self.base_url.join(&command_url)?;
        let command_body = ExecuteCommandBody {
            command: "sdm.devices.commands.CameraLiveStream.GenerateRtspStream".into(),
            params: std::collections::HashMap::new(),
        };

        let response = self
            .client
            .post(get_rtsp_stream_url)
            .json::<ExecuteCommandBody>(&command_body)
            .bearer_auth(&token_str)
            .send()
            .await?
            .text()
            .await?;

        let response: Result<StreamResponse<RtspStreamGenerated>> =
            serde_json::from_str::<StreamResponse<RtspStreamGenerated>>(&response)
                .map_err(|e| e.into());

        response
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn get_device_list() {
        let future_devices = async {
            let api = SmartDeviceMgmtApi::new().await;
            api.device_list().await.unwrap()
        };

        let devices = async_std::task::block_on(future_devices);

        assert!(devices.cameras.len() > 0);
    }

    #[test]
    fn get_rtsp_stream() {
        let rtsp_camera_url = async {
            let api = SmartDeviceMgmtApi::new().await;
            let devices = api.device_list().await.unwrap();
            let first_rtsp = devices
                .cameras
                .iter()
                .find(|c| {
                    c.details
                        .camera_live_stream
                        .supported_protocols
                        .contains(&"RTSP".to_string())
                })
                .expect("There should be at least one RTSP enabled camera");

            let rtsp_camera_id = &first_rtsp.name;

            let response = api.generate_rtsp_stream(&rtsp_camera_id).await?;

            anyhow::Ok(response.results.stream_urls)
        };

        match async_std::task::block_on(rtsp_camera_url) {
            Ok(RtspUrl(url)) => {
                println!("{:}", url);
                assert!(url.starts_with("rtsp"))
            }
            Err(e) => panic!("unexpected error {:?}", e),
        };
    }
}

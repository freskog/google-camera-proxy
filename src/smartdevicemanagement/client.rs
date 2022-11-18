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
    async fn new() -> SmartDeviceMgmtApi {
        let auth = auth().await;
        let client = reqwest::Client::new();
        let scopes = vec!["https://www.googleapis.com/auth/sdm.service".to_string()];
        let base_url:Url = Url::parse("https://smartdevicemanagement.googleapis.com/v1/enterprises/92046848-4dc7-465d-948e-3060efad9fe9/").unwrap();
        SmartDeviceMgmtApi {
            auth,
            scopes,
            base_url,
            client,
        }
    }

    async fn device_list(&self) -> Result<DeviceList, anyhow::Error> {
        let device_list = self.base_url.join("devices")?;
        println!("GET {:}", device_list);
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

        println!("{:}", response);

        let devices: Result<DeviceList, anyhow::Error> =
            serde_json::from_str::<DeviceList>(&response).map_err(|err| err.into());

        devices
    }
}

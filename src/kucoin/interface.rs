use data_encoding::BASE64;
use hmac::{Hmac, Mac};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_this_or_that::{as_f64, as_u64};
use sha2::Sha256;
use std::sync::Arc;
use std::{
    fs::File,
    io::BufReader,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

// Alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct KucoinConfiguration {
    credentials: KucoinCreds,
    configuration: KucoinConfig,
}
#[derive(Debug, Deserialize)]
pub struct KucoinCreds {
    pub api_key: String,
    pub api_passphrase: String,
    pub api_secret: String,
    pub api_key_version: String,
}
#[derive(Debug, Deserialize)]
pub struct KucoinConfig {
    pub base_token: String,
    pub trade_amount: f32,
    pub enviroment: KucoinEnviroment,
}

#[derive(Serialize, Debug)]
pub enum KucoinRequestType {
    Get,
    Post,
    OrderPost,
    WebsocketToken,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KucoinRequestOrderPost {
    pub timeInForce: String,
    pub size: f64,
    pub price: f64,
    pub symbol: String,
    pub side: String,
    pub clientOid: u32,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
// status tier
pub struct KucoinResponseL0 {
    #[serde(deserialize_with = "as_u64")]
    code: u64,
    #[serde(default)]
    data: KucoinResponseL1,
    #[serde(default)]
    // only in reponse from order reqest
    orderId: String,
    #[serde(default)]
    msg: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub struct KucoinResponseL1 {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub instanceServers: Vec<KucoinResponseL2Token>,
    #[serde(default)]
    pub ticker: Vec<KucoinResponseL2Coins>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub struct KucoinResponseL2Coins {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub symbolName: String,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub buy: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub sell: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub changeRate: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub changePrice: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub high: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub low: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_u64")]
    pub vol: u64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub volValue: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub last: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub averagePrice: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub takerFeeRate: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub makerFeeRate: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub takerCoefficient: f64,
    #[serde(default)]
    #[serde(deserialize_with = "as_f64")]
    pub makerCoefficient: f64,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub struct KucoinResponseL2Token {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub encrypt: bool,
    #[serde(default)]
    #[serde(deserialize_with = "as_u64")]
    pub pingInterval: u64,
    #[serde(default)]
    #[serde(deserialize_with = "as_u64")]
    pub pingTimeout: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KucoinEnviroment {
    Live,
    Sandbox,
}

#[derive(Debug)]
pub struct KucoinInterface {
    config: Arc<KucoinConfig>,
    creds: Arc<KucoinCreds>,
    client: Client,
}

impl KucoinInterface {
    pub fn new() -> KucoinInterface {
        KucoinInterface::default()
    }

    pub fn default() -> KucoinInterface {
        // Gets api credentials
        let config_file_path = "config.json".to_string();
        let config_file = File::open(config_file_path).expect("unable to read config.json");
        let configuration: KucoinConfiguration =
            serde_json::from_reader(BufReader::new(config_file))
                .expect("unable to parse KucoinKeys.json");

        // Makes new reqwest client so its all the same session
        KucoinInterface {
            config: Arc::new(configuration.configuration),
            creds: Arc::new(configuration.credentials),
            client: Client::new(),
        }
    }

    pub async fn request(
        &self,
        endpoint: &str,
        json: Option<String>, //Option<KucoinRequestOrderPost>,
        method: KucoinRequestType,
    ) -> Option<KucoinResponseL1> {
        let since_the_epoch = SystemTime::now() // this is wrong // TODO: need to convert to UTC
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let signed_secret = HmacSha256::new_from_slice(self.creds.api_secret.as_bytes()).unwrap();

        let method_str: &str = match method {
            KucoinRequestType::Get => "GET",
            KucoinRequestType::Post => "POST",
            KucoinRequestType::OrderPost => "POST",
            KucoinRequestType::WebsocketToken => "POST",
        };

        let payload_str = match json {
            Some(data) => format!("{}{}{}{}", &since_the_epoch, method_str, endpoint, data),
            None => format!("{}{}{}", &since_the_epoch, method_str, endpoint),
        };
        let mut payload = signed_secret.clone();
        payload.update(payload_str.as_bytes());
        let payload_hmac = payload.finalize();
        let b64_signed_payload: String =
            BASE64.encode(&format!("{:x}", payload_hmac.into_bytes()).into_bytes());

        let mut passphrase = signed_secret.clone();
        passphrase.update(self.creds.api_passphrase.as_bytes());
        let passphrase_hmac = passphrase.finalize();
        let b64_signed_passphrase: String =
            BASE64.encode(&format!("{:x}", passphrase_hmac.into_bytes()).into_bytes());

        // Get headers

        let base_url: &str = match self.config.enviroment {
            KucoinEnviroment::Live => "api.kucoin.com",
            KucoinEnviroment::Sandbox => "openapi-sandbox.kucoin.com",
        };
        let url: Url = Url::parse(&format!("https://{}{}", base_url, endpoint))
            .expect("Was unable to join the endpoint and base_url");

        let client = match method {
            KucoinRequestType::Post => self.client.post(url),
            KucoinRequestType::Get => self.client.get(url),
            KucoinRequestType::WebsocketToken => self.client.post(url),
            KucoinRequestType::OrderPost => self.client.post(url),
        };
        let client = client
            .header(CONTENT_TYPE, "application/json")
            .header(
                HeaderName::from_static("kc-api-key-version"),
                HeaderValue::from_str(&self.creds.api_key_version).unwrap(),
            )
            .header(
                HeaderName::from_static("kc-api-passphrase"),
                HeaderValue::from_str(&b64_signed_passphrase).unwrap(),
            )
            .header(
                HeaderName::from_static("kc-api-timestamp"),
                HeaderValue::from_str(&since_the_epoch).unwrap(),
            )
            .header(
                HeaderName::from_static("kc-api-sign"),
                HeaderValue::from_str(&b64_signed_payload).unwrap(),
            )
            .header(
                HeaderName::from_static("kc-api-key"),
                HeaderValue::from_str(&self.creds.api_key).unwrap(),
            );
        let client = client.body(payload_str);
        self.response(
            client
                .send()
                .await
                .expect("failed to post reqwest")
                .text()
                .await
                .expect("failed to get payload"),
        )
    }

    fn response(&self, response: String) -> Option<KucoinResponseL1> {
        println!("{}", response);
        // TODO: maybe parse the status code here and panic with better errors
        let l1: KucoinResponseL0 = serde_json::from_str(&response).unwrap();
        if l1.code != 200000 {
            panic!("Recived Bad Response Status from Kucoin:\n\n{:?}", l1);
        } else {
            Some(l1.data)
        }
    }

    pub async fn get_pairs(&self) -> Option<KucoinResponseL1> {
        self.request("/api/v1/market/allTickers", None, KucoinRequestType::Get)
            .await
    }

    pub async fn get_account(&self) -> Option<KucoinResponseL1> {
        self.request("/api/v1/accounts", None, KucoinRequestType::Get)
            .await
    }

    pub async fn get_websocket_info(&self) -> Option<KucoinResponseL1> {
        self.request(
            "/api/v1/bullet-public", // TODO: This should be private and auth with creds
            None,
            KucoinRequestType::WebsocketToken,
        )
        .await
    }

    pub fn diagnose(&self) {
        println!(
            "api key: {}, \napi passphrase: {}, \napi secret: {}, \napi key version: {}",
            self.creds.api_key,
            self.creds.api_passphrase,
            self.creds.api_secret,
            self.creds.api_key_version
        )
    }
}

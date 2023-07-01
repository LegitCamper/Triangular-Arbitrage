use data_encoding::BASE64;
use hmac::{Hmac, Mac};
use reqwest::Client;
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

#[derive(Serialize, Debug, Clone)]
pub struct KucoinRequestOrderPost {
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    pub size: f64,
    pub price: f64,
    pub symbol: String,
    pub side: String,
    #[serde(rename = "clientOid")]
    pub client_o_id: u32,
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

        KucoinInterface {
            config: Arc::new(configuration.configuration),
            creds: Arc::new(configuration.credentials),
            client: Client::new(),
        }
    }

    pub async fn request(
        &self,
        endpoint: &str,
        json: Option<KucoinRequestOrderPost>,
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

        let payload_str = match json.clone() {
            Some(data) => format!(
                "{}{}{}{}",
                &since_the_epoch,
                method_str,
                endpoint,
                serde_json::to_string(&data).unwrap()
            ),
            None => format!("{}{}{}", &since_the_epoch, method_str, endpoint),
        };
        println!("{}", payload_str);
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

        let base_url: &str = match self.config.enviroment {
            KucoinEnviroment::Live => "api.kucoin.com",
            KucoinEnviroment::Sandbox => "openapi-sandbox.kucoin.com",
        };
        let url: Url = Url::parse(&format!("https://{}{}", base_url, endpoint)).unwrap();

        let mut client = match method {
            KucoinRequestType::Post => self.client.post(url),
            KucoinRequestType::Get => self.client.get(url),
            KucoinRequestType::WebsocketToken => self.client.post(url),
            KucoinRequestType::OrderPost => self.client.post(url),
        };

        // Get headers
        client = client
            .header("kc-api-key", &self.creds.api_key)
            .header("kc-api-sign", &b64_signed_payload)
            .header("kc-api-timestamp", &since_the_epoch)
            .header("kc-api-passphrase", &b64_signed_passphrase)
            .header("kc-api-key-version", &self.creds.api_key_version)
            .header("charset", "utf-8");

        println!("{:?}", client);

        client = match json.clone() {
            Some(data) => client.json(&data.clone()),
            None => client,
        };

        let response = client.send().await.unwrap();
        if response.status().is_success() {
            serde_json::from_str(&response.text().await.unwrap()).unwrap()
        } else {
            panic!(
                "status {}, error: {}",
                response.status(),
                response.text().await.unwrap()
            )
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

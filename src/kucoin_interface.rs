use data_encoding::BASE64;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_this_or_that::{as_f64, as_u64};
use std::str::FromStr;
use std::sync::Arc;
use std::{
    fs::File,
    io::BufReader,
    time::{SystemTime, UNIX_EPOCH},
};

use url::Url;

#[derive(Debug)]
pub struct KucoinInterface(pub Arc<KucoinCreds>, Client);

#[derive(Debug, Deserialize)]
pub struct KucoinCreds {
    api_key: String,
    api_passphrase: String,
    api_secret: String,
    api_key_version: String,
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
    data: KucoinResponseL1,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
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

impl KucoinInterface {
    pub fn new() -> KucoinInterface {
        // Gets api credentials
        let creds_file_path = "KucoinKeys.json".to_string();
        let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
        let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
            .expect("unable to parse KucoinKeys.json");

        // Makes new reqwest client so its all the same session
        KucoinInterface(Arc::new(api_creds), Client::new())
    }

    pub fn default() -> KucoinInterface {
        // Gets api credentials
        let creds_file_path = "KucoinKeys.json".to_string();
        let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
        let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
            .expect("unable to parse KucoinKeys.json");

        // Makes new reqwest client so its all the same session
        KucoinInterface(Arc::new(api_creds), Client::new())
    }

    pub fn get_headers(&self, payload: String, passphrase: String, timestamp: String) -> HeaderMap {
        // Make header type
        let mut headers = HeaderMap::new();
        headers.insert(
            "KC-API-KEY",
            HeaderValue::from_bytes(self.0.api_key.as_bytes()).unwrap(),
        );
        headers.insert("KC-API-SIGN", HeaderValue::try_from(payload).unwrap());
        headers.insert(
            "KC-API-TIMESTAMP",
            HeaderValue::try_from(timestamp).unwrap(),
        );
        headers.insert(
            "KC-API-PASSPHRASE",
            HeaderValue::try_from(passphrase).unwrap(),
        );
        headers.insert(
            "KC-API-KEY-VERSION",
            HeaderValue::try_from(self.0.api_key_version.as_bytes()).unwrap(),
        );
        headers
    }

    pub async fn request(
        &self,
        endpoint: &str,
        json: String,
        method: KucoinRequestType,
    ) -> Option<KucoinResponseL1> {
        // alias values in self
        // let api_creds = &self.0;
        // let client = &self.1;

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .to_string();

        // Signs the api secret with hmac sha256
        let signed_key = hmac::Key::new(hmac::HMAC_SHA256, self.0.api_secret.as_bytes());

        // signs kucoin_request_string
        let payload = format!("{}{}{}{}", &since_the_epoch, "POST", endpoint, json);
        let signed_payload = hmac::sign(&signed_key, payload.as_bytes());
        let b64_signed_payload: String = BASE64.encode(signed_payload.as_ref());

        // Signs/Encrypt passphrase with HMAC-sha256 via API-Secret
        let signed_passphrase = hmac::sign(&signed_key, self.0.api_passphrase.as_bytes());
        let b64_signed_passphrase: String = BASE64.encode(signed_passphrase.as_ref());

        // Get headers
        let headers = self.get_headers(b64_signed_payload, b64_signed_passphrase, since_the_epoch);
        // let headers = HeaderMap::new(); // TODO: REMOVE THIS

        let base_url: Url = Url::parse("https://api.kucoin.com").unwrap();
        let url: Url = base_url
            .join(endpoint)
            .expect("Was unable to join the endpoint and base_url");

        match method {
            KucoinRequestType::Post => {
                let res = self
                    .1
                    .post(url)
                    .send()
                    .await
                    .expect("failed to post reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(self.response(res))
            }
            KucoinRequestType::Get => {
                let res = self
                    .1
                    .get(url)
                    .json(&json)
                    .send()
                    .await
                    .expect("failed to get reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(self.response(res))
            }
            KucoinRequestType::WebsocketToken => {
                let res = self
                    .1
                    .post(url) // TODO: Should be private endpoint and use creds
                    .headers(headers)
                    .send()
                    .await
                    .expect("failed to post reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(self.response(res))
            }
            KucoinRequestType::OrderPost => {
                let res = self
                    .1
                    .post(url)
                    .headers(headers)
                    .json(&json)
                    .send()
                    .await
                    .expect("failed to post reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(self.response(res))
            }
        }
    }

    fn response(&self, response: String) -> KucoinResponseL1 {
        // TODO: maybe parse the status code here and panic with better errors
        let l1: KucoinResponseL0 = serde_json::from_str(&response).unwrap();
        if l1.code != 200000 {
            panic!(
                "Unable to read Kucoin Reponse\nSomething Probably Went Wrong\n{:?}",
                l1
            )
        } else {
            l1.data
        }
    }

    pub async fn get_pairs(&self) -> Option<KucoinResponseL1> {
        self.request(
            "/api/v1/market/allTickers",
            String::from(""),
            KucoinRequestType::Get,
        )
        .await
    }

    pub async fn get_websocket_info(&self) -> Option<KucoinResponseL1> {
        self.request(
            "/api/v1/bullet-private", // TODO: This should be private and auth with creds
            String::from(""),
            KucoinRequestType::WebsocketToken,
        )
        .await
    }

    // The clones here are delibrate
    pub fn clone_keys(&self) -> KucoinCreds {
        KucoinCreds {
            api_key: self.0.api_key.to_owned(),
            api_passphrase: self.0.api_passphrase.to_owned(),
            api_secret: self.0.api_secret.to_owned(),
            api_key_version: self.0.api_key_version.to_owned(),
        }
    }
}

use data_encoding::BASE64;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_this_or_that::{as_f64, as_u64};
use std::{
    fs::File,
    io::BufReader,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

#[derive(Debug)]
pub struct KucoinInterface(pub KucoinCreds, Client);

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
struct KucoinRequestOrderPost {
    timeInForce: String,
    size: f64,
    price: f64,
    symbol: String,
    side: String,
    clientOid: u32,
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
// data tier
pub struct KucoinResponseL1 {
    token: String, // Only returned with websocket token
    instanceServers: Vec<KucoinResponseL2>,
    ticker: Vec<KucoinResponseL2>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub struct KucoinResponseL2 {
    // for pair response
    symbol: String,
    symbolName: String,
    #[serde(deserialize_with = "as_f64")]
    buy: f64,
    #[serde(deserialize_with = "as_f64")]
    sell: f64,
    #[serde(deserialize_with = "as_f64")]
    changeRate: f64,
    #[serde(deserialize_with = "as_f64")]
    changePrice: f64,
    #[serde(deserialize_with = "as_f64")]
    high: f64,
    #[serde(deserialize_with = "as_f64")]
    low: f64,
    #[serde(deserialize_with = "as_u64")]
    vol: u64,
    #[serde(deserialize_with = "as_f64")]
    volValue: f64,
    #[serde(deserialize_with = "as_f64")]
    last: f64,
    #[serde(deserialize_with = "as_f64")]
    averagePrice: f64,
    #[serde(deserialize_with = "as_f64")]
    takerFeeRate: f64,
    #[serde(deserialize_with = "as_f64")]
    makerFeeRate: f64,
    #[serde(deserialize_with = "as_f64")]
    takerCoefficient: f64,
    #[serde(deserialize_with = "as_f64")]
    makerCoefficient: f64,
    // for websocket tocker response
    endpoint: String,
    encrypt: bool,
    #[serde(deserialize_with = "as_u64")]
    pingInterval: u64,
    #[serde(deserialize_with = "as_u64")]
    pingTimeout: u64,
}

impl KucoinInterface {
    pub fn new() -> KucoinInterface {
        // Gets api credentials
        let creds_file_path = "KucoinKeys.json".to_string();
        let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
        let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
            .expect("unable to parse KucoinKeys.json");

        // Makes new reqwest client so its all the same session
        KucoinInterface(api_creds, Client::new())
    }

    pub async fn request(
        self,
        endpoint: &str,
        json: String,
        method: KucoinRequestType,
    ) -> Option<KucoinResponseL1> {
        // alias values in self
        let api_creds = &self.0;
        let client = &self.1;

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .to_string();

        // Signs the api secret with hmac sha256
        let signed_key = hmac::Key::new(hmac::HMAC_SHA256, api_creds.api_secret.as_bytes());

        // signs kucoin_request_string
        let payload = format!("{}{}{}{}", &since_the_epoch, "POST", endpoint, json); // have it as static POST because it doest seem like the GET needs it
        println!("\nPAYLOAD:   \n {}", payload);
        let signed_payload = hmac::sign(&signed_key, payload.as_bytes());
        let b64_signed_payload: String = BASE64.encode(signed_payload.as_ref());

        // Signs/Encrypt passphrase with HMAC-sha256 via API-Secret
        let signed_passphrase = hmac::sign(&signed_key, api_creds.api_passphrase.as_bytes());
        let b64_signed_passphrase: String = BASE64.encode(signed_passphrase.as_ref());

        let base_url: Url = Url::parse("https://api.kucoin.com").unwrap();
        let url: Url = base_url
            .join(endpoint)
            .expect("Was unable to join the endpoint and base_url");

        // Make header type
        let mut _headers = HeaderMap::new();
        // headers.insert(
        //     HeaderName::from_static("KC-API-KEY"),
        //     HeaderValue::from_static(&self.0.api_key),
        // );

        match method {
            KucoinRequestType::Post => {
                let res = client
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
                let res = client
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
                let res = client
                    .post(url) // TODO: Should be private endpoint and use creds
                    // .header("KC-API-KEY", api_creds.api_key)
                    // .header("KC-API-SIGN", b64_encoded_sig)
                    // .header("KC-API-TIMESTAMP", since_the_epoch)
                    // .header("API-PASSPHRASE", hmac_passphrase)
                    // .header("KC-API-VERSION", api_creds.api_key_version)
                    .send()
                    .await
                    .expect("failed to post reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(self.response(res))
            }
            KucoinRequestType::OrderPost => {
                println!(
                    "Key: {}\nPayload: {}\nTimestamp: {}\nPassphrase: {}\nVersion: {}\nJson: {}",
                    api_creds.api_key,
                    b64_signed_payload,
                    since_the_epoch,
                    b64_signed_passphrase,
                    api_creds.api_key_version,
                    json
                );
                let res = client
                    .post(url)
                    .header("KC-API-KEY", api_creds.api_key.clone())
                    // .header("KC-API-SIGN", b64_signed_payload)
                    .header("KC-API-TIMESTAMP", since_the_epoch)
                    .header("API-PASSPHRASE", b64_signed_passphrase)
                    .header("KC-API-VERSION", api_creds.api_key_version.clone())
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

    fn response(self, response: String) -> KucoinResponseL1 {
        // TODO: maybe parse the status code here and panic with better errors
        println!("{}", response);
        let l1: KucoinResponseL0 = serde_json::from_str(&response).expect("fuckj");
        // println!("{:?}", l1);
        if l1.code != 200000 {
            panic!(
                "Unable to read Kucoin Reponse\nSomething Probably Went Wrong\n{:?}",
                l1
            )
        } else {
            l1.data
        }
    }

    // pub async fn get_pairs(self) -> Option<KucoinResponseL0> {
    //     self.request(
    //         "/api/v1/market/allTickers",
    //         String::from(""),
    //         KucoinRequestType::Get,
    //     );
    // }

    pub fn clone_keys(self) -> KucoinCreds {
        KucoinCreds {
            api_key: self.0.api_key,
            api_passphrase: self.0.api_passphrase,
            api_secret: self.0.api_secret,
            api_key_version: self.0.api_key_version,
        }
    }
}

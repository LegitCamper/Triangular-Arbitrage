// use data_encoding::BASE64;
// use hmac::{Hmac, Mac};
// use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
// use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_this_or_that::{as_f64, as_u64};
// use sha2::Sha256;
// use std::sync::Arc;
use std::{
    fs::File,
    io::BufReader,
    // time::{SystemTime, UNIX_EPOCH},
};
// use url::Url;

// Alias for HMAC-SHA256
// type HmacSha256 = Hmac<Sha256>;

// #[derive(Debug)]
// pub struct KucoinInterface(pub Arc<KucoinCreds>, Client);

use kucoin_rs::kucoin::client::Kucoin;

#[derive(Debug, Deserialize)]
pub struct KucoinCreds {
    pub api_key: String,
    pub api_passphrase: String,
    pub api_secret: String,
    pub api_key_version: String,
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

pub fn get_creds() -> KucoinCreds {
    // Gets api credentials
    let creds_file_path = "KucoinKeys.json".to_string();
    let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
    let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
        .expect("unable to parse KucoinKeys.json");
    api_creds
}

// impl KucoinInterface {
//     pub fn new() -> KucoinInterface {
//         // Gets api credentials
//         let creds_file_path = "KucoinKeys.json".to_string();
//         let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
//         let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
//             .expect("unable to parse KucoinKeys.json");

//         // Makes new reqwest client so its all the same session
//         KucoinInterface(Arc::new(api_creds), Client::new())
//     }

//     pub fn default() -> KucoinInterface {
//         // Gets api credentials
//         let creds_file_path = "KucoinKeys.json".to_string();
//         let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
//         let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
//             .expect("unable to parse KucoinKeys.json");

//         // Makes new reqwest client so its all the same session
//         KucoinInterface(Arc::new(api_creds), Client::new())
//     }

//     pub fn get_headers(&self, method: String, endpoint: &str, json: &String) -> HeaderMap {
//         let since_the_epoch = SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .unwrap()
//             .as_millis()
//             .to_string();

//         let mut hmac_secret = HmacSha256::new_varkey(self.0.api_secret.as_bytes())
//             .expect("HMAC can take key of any size");
//         // hmac_passphrase.input(passphrase.as_bytes());

//         // signs payload
//         let payload_str = format!("{}{:?}{}{}", &since_the_epoch, method, endpoint, json);
//         let mut payload = hmac_secret.clone();
//         payload.input(payload_str.as_bytes());
//         let sign_result = payload.result();
//         let sign_bytes = sign_result.code();
//         let sign_digest = BASE64.encode(&sign_bytes);

//         // signs passphrase
//         let mut passphrase = hmac_secret; //.clone();
//         passphrase.input(self.0.api_passphrase.as_bytes());
//         let passphrase_result = passphrase.result();
//         let passphrase_bytes = passphrase_result.code();
//         let passphrase_digest = BASE64.encode(&passphrase_bytes);

//         let mut headers = HeaderMap::new();
//         headers.insert(
//             HeaderName::from_bytes("KC-API-KEY".as_bytes()).unwrap(),
//             HeaderValue::from_bytes(self.0.api_key.as_bytes()).unwrap(),
//         );
//         headers.insert(
//             HeaderName::from_bytes("KC-API-SIGN".as_bytes()).unwrap(),
//             HeaderValue::from_bytes(sign_digest.as_bytes()).unwrap(),
//         );
//         headers.insert(
//             HeaderName::from_bytes("KC-API-TIMESTAMP".as_bytes()).unwrap(),
//             HeaderValue::from_bytes(since_the_epoch.as_bytes()).unwrap(),
//         );
//         headers.insert(
//             HeaderName::from_bytes("KC-API-PASSPHRASE".as_bytes()).unwrap(),
//             HeaderValue::from_bytes(passphrase_digest.as_bytes()).unwrap(),
//         );
//         headers.insert(
//             HeaderName::from_bytes("KC-API-KEY-VERSION".as_bytes()).unwrap(),
//             HeaderValue::from_bytes(self.0.api_key_version.as_bytes()).unwrap(),
//         );

//         println!("{:?}", payload_str);
//         // let headers = self.get_headers(b64_signed_payload, b64_signed_passphrase, since_the_epoch);
//         println!("{:?}", headers);

//         headers
//     }

//     pub async fn request(
//         &self,
//         endpoint: &str,
//         json: String,
//         method: KucoinRequestType,
//     ) -> Option<KucoinResponseL1> {
//         let base_url: Url = Url::parse("https://api.kucoin.com").unwrap();
//         let url: Url = base_url
//             .join(endpoint)
//             .expect("Was unable to join the endpoint and base_url");

//         match method {
//             KucoinRequestType::Post => {
//                 let res = self
//                     .1
//                     .post(url)
//                     .headers(self.get_headers(String::from("POST"), endpoint, &json))
//                     .send()
//                     .await
//                     .expect("failed to post reqwest")
//                     .text()
//                     .await
//                     .expect("failed to get payload");
//                 self.response(res)
//             }
//             KucoinRequestType::Get => {
//                 let res = self
//                     .1
//                     .get(url)
//                     .headers(self.get_headers(String::from("GET"), endpoint, &json))
//                     .json(&json)
//                     .send()
//                     .await
//                     .expect("failed to get reqwest")
//                     .text()
//                     .await
//                     .expect("failed to get payload");
//                 self.response(res)
//             }
//             KucoinRequestType::WebsocketToken => {
//                 let res = self
//                     .1
//                     .post(url) // TODO: Should be private endpoint and use creds
//                     .headers(self.get_headers(String::from("POST"), endpoint, &json))
//                     .json(&json)
//                     .send()
//                     .await
//                     .expect("failed to post reqwest")
//                     .text()
//                     .await
//                     .expect("failed to get payload");
//                 self.response(res)
//             }
//             KucoinRequestType::OrderPost => {
//                 let res = self
//                     .1
//                     .post(url)
//                     .headers(self.get_headers(String::from("POST"), endpoint, &json))
//                     .json(&json)
//                     .send()
//                     .await
//                     .expect("failed to post reqwest")
//                     .text()
//                     .await
//                     .expect("failed to get payload");
//                 self.response(res)
//             }
//         }
//     }

//     fn response(&self, response: String) -> Option<KucoinResponseL1> {
//         // TODO: maybe parse the status code here and panic with better errors
//         let l1: KucoinResponseL0 = serde_json::from_str(&response).unwrap();
//         if l1.code != 200000 {
//             panic!("Recived Bad Response Status from Kucoin:\n\n{:?}", l1);
//             // None
//         } else {
//             Some(l1.data)
//         }
//     }

//     pub async fn get_pairs(&self) -> Option<KucoinResponseL1> {
//         self.request(
//             "/api/v1/market/allTickers",
//             String::from(""),
//             KucoinRequestType::Get,
//         )
//         .await
//     }

//     pub async fn get_websocket_info(&self) -> Option<KucoinResponseL1> {
//         self.request(
//             "/api/v1/bullet-public", // TODO: This should be private and auth with creds
//             String::from(""),
//             KucoinRequestType::WebsocketToken,
//         )
//         .await
//     }

//     // The clones here are delibrate
//     pub fn clone_keys(&self) -> KucoinCreds {
//         KucoinCreds {
//             api_key: self.0.api_key.to_owned(),
//             api_passphrase: self.0.api_passphrase.to_owned(),
//             api_secret: self.0.api_secret.to_owned(),
//             api_key_version: self.0.api_key_version.to_owned(),
//         }
//     }
// }

// pub fn Kucoin_api(&self, api: Kucoin) {
// fn
// }

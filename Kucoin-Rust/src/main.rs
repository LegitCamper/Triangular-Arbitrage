//use core::future::poll_fn;
use rand::prelude::*;
use std::borrow::Borrow;
use std::env;
//use std::fmt::Write;
//use futures_util::{future, pin_mut, StreamExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};
use std::fs::{read_to_string, remove_file, File};
use std::io::{BufRead, BufReader, Error, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
//use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::{connect, Message};
//use std::os::unix::thread::JoinHandleExt;
use std::path::Path;
//use std::marker::Tuple;
//extern crate libc;
//use std::process;
use duration_string::DurationString;
use itertools::Itertools;
//use reqwest::header::USER_AGENT;
use data_encoding::BASE64;
use ring::{digest, hmac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_this_or_that::as_f64;
//use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use url::Url;

// Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"

fn cwd_plus_path(path: String) -> String {
    let cwd = env::current_dir()
        .expect("Cannot get CWD")
        .display()
        .to_string();
    format!("{}{}", cwd.to_owned(), path.to_owned())
}

#[derive(Debug, Serialize, Deserialize)]
struct KucoinCreds {
    api_key: String,
    api_passphrase: String,
    api_secret: String,
    api_key_version: String,
}

fn get_api_keys<'a>() -> KucoinCreds {
    let json_file_path = cwd_plus_path("/KucoinKeys.json".to_string());
    let data = read_to_string(json_file_path).expect("unable to read KucoinKeys.json");
    let api_creds: KucoinCreds =
        serde_json::from_str(&data).expect("unable to parse KucoinKeys.json");
    api_creds
}

#[derive(Serialize, Debug)]
enum KucoinRequest {
    Get(KucoinRequestGet),
    Post(KucoinRequestPost),
}

#[derive(Serialize, Debug)]
enum KucoinRequestPost {
    Order(KucoinRequestOrderPost),
    Websocket,
}

#[derive(Serialize, Debug)]
struct KucoinRequestGet {
    client_id: u32,
}

#[derive(Serialize, Debug)]
struct KucoinRequestOrderPost {
    order_type: String,
    order_amount: f64,
    order_price: f64,
    order_symbol: String,
    order_side: String,
    client_id: u32,
}

fn get_timestamp<'a>() -> String {
    // Gets current time in milliseconds
    //let since_the_epoch: String = DurationString::from(
    //    SystemTime::now()
    //        .duration_since(UNIX_EPOCH)
    //        .expect("Time went backwards"),
    //)
    //.to_string();
    let since_the_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        .to_string();
    since_the_epoch
}

fn get_signed_json<'a>(
    api_creds: &KucoinCreds,
    since_the_epoch: &String,
    endpoint: &str,
    json: &String,
) -> String {
    let signed_key = hmac::Key::new(hmac::HMAC_SHA256, &api_creds.api_secret.as_bytes());
    let payload = format!("{}+{}+{}+{}", &since_the_epoch, "POST", endpoint, json); // have it as static POST
                                                                                    // because it doest seem
                                                                                    // like the GET needs it
    let signature = hmac::sign(&signed_key, payload.as_bytes());
    let b64_encoded_sig = BASE64.encode(signature.as_ref());
    println!("b64_encoded_sig: {}", b64_encoded_sig);
    b64_encoded_sig
}

async fn kucoin_rest_api<'a>(data: &'a KucoinRequest, endpoint: &str) -> Option<String> {
    let api_creds: KucoinCreds = get_api_keys();

    let json = serde_json::to_string(&data).unwrap();

    let since_the_epoch: String = get_timestamp();

    let b64_encoded_sig: String = get_signed_json(&api_creds, &since_the_epoch, endpoint, &json);

    // adding all the reqiured headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::HeaderName::from_static("KC-API-KEY"),
        reqwest::header::HeaderValue::from_static(&api_creds.api_key),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("KC-API-SIGN"),
        reqwest::header::HeaderValue::from_static(&b64_encoded_sig),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("KC-API-TIMESTAMP"),
        reqwest::header::HeaderValue::from_static(&since_the_epoch),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("API-KEY-PASSPHRASE"),
        reqwest::header::HeaderValue::from_static(&api_creds.api_passphrase),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("KC-API-KEY-VERSION"),
        reqwest::header::HeaderValue::from_static(&api_creds.api_key_version),
    );

    let base_url: Url = Url::parse("https://api.kucoin.com").unwrap();
    let url: Url = base_url
        .join(endpoint)
        .expect("Was unable to join the endpoint and base_url");
    println!("url: {}", url);

    let client = reqwest::Client::new();
    match data {
        Get => {
            let res = client
                .get(url)
                .headers(headers)
                .json(&json)
                .send()
                .await
                .expect("failed to get reqwest")
                .text()
                .await
                .expect("failed to get payload");
            Some(res)
        }
        Post => match Post {
            Order => {
                client
                    .post(url)
                    .headers(headers)
                    .json(&json)
                    .send()
                    .await
                    .expect("failed to post reqwest");
                None
            }
            Webscocket => {
                let res = client
                    .post(url)
                    .headers(headers)
                    .send()
                    .await
                    .expect("failed to post reqwest")
                    .text()
                    .await
                    .expect("failed to get payload");
                Some(res)
            }
        },
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KucoinCoins {
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
    #[serde(deserialize_with = "as_f64")]
    vol: f64,
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
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KucoinCoinsTime {
    ticker: Vec<KucoinCoins>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KucoinCoinsCode {
    data: KucoinCoinsTime,
}

async fn get_tradable_coin_pairs() -> Vec<String> {
    //let api_creds: &'a KucoinCreds = get_api_keys();
    let mut rng = rand::thread_rng();
    let kucoin_request = KucoinRequest::Get(KucoinRequestGet {
        client_id: rng.gen_range(1000..99999), // Generates new random client id
    });
    let kucoin_request_string = kucoin_rest_api(&kucoin_request, "/api/v1/market/allTickers").await;
    let coin_pairs_struct: KucoinCoinsCode = match kucoin_request_string {
        Some(x) => serde_json::from_str(&x.as_str()).expect("JSON was not well-formatted"),
        None => panic!("Failed to get coin pairs"),
    };

    let mut coin_pairs: Vec<String> = Vec::new();

    for i in coin_pairs_struct.data.ticker.iter() {
        coin_pairs.push(i.symbol.clone());
    }
    coin_pairs
}

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

fn has_two_occurrences(arr: &[String], string: &str) -> bool {
    let mut count = 0;
    for s in arr {
        if s == string {
            count += 1;
        }
        if count > 2 {
            return false;
        }
    }
    count == 2
}

fn validate_combination(pairs_list: &[String; 6]) -> bool {
    let pairs_list_len = pairs_list.len() - 1;

    // ensures the pairs can chain together
    let mut chainable: bool = false;
    for i in pairs_list.iter() {
        if has_two_occurrences(&pairs_list[..], i) {
            chainable = true
        } else {
            chainable = false;
            break;
        }
    }

    // ensures first and last pair have a stable coin
    let pairs_list_middle: &[String] = &pairs_list[2..pairs_list_len - 1]; // gets slice of pairs_list
    let mut stable: bool = false;
    for i in STABLE_COINS.iter() {
        let si = i.to_string();
        if si == pairs_list[0]
            || si == pairs_list[1] && si == pairs_list[pairs_list_len - 1]
            || si == pairs_list[pairs_list_len] && pairs_list_middle.contains(&si) == false
        {
            stable = true;
            //println!("{}", pairs_list.contains(&si));
        }
    }
    if stable && chainable {
        true
    } else {
        false
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogStruct {
    vec: Vec<[String; 6]>,
}

async fn create_valid_pairs_catalog(coin_pairs: Vec<String>) {
    // Deletes old pairs catalog and makes new file to write to
    let catalog_file_path = cwd_plus_path("/Triangular_pairs.catalog".to_string());
    if Path::new(&catalog_file_path).exists() {
        remove_file(&catalog_file_path).expect("failed to remove Triangular_pairs.catalog");
    };
    let catalog_file = File::create(&catalog_file_path);
    let mut output_list: Vec<[String; 6]> = Vec::new();

    for i in coin_pairs.iter().combinations(3) {
        let pair1: [&str; 2] = i[0].split("-").collect::<Vec<&str>>().try_into().unwrap();
        let pair2: [&str; 2] = i[1].split("-").collect::<Vec<&str>>().try_into().unwrap();
        let pair3: [&str; 2] = i[2].split("-").collect::<Vec<&str>>().try_into().unwrap();

        let pairs_list = [
            pair1[0].to_string(),
            pair1[1].to_string(),
            pair2[0].to_string(),
            pair2[1].to_string(),
            pair3[0].to_string(),
            pair3[1].to_string(),
        ];
        if validate_combination(&pairs_list) == true {
            output_list.push(pairs_list);
        }
    }
    writeln!(
        &mut catalog_file
            .as_ref()
            .expect("could not open catalog for writing"),
        "{:?}",
        serde_json::to_string(&output_list).unwrap()
    )
    .expect("Failed to write CatalogStruct to catalog");
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

fn find_triangular_arbitrage() {
    let json_file_path = cwd_plus_path("/Triangular_pairs.catalog".to_string());
    //println!("{}", cwd() + "/Triangular_pairs.catalog");
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: CatalogStruct =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");
    //println!("{:?}", triangular_pairs)
}

/////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

fn new_pipe(fifo_path: &str) {
    if Path::new(&fifo_path).exists() {
        remove_file(fifo_path).expect("failed to remove fifo pipe");
    }
}

fn execute_trades() {
    let mut restricted_pairs: Vec<String> = Vec::new(); // Holds pairs that I dont want to trade during runtime
    loop {
        // read named pip and execute orders
    }
}

/////////////////////////////////////////////////////////  Webscocket  /////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct WebsocketToRust {}
#[derive(Debug, Serialize, Deserialize)]
struct WebsocketRustToSerde {}

async fn kucoin_websocket(api_creds: KucoinCreds) {
    // retreive temporary api token
    let websocket_info = kucoin_rest_api(
        &KucoinRequest::Post(KucoinRequestPost::Websocket),
        "/api/v1/bullet-private",
    )
    .await;
    let (mut socket, _response) =
        connect(Url::parse("ws://localhost:8765").unwrap()).expect("Can't connect");
    // Write a message containing "Hello, Test!" to the server
    socket
        .write_message(Message::Text("Hello, Test!".into()))
        .unwrap();

    // Loop forever, handling parsing each message
    loop {
        let msg = socket.read_message().expect("Error reading message");
        let msg = match msg {
            tungstenite::Message::Text(s) => s,
            _ => {
                panic!()
            }
        };
        let parsed: serde_json::Value = serde_json::from_str(&msg).expect("Can't parse to JSON");
        println!("{:?}", parsed["result"]);
    }
}

fn kucoin_websocket_spawner() {
    // tokio task
    // kucoin_websocket()
}

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

// Runs all modules
#[tokio::main]
async fn main() {
    let fifo_path: String = cwd_plus_path("/trades.pipe".to_string());
    //let api_creds: KucoinCreds = get_api_keys();
    let coin_pairs: Vec<String> = get_tradable_coin_pairs().await;

    // Part 1 -- create_valid_pairs
    //create_valid_pairs_catalog(coin_pairs).await
    // Part 2 -- websocket_spawner
    // Part 3 -- find_triangular_arbitrage
    // find_triangular_arbitrage()
    // Part 4 -- execute_trades
    let websocket_token = kucoin_rest_api(
        &KucoinRequest::Post(KucoinRequestPost::Websocket),
        "/api/v1/bullet-private",
    )
    .await;
    println!("{:?}", websocket_token);
}

// Remove warnings while building
use core::future::poll_fn;
use futures::channel::mpsc::Receiver;
use rand::prelude::*;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};
#[allow(unused_imports)]
use std::{
    borrow::Borrow,
    env,
    fs::{read_to_string, remove_file, File},
    io::{BufRead, BufReader, BufWriter, Error, Write},
    path::Path,
    process,
    //os::unix::thread::JoinHandleExt,
    //marker::Tuple,
    //fmt::Write,
    //ffi::CString
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
    sync::oneshot,
    task,
    task::JoinSet,
};
//use futures_util::{future, pin_mut, StreamExt};
//use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::{connect, Message};
//extern crate libc;
use data_encoding::BASE64;
use duration_string::DurationString;
use itertools::Itertools;
use ring::{digest, hmac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_this_or_that::as_f64;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KucoinCreds {
    api_key: String,
    api_passphrase: String,
    api_secret: String,
    api_key_version: String,
}

#[derive(Serialize, Debug)]
enum KucoinRequestType {
    Get,
    Post,
    WebsocketToken,
}

#[derive(Serialize, Debug)]
enum KucoinRequestPost {
    Order(KucoinRequestOrderPost),
    WebsocketToken,
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

#[derive(Debug)]
struct KucoinRequest {
    headers: HeaderMap,
    request: String,
    method: KucoinRequestType,
    endpoint: String,
}

fn construct_kucoin_request(
    endpoint: String,
    json: String,
    method: KucoinRequestType,
) -> KucoinRequest {
    let json_file_path = cwd_plus_path("/KucoinKeys.json".to_string());
    let data = read_to_string(json_file_path).expect("unable to read KucoinKeys.json");
    let api_creds: KucoinCreds =
        serde_json::from_str::<KucoinCreds>(&data).expect("unable to parse KucoinKeys.json");

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

    // signs kucoin_request_string
    let signed_key = hmac::Key::new(hmac::HMAC_SHA256, &api_creds.api_secret.as_bytes());
    let payload = format!("{}+{}+{}+{}", &since_the_epoch, "POST", endpoint, json); // have it as static POST
                                                                                    // because it doest seem
                                                                                    // like the GET needs it
    let signature = hmac::sign(&signed_key, payload.as_bytes());
    let b64_encoded_sig = BASE64.encode(signature.as_ref());
    //println!("b64_encoded_sig: {}", b64_encoded_sig);

    // adding all the reqiured headers
    let mut headers = reqwest::header::HeaderMap::new();
    // headers.insert(
    // reqwest::header::HeaderName::from_static("KC-API-KEY"),
    // reqwest::header::HeaderValue::from_static(&api_creds.api_key),
    // );
    // headers.insert(
    //     reqwest::header::HeaderName::from_static("KC-API-SIGN"),
    //     reqwest::header::HeaderValue::from_static(&'a b64_encoded_sig),
    // );
    // headers.insert(
    //     reqwest::header::HeaderName::from_static("KC-API-TIMESTAMP"),
    //     reqwest::header::HeaderValue::from_static(&since_the_epoch),
    // );
    // headers.insert(
    //     reqwest::header::HeaderName::from_static("API-KEY-PASSPHRASE"),
    //     reqwest::header::HeaderValue::from_static(&api_creds.api_passphrase),
    // );
    // headers.insert(
    //     reqwest::header::HeaderName::from_static("KC-API-KEY-VERSION"),
    //     reqwest::header::HeaderValue::from_static(&api_creds.api_key_version),
    // );

    let base_url: Url = Url::parse("https://api.kucoin.com").unwrap();
    let url: Url = base_url
        .join(&endpoint)
        .expect("Was unable to join the endpoint and base_url");
    match method {
        ref Post => KucoinRequest {
            headers,
            request: json,
            method,
            endpoint: url.to_string(),
        },
        ref Get => KucoinRequest {
            headers,
            request: json,
            method,
            endpoint: url.to_string(),
        },
    }
}

impl KucoinRequest {
    async fn Get(self) -> Option<String> {
        let client = reqwest::Client::new();
        let res = client
            .get(&self.endpoint)
            .headers(self.headers)
            .json(&self.request)
            .send()
            .await
            .expect("failed to get reqwest")
            .text()
            .await
            .expect("failed to get payload");
        Some(res)
    }
    async fn OderPost(self) -> Option<String> {
        let client = reqwest::Client::new();
        client
            .post(&self.endpoint)
            .headers(self.headers)
            .json(&self.request)
            .send()
            .await
            .expect("failed to post reqwest");
        None
    }
    async fn Websocket(self) -> Option<String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&self.endpoint)
            .headers(self.headers)
            .send()
            .await
            .expect("failed to post reqwest")
            .text()
            .await
            .expect("failed to get payload");
        Some(res)
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
    // recursive struct
    data: Box<KucoinCoins>,
    ticker: Box<KucoinCoins>,
}

#[derive(Debug, Serialize)]
struct EmptyKucoinJson {
    string: String,
}

async fn get_tradable_coin_pairs() -> Option<Vec<String>> {
    // TODO
    fn unbox<KucoinCoins>(value: Box<KucoinCoins>) -> KucoinCoins {
        *value
    }
    let mut rng = rand::thread_rng();
    let kucoin_request = construct_kucoin_request(
        "/api/v1/market/allTickers".to_string(),
        //serde_json::from_str("{}").expect("Failed to Serialize"), // apeantly Kucoin Get requests for all tokens needs no params
        "Nothing to see here!".to_string(),
        KucoinRequestType::Get,
    );
    println!("{:?}", kucoin_request);
    match KucoinRequest::Get(kucoin_request).await {
        Some(kucoin_response) => {
            let coin_pairs_struct: KucoinCoins = serde_json::from_str(kucoin_response.as_str())
                .expect("JSON was not well-formatted");
            let coin_pairs = unbox(coin_pairs_struct.data.ticker);
            println!("{:?}", coin_pairs);

            let mut coin_pairs: Vec<String> = Vec::new();

            for i in coin_pairs.iter() {
                //coin_pairs.push(i.symbol.clone());
            }
            //println!("{:?}", coin_pairs);
            Some(coin_pairs)
        }
        None => None,
    }
}

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

fn validate_combination(pairs_list: &[String; 6]) -> bool {
    let pairs_list_len = pairs_list.len() - 1;

    // ensures the pairs can chain together
    let mut chainable: bool = false;
    for i in pairs_list.iter() {
        if {
            let arr: &[String] = &pairs_list[..];
            let mut count = 0;
            for s in arr {
                if s == i {
                    count += 1;
                }
                if count > 2 {
                    return false;
                }
            }
            count == 2
        } {
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
    serde_json::to_writer(
        BufWriter::new(
            File::create(cwd_plus_path("/Triangular_pairs.catalog".to_string()))
                .expect("could not open catalog for writing"),
        ),
        &output_list,
    )
    .expect("Failed to write pair combinations to catalog");
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

fn find_triangular_arbitrage() {
    let json_file_path = cwd_plus_path("/Triangular_pairs.catalog".to_string());
    //println!("{}", cwd() + "/Triangular_pairs.catalog");
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: CatalogStruct =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");
    println!("{:?}", triangular_pairs)
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

// struct to represent the output of symbol data from websocket
#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct KucoinCoinPrices {
    types: String,
    topic: String,
    subject: String,
    data: Box<KucoinCoinPrices>,
    // This is a sub-struct
    //#[serde(deserialize_with = "as_f64")]
    sequence: u32,
    #[serde(deserialize_with = "as_f64")]
    price: f64,
    #[serde(deserialize_with = "as_f64")]
    size: f64,
    #[serde(deserialize_with = "as_f64")]
    bestAsk: f64,
    #[serde(deserialize_with = "as_f64")]
    bestAskSize: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBid: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBidSize: f64,
}

async fn kucoin_websocket(
    websocket_token: String,
    channel_writer: oneshot::Sender<KucoinCoinPrices>,
) {
    let empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    // retreive temporary api token
    let websocket_info = construct_kucoin_request(
        "/api/v1/bullet-public".to_string(),
        serde_json::to_string(&empty_json_request).expect("Failed to Serialize"), // no json params req
        KucoinRequestType::Post,
    );
    KucoinRequest::Get(websocket_info).await;
    let (mut socket, _response) =
        connect(Url::parse("wss://ws-api-spot.kucoin.com/").unwrap()).expect("Can't connect");
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

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

// structs for message passing between tokio tasks
struct websocket_to_validator {
    //symobl_prices: // this should be the struct of the type that websocket returns
}

struct validator_to_buyer {
    price: f32,
    amount: f32,
    // other order params
}

async fn main_spawner(
    coin_pairs: Vec<String>,
    channel_reader: oneshot::Receiver<KucoinCoinPrices>,
) {
    // use tokio tasks and channels to communticate between the fetcher, validator, and buyers
    let (tx, rx) = oneshot::channel(); // initates the channel
    tokio::spawn(async move {
        tx.send(3) // execute websocket and pass result through channel
    });

    match rx.await {
        Ok(v) => println!("Got Websocket data...."), // validate/test the prices to find and arbitrage
        Err(_) => println!("the sender dropped"),
    }
}

#[tokio::main]
async fn main() {
    let empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    let coin_pairs: Vec<String> = match get_tradable_coin_pairs().await {
        Some(x) => x,
        None => panic!("Failed connect to Kucoin and retrive list of coins"),
    };

    let websocket_token = KucoinRequest::Get(construct_kucoin_request(
        "/api/v1/bullet-private".to_string(),
        serde_json::to_string(&empty_json_request).expect("Failed to Serialize"), // no json params req
        KucoinRequestType::Post,
    ))
    .await;
    println!("{:?}", websocket_token);

    // Part 1 -- create_valid_pairs
    //create_valid_pairs_catalog(coin_pairs).await
    // Part 2 -- websocket_spawner
    // Part 3 -- find_triangular_arbitrage
    //find_triangular_arbitrage()
    // Part 4 -- execute_trades

    let (tx, rx) = oneshot::channel::<KucoinCoinPrices>(); // oneshot channel for websocket and validator
                                                           //thread::spawn(|| kucoin_websocket(websocket_token.unwrap(), tx));
    thread::spawn(|| main_spawner(vec!["r".to_string()], rx));
}

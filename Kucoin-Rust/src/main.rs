// Remove warnings while building
#[allow(unused_imports)]
use core::future::poll_fn;
// use futures::channel::mpsc::Receiver;
use rand::prelude::*;
use reqwest::header::HeaderMap;
use std::{
    env,
    fs::{remove_file, File},
    io::{BufRead, BufReader, BufWriter},
    path::Path,
    sync::mpsc,
    //os::unix::thread::JoinHandleExt,
    //marker::Tuple,
    //fmt::Write,
    //ffi::CString
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

//use futures_util::{future, pin_mut, StreamExt};
use futures::StreamExt; // 0.3.13
                        // use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
                        // use tungstenite::{connect, Message};
use workflow_core;
use workflow_websocket;
//extern crate libc;
use data_encoding::BASE64;
use itertools::Itertools;
// use message_io::{
// network::{NetEvent, Transport},
// webscoket library
// node::{self, NodeEvent},
// };
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(Debug, Deserialize)]
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
    endpoint: &str,
    json: String,
    method: KucoinRequestType,
) -> KucoinRequest {
    let creds_file_path = cwd_plus_path("/KucoinKeys.json".to_string());
    let creds_file = File::open(creds_file_path).expect("unable to read KucoinKeys.json");
    let api_creds: KucoinCreds = serde_json::from_reader(BufReader::new(creds_file))
        .expect("unable to parse KucoinKeys.json");

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
    let _b64_encoded_sig: String = BASE64.encode(signature.as_ref());
    //println!("b64_encoded_sig: {}", b64_encoded_sig);

    // adding all the reqiured headers
    let headers = reqwest::header::HeaderMap::new();
    // headers.insert(HeaderName::from_static("KC-API-KEY"), HeaderValue::from_static(&api_creds.api_key.as_str()));
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
        .join(endpoint)
        .expect("Was unable to join the endpoint and base_url");
    match method {
        ref _Post => KucoinRequest {
            headers,
            request: json,
            method,
            endpoint: url.to_string(),
        },
        ref _Get => KucoinRequest {
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
            .get(&*self.endpoint)
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
            .post(&*self.endpoint)
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
            .post(&*self.endpoint)
            .send()
            .await
            .expect("failed to post reqwest")
            .text()
            .await
            .expect("failed to get payload");
        Some(res)
    }
}

// make these three prettier with recursive structs or sum
#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KucoinCoinsL0 {
    data: KucoinCoinsL1,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KucoinCoinsL1 {
    ticker: Vec<KucoinCoinsL2>,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KucoinCoinsL2 {
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

#[derive(Debug, Serialize)]
struct EmptyKucoinJson {
    string: String,
}

async fn get_tradable_coin_pairs() -> Option<Vec<String>> {
    // fn unbox<KucoinCoins>(value: Box<KucoinCoins>) -> KucoinCoins {
    // *value
    // }
    let _rng = rand::thread_rng();
    let kucoin_request = construct_kucoin_request(
        "/api/v1/market/allTickers",
        //serde_json::from_str("{}").expect("Failed to Serialize"), // apeantly Kucoin Get requests for all tokens needs no params
        "Nothing to see here!".to_string(),
        KucoinRequestType::Get,
    );
    match KucoinRequest::Get(kucoin_request).await {
        Some(kucoin_response) => {
            let coin_pairs_struct: KucoinCoinsL0 = serde_json::from_str(kucoin_response.as_str())
                .expect("JSON was not well-formatted");
            let _coin_pairs = coin_pairs_struct.data.ticker;
            // println!("{:?}", coin_pairs);

            // replace with a map and filter statment later
            let coin_pairs: Vec<String> = Vec::new();

            for _i in coin_pairs.iter() {
                // println!("{:?}", coin_pairs);
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
    let _triangular_pairs: CatalogStruct =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");
    // println!("{:?}", triangular_pairs)
}

/////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

fn execute_trades() {
    let _restricted_pairs: Vec<String> = Vec::new(); // Holds pairs that I dont want to trade during runtime
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

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct websocket_detailsL1 {
    data: websocket_detailsL2,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct websocket_detailsL2 {
    token: String,
    instanceServers: Vec<websocket_detailsL3>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct websocket_detailsL3 {
    endpoint: String,
    pingInterval: i32,
    pingTimeout: i32,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct kucoin_websocket_subscription {
    id: i32,
    r#type: String,
    topic: String,
    privateChannel: String,
    response: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct kucoin_webscoket_ping {
    id: i32,
    r#type: String,
}

// websocket responses
enum Websocket_Signal {
    Greet,
    Ping,
}

async fn kucoin_websocket(
    // websocket_token: String,
    _channel_writer: mpsc::Sender<KucoinCoinPrices>,
) {
    let empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    // retreive temporary api token
    let websocket_info = construct_kucoin_request(
        "/api/v1/bullet-public",
        serde_json::to_string(&empty_json_request).expect("Failed to Serialize"), // no json params req
        KucoinRequestType::Post,
    );
    let websocket_info: websocket_detailsL1 =
        serde_json::from_str(&KucoinRequest::Websocket(websocket_info).await.unwrap())
            .expect("Cant't parse from json");

    let websocket_url = Url::parse(
        format!(
            "{}?token={}",
            websocket_info.data.instanceServers[0].endpoint, websocket_info.data.token
        )
        .as_str(),
    )
    .unwrap();

    // Searilize kucoin subscription
    let kucoin_id: i32 = rand::thread_rng().gen_range(13..15);
    let subscription = json!(kucoin_websocket_subscription {
        id: kucoin_id,
        r#type: "subscribe".to_string(),
        topic: "/market/ticker:all".to_string(),
        privateChannel: "false".to_string(),
        response: "false".to_string(),
    });
    // Searilize kucoin ping message
    let ping = json!(kucoin_webscoket_ping {
        id: kucoin_id,
        r#type: "ping".to_string()
    });

    // Webscoket stuff
    let ws = workflow_websocket::client::WebSocket::new(
        &websocket_url.to_string(),
        workflow_websocket::client::Options::default(),
    );
    ws.as_ref()
        .expect("Failed to connect to websocket")
        .connect(true)
        .await;

    // Send messages (Pings and subscription)
    let ws_send = ws.expect("Could not clone ws for sender").clone();
    workflow_core::task::spawn(async move {
        loop {
            ws_send.send(workflow_websocket::client::Message::Text("he".to_string());
            // .send(workflow_websocket::client::Message::Text(ping.to_string()))
            // .await
            // workflow_core::task::sleep(Duration::from_secs(10)).await;
        }
    });

    // Recive messages (Symbol data)
    let ws_read = ws.expect("Could not clone ws for reader").clone();
    workflow_core::task::spawn(async move {
        loop {
            // ws_read
        }
    })
}

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

struct validator_to_buyer {
    price: f32,
    amount: f32,
    // other order params
}

#[tokio::main]
async fn main() {
    let _empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    // gets a list of all the current symbols
    let _coin: Vec<String> = match get_tradable_coin_pairs().await {
        Some(x) => x,
        None => panic!("Failed connect to Kucoin and retrive list of coins"),
    };

    let _handles = Vec::<std::thread::JoinHandle<()>>::new();

    let (websocket_writer, websocket_reader) = mpsc::channel::<KucoinCoinPrices>(); // mpsc channel for websocket and validator
    let websocket_thread = thread::spawn(move || {
        kucoin_websocket(websocket_writer) //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
    });
    let (_validator_writer, _validator_reader) = mpsc::channel::<validator_to_buyer>(); // initates the channel
    let validator_thread = thread::spawn(move || {
        while let Ok(msg) = websocket_reader.recv() {
            println!("{:?}", msg)
        }
    });

    // pauses while threads are running
    websocket_thread.join().unwrap().await;
    validator_thread.join().unwrap();
}

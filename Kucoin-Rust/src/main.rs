// Remove warnings while building
#[allow(unused_imports)]
use core::future::poll_fn;
// use futures::channel::mpsc::Receiver;
use rand::prelude::*;
use reqwest::header::HeaderMap;
use std::{
    collections::HashMap,
    env,
    fs::{read_to_string, remove_file, File},
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
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI" // TODO: this is static rn so dont add to the list
const STARING_AMOUNT: f64 = 100.0; // Staring amount in USD

//////////////////////////////////////////////////// Kucoin Rest API /////////////////////////////////////////////////

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
    OrderPost,
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

async fn kucoin_request(
    endpoint: &str,
    json: String,
    method: KucoinRequestType,
) -> Option<String> {
    let creds_file_path = "KucoinKeys.json".to_string();
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

    // makes http client 
    let client = reqwest::Client::new();

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
            Some(res)
        },
        KucoinRequestType::Get => {
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
        },
        KucoinRequestType::WebsocketToken => {
            let res = client
                .post(url)
                .send()
                .await
                .expect("failed to post reqwest")
                .text()
                .await
                .expect("failed to get payload");
            Some(res)
        },
        KucoinRequestType::OrderPost => {
        client
            .post(url)
            .headers(headers)
            .json(&json)
            .send()
            .await
            .expect("failed to post reqwest");
        None
        }
        
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
    let kucoin_response = kucoin_request(
        "/api/v1/market/allTickers",
        "Nothing to see here!".to_string(),
        KucoinRequestType::Get,
    );
    match kucoin_response.await {
        Some(kucoin_response) => {
            let coin_pairs_struct: KucoinCoinsL0 = serde_json::from_str(kucoin_response.as_str())
                .expect("JSON was not well-formatted");
            let coin_pairs = coin_pairs_struct.data.ticker;

            // TODO: replace with a map and filter statment later
            let mut new_coin_pairs: Vec<String> = Vec::new();

            for i in coin_pairs.iter() {
                new_coin_pairs.push(i.symbol.clone());
            }
            Some(new_coin_pairs)
        }
        None => None,
    }
}

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

async fn create_valid_pairs_catalog() -> Vec<([&String; 3],[String; 6])> {
        // gets a list of all the current symbols
    let coin_pairs: Vec<String> = match get_tradable_coin_pairs().await {
        Some(x) => x,
        None => panic!("Failed connect to Kucoin and retrive list of coins"),
    };
    let (tx, rx) = mpsc::channel(); // makes channel to pass pairs through

    let validator = thread::Builder::new()
        .name("Coin Pair Combinations Validator".to_string())
        .spawn(move || {
            for pair1 in coin_pairs.iter() {
                if !pair1.contains(STABLE_COINS[0]) {
                    // TODO: make dynamic incase I deal with more stable coins
                    continue;
                };
                let pair1_split: [&str; 2] =
                    pair1.split('-').collect::<Vec<&str>>().try_into().unwrap();
                for pair2 in coin_pairs.iter() {
                    if pair2 == pair1 || pair2.contains(STABLE_COINS[0]) {
                        continue;
                    };
                    let pair2_split: [&str; 2] =
                        pair2.split('-').collect::<Vec<&str>>().try_into().unwrap();
                    if pair2_split[0] != pair1_split[0]
                        && pair2_split[0] != pair1_split[1]
                        && pair2_split[1] != pair1_split[0]
                        && pair2_split[1] != pair1_split[1]
                    {
                        continue;
                    };
                    for pair3 in coin_pairs.iter() {
                        if pair3 == pair2 || pair3 == pair1 || !pair3.contains(STABLE_COINS[0]) {
                            continue;
                        }
                        let pair3_split: [&str; 2] =
                            pair3.split('-').collect::<Vec<&str>>().try_into().unwrap();
                        if pair3_split[0] != pair2_split[0]
                            && pair3_split[0] != pair2_split[1]
                            && pair3_split[1] != pair2_split[0]
                            && pair3_split[1] != pair2_split[1]
                        {
                            continue;
                        }

                        let valid_pair = (
                            [pair1, pair2, pair3],
                            [pair1_split[0].to_string(),
                            pair1_split[1].to_string(),
                            pair2_split[0].to_string(),
                            pair2_split[1].to_string(),
                            pair3_split[0].to_string(),
                            pair3_split[1].to_string()],
                        );

                        tx.send(valid_pair).unwrap();
                        // println!("{:?}", valid_pair);
                    }
                }
            }
        });
    // waits for thread to finish before sending valid pairs
    let mut output_list: Vec<([&String; 3], [String; 6])> = Vec::new();
    for received in rx {
        output_list.push(received);
    }
    output_list.clone()
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
enum ArbOrd {
    Buy(String, String), // pair1, pair2, price, size
    Sell(String, String),
}

// TODO: should calulate this during catalog build in the future to prevent waisted IO
fn find_order_order(coin_pair: Vec<String>) -> Vec<ArbOrd> {
    let mut order: Vec<ArbOrd> = vec![];
    
    // get first order
    if coin_pair[0] == coin_pair[2] || coin_pair[0] == coin_pair[3] {
        order.push(ArbOrd::Buy(
            coin_pair[0].to_owned(),
            coin_pair[1].to_owned(),
        ));
    } else if coin_pair[1] == coin_pair[2] || coin_pair[0] == coin_pair[3] {
        order.push(ArbOrd::Sell(
            coin_pair[0].to_owned(),
            coin_pair[1].to_owned(),
        ));
    }
    // get second order
    if coin_pair[2] == coin_pair[4] || coin_pair[2] == coin_pair[5] {
        order.push(ArbOrd::Buy(
            coin_pair[2].to_owned(),
            coin_pair[3].to_owned(),
        ));
    } else if coin_pair[3] == coin_pair[4] || coin_pair[3] == coin_pair[5] {
        order.push(ArbOrd::Sell(
            coin_pair[2].to_owned(),
            coin_pair[3].to_owned(),
        ));
    }
    // get third order
    if coin_pair[4] == coin_pair[0] || coin_pair[4] == coin_pair[1] {
        order.push(ArbOrd::Buy(
            coin_pair[4].to_owned(),
            coin_pair[5].to_owned(),
        ));
    } else if coin_pair[5] == coin_pair[0] || coin_pair[5] == coin_pair[1] {
        order.push(ArbOrd::Sell(
            coin_pair[4].to_owned(),
            coin_pair[5].to_owned(),
        ));
    }
    order
}

// TODO: This assumes they are selling more than I am buying
fn calculate_profitablity( //This also returns price and size
    pair_strings: &Strings,
    order: &Vec<ArbOrd>,
    coin_storage: &HashMap<String, Kucoin_websocket_responseL1>,
) -> (f64, f64, f64) { //profit, price, size
    // TODO: make stable coins dynamic incase I add more
    // transaction 1
    let mut coin_amount = match &order[0] {
        ArbOrd::Buy(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[0]].bestAsk,
                coin_storage[&pair_strings[0]].bestAsk,
                coin_storage[&pair_strings[0]].price,
            )
        }
        ArbOrd::Sell(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[0]].bestBid,
                coin_storage[&pair_strings[0]].bestBid,
                coin_storage[&pair_strings[0]].price,
            )
        }
    };
    // Transaction 2
    coin_amount = match &order[1] {
        ArbOrd::Buy(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[1]].bestAsk,
                coin_storage[&pair_strings[1]].bestAsk,
                coin_storage[&pair_strings[1]].price,
            )
        }
        ArbOrd::Sell(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[1]].bestBid,
                coin_storage[&pair_strings[1]].bestBid,
                coin_storage[&pair_strings[1]].price,
            )
        }
    };
    // Transaction 3
    coin_amount = match &order[2] {
        ArbOrd::Buy(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[2]].bestAsk,
                coin_storage[&pair_strings[2]].bestAsk,
                coin_storage[&pair_strings[2]].price,
            )
        }
        ArbOrd::Sell(pair1, pair2) => {
            (STARING_AMOUNT / coin_storage[&pair_strings[2]].bestBid,
                coin_storage[&pair_strings[2]].bestBid,
                coin_storage[&pair_strings[2]].price,
            )
        }
    };
    coin_amount
}

fn find_triangular_arbitrage(
    valid_coin_pairs: &Vec<([&String; 3],[String; 6])>,
    // coin_fees: CoinFees,
    websocket_reader: mpsc::Receiver<Kucoin_websocket_response>,
    validator_writer: mpsc::Sender<Vec<order_details>>,
) {
    // skipping caluculation for fees - assuming KCS fees are enabled
    // println!("skipping caluculation for fees - assuming KCS fees are enabled");
    
    // Define methode for storing current best prices
    let mut coin_storage: HashMap<String, Kucoin_websocket_responseL1> = HashMap::new();
    while let Ok(msg) = websocket_reader.recv() {
        coin_storage.insert(msg.subject, msg.data);

        // main validator loop
        for pairs_tuple in valid_coin_pairs {    
            let (pairs, pairs_split) = pairs_tupl;
            // loop through data and chekc for arbs
            if coin_storage
                .get(&pairs[0])
                .is_some()
                && coin_storage
                    .get(&pairs[1])
                    .is_some()
                && coin_storage
                    .get(&pairs[2])
                    .is_some()
            {
                // anything in here has been garenteed to be in coin_storage
                // TODO: Consider checking timestamp here. future iterations
                let order_order = find_order_order(pairs.to_vec());
                let profit = calculate_profitablity(&pair_strings, &order_order, &coin_storage);
                if profit.0 - STARING_AMOUNT >= 0.01 {
                    // orders = Vec[ArbOrd, (
                        
                    // ), (), ()]
                    // validator_writer.send(order_order);
                    // println!("profit: {profit}");
                }
            }
        }
    }
}

/////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

fn execute_trades(validator_reader: mpsc::Receiver<Vec<order_details>>) {
    loop {
        println!("READ FROM VALIDATOR: {:?}", validator_reader)
    }
}

/////////////////////////////////////////////////////////  Webscocket  /////////////////////////////////////////////////////////

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

// Kucoin websocket return - Serde
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Kucoin_websocket_response {
    r#type: String,
    topic: String,
    subject: String,
    data: Kucoin_websocket_responseL1,
}
#[derive(Clone)]
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Kucoin_websocket_responseL1 {
    #[serde(deserialize_with = "as_f64")]
    bestAsk: f64,
    #[serde(deserialize_with = "as_f64")]
    bestAskSize: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBid: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBidSize: f64,
    #[serde(deserialize_with = "as_f64")]
    price: f64,
    #[serde(deserialize_with = "as_f64")]
    sequence: f64,
    #[serde(deserialize_with = "as_f64")]
    size: f64,
    #[serde(deserialize_with = "as_f64")]
    time: f64,
}

async fn kucoin_websocket(
    // websocket_token: String,
    channel_writer: mpsc::Sender<Kucoin_websocket_response>,
) {
    let empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    // retreive temporary api token
    let websocket_info: websocket_detailsL1 = match kucoin_request(
        "/api/v1/bullet-public",
        serde_json::to_string(&empty_json_request).expect("Failed to Serialize"), // no json params req
        KucoinRequestType::Post,
    ).await {
        Some(x) =>
            serde_json::from_str(&x).expect("Cant't parse from json"),
        None => panic!("Did not get valid response from kucoin")
    };

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
        .await
        .unwrap();
    ws.as_ref()
        .expect("")
        .send(workflow_websocket::client::Message::Text(
            subscription.to_string(),
        ))
        .await
        .expect("Failed to subscribe to the websocket");

    // Send messages (Pings and subscription)
    let ws_send = ws.as_ref().expect("Could not clone ws for sender").clone();
    workflow_core::task::spawn(async move {
        loop {
            ws_send
                .send(workflow_websocket::client::Message::Text(ping.to_string()))
                .await
                .expect("Failed to send ping to websocket");
            workflow_core::task::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Recive messages (Symbol data)
    let ws_read = ws.expect("Could not clone ws for reader").clone();
    workflow_core::task::spawn(async move {
        loop {
            let response = ws_read.recv();
            if let Ok(workflow_websocket::client::Message::Text(x)) = response.await {
                if x.contains("message") {
                    // println!("{}", x);
                    let response: Kucoin_websocket_response =
                        serde_json::from_str(x.as_str()).expect("Cannot desearlize websocket data");
                    println!("{:?}", response); // TODO: remove this
                    channel_writer.send(response).unwrap()
                } else {
                    println!("Webosocket Response: {}", x)
                }
            }
        }
    });

    // wait for the tasks to finish (forever)
    tokio::time::sleep(std::time::Duration::MAX).await
}

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
struct CoinFees {
    class_a: CoinFeesL1,
    class_b: CoinFeesL1,
    class_c: CoinFeesL1,
}
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
#[allow(dead_code)]
struct CoinFeesL1 {
    regular_maker: f64,
    regular_taker: f64,
    KCS_maker: f64,
    KCS_taker: f64,
    coins: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Dont need coin_fees because I assume they pay fees with KCS
    // let coin_fees_path = cwd_plus_path("/coinfees.json".to_string());
    // let coin_fees_string = read_to_string(coin_fees_path).expect("Unable to read file");
    // let coin_fees: CoinFees =
        // serde_json::from_str(&coin_fees_string).expect("Failed to Desearlize coin_fees");

    // Gets valid pair combinations
    let pair_combinations = create_valid_pairs_catalog().await; // creates json with all the coins
    println!("Generated Valid Coin Pairs successfully");

    let (websocket_writer, websocket_reader) = mpsc::channel::<Kucoin_websocket_response>(); // mpsc channel for websocket and validator
    let websocket_thread = thread::Builder::new()
        .name("Websocket Thread".to_string())
        .spawn(move || {
            kucoin_websocket(websocket_writer) //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
        })
        .unwrap();
    let (validator_writer, validator_reader) =
        mpsc::channel::<Vec<order_details>>(); // initates the channel
    let validator_thread = thread::Builder::new()
        .name("Validator Thread".to_string())
        .spawn(move || {
            find_triangular_arbitrage(
                &pair_combinations,
                // coin_fees,
                websocket_reader,
                validator_writer,
            );
        })
        .unwrap();
    let ordering_thread = thread::Builder::new()
        .name("Ordering Thread".to_string())
        .spawn(move || execute_trades(validator_reader))
        .unwrap();

    // pauses while threads are running
    websocket_thread.join().unwrap().await;
    validator_thread.join().unwrap();
    ordering_thread.join().unwrap();
}

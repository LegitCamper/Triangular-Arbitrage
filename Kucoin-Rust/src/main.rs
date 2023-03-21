//use core::future::poll_fn;
use rand::prelude::*;
use std::env;
//use std::fmt::Write;
use std::fs::{read_to_string, remove_file, File};
use std::io::{BufRead, BufReader, Error, Write};
//use std::os::unix::thread::JoinHandleExt;
use std::path::Path;
//use std::marker::Tuple;
//extern crate libc;
//use std::process;
use duration_string::DurationString;
use itertools::Itertools;
//use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_this_or_that::as_f64;
//use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use url::Url;

// Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"

async fn coin_pairs() -> Vec<String> {
    get_tradable_coin_pairs().await
}

fn cwd_plus_path(path: String) -> String {
    let cwd = env::current_dir()
        .expect("Cannot get CWD")
        .display()
        .to_string();
    format!("{}{}", cwd.to_owned(), path.to_owned())
}

fn get_api_keys() -> (String, String, String, String) {
    let json_file_path = cwd_plus_path("/KucoinKeys.json".to_string());
    let data = read_to_string(json_file_path).expect("unable to read KucoinKeys.json");
    let api_keys: serde_json::Value =
        serde_json::from_str(&data).expect("unable to parse KucoinKeys.json");
    let api_key: String = api_keys["keys"].to_string();
    let api_secret: String = api_keys["secret"].to_string();
    let mut api_passphrase: String = api_keys["passphrase"].to_string();
    api_passphrase = "".to_string(); // need to encode with base64 and encrypt with secret
    let api_key_version = "2".to_string();

    // Gets current time in milliseconds
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let since_the_epoch = DurationString::from(since_the_epoch).into();

    // Returns Login Creds
    (api_key, api_passphrase, api_key_version, since_the_epoch)
}

#[derive(Serialize, Debug)]
enum KucoinRequest {
    Get(KucoinRequestGet),
    Post(KucoinRequestPost),
}

#[derive(Serialize, Debug)]
struct KucoinRequestGet {
    client_id: u32,
}

#[derive(Serialize, Debug)]
struct KucoinRequestPost {
    order_type: String,
    order_amount: f64,
    order_price: f64,
    order_symbol: String,
    order_side: String,
    client_id: u32,
}

async fn kucoin_rest_api(data: KucoinRequest, endpoint: &str) -> String {
    let (api_key, api_passphrase, api_key_version, api_timestamp) = get_api_keys();

    let json = serde_json::to_string(&data).unwrap();

    let base_url: Url =
        Url::parse("https://api.kucoin.com/").expect("Was unable to parse base_url");
    let url: Url = base_url
        .join(endpoint)
        .expect("Was unable to join the endpoint and base_url");

    let client = reqwest::Client::new();
    match data {
        Get => {
            let res = client
                .get(url)
                .header("API_KEY", api_key)
                .header("API_PASSPHRASE", api_passphrase)
                .header("API_KEY_VERSION", api_key_version)
                .header("API_TIMESTAMP", api_timestamp)
                .json(&json)
                .send()
                .await
                .expect("failed to get reqwest")
                .text()
                .await
                .expect("failed to get payload");
            res
        }
        Post => {
            client
                .post(url)
                .header("API_KEY", api_key)
                .header("API_PASSPHRASE", api_passphrase)
                .header("API_KEY_VERSION", api_key_version)
                .header("API_TIMESTAMP", api_timestamp)
                .json(&json)
                .send()
                .await
                .expect("failed to post reqwest");
            "Okay".to_string()
        }
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
    let mut rng = rand::thread_rng();
    let kucoin_request = KucoinRequest::Get(KucoinRequestGet {
        client_id: rng.gen_range(1000..99999), // Generates new random client id
    });
    let kucoin_request_string = kucoin_rest_api(kucoin_request, "/api/v1/market/allTickers").await;
    let coin_pairs_struct: KucoinCoinsCode =
        serde_json::from_str(&kucoin_request_string.as_str()).expect("JSON was not well-formatted");

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
    let catalog_file = File::create(&catalog_file_path);
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
        remove_file(fifo_path);
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
struct websocket_serde {

}

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

// Runs all modules
#[tokio::main]
async fn main() {
    let coin_pairs: Vec<String> = coin_pairs().await;
    let fifo_path: String = cwd_plus_path("/trades.pipe".to_string());

    // Part 1 -- create_valid_pairs
    //create_valid_pairs_catalog(coin_pairs).await
    // Part 2 -- websocket_spawner
    // Part 3 -- find_triangular_arbitrage
    // find_triangular_arbitrage()
    // Part 4 -- execute_trades
}

use core::future::poll_fn;
use rand::prelude::*;

use std::env;
use std::fmt::Display;
use std::fs::{read_to_string, remove_file, File};
use std::os::unix::process;
use std::path::Path;
extern crate libc;
use duration_string::DurationString;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

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

#[derive(Serialize, Deserialize, Debug)]
struct KucoinRequest {
    get_or_post: String,
    get_symbols: bool,
    order_type: String,
    order_amount: f64,
    order_price: f64,
    order_symbol: String,
    order_side: String,
    client_id: u32,
}

async fn kucoin_rest_api(data: KucoinRequest, endpoint: &str) -> String {
    println!("{}", data.get_or_post);
    let (api_key, api_passphrase, api_key_version, api_timestamp) = get_api_keys();
    println!("{}", api_passphrase);
    let json = serde_json::to_string(&data).unwrap();

    let base_url: Url =
        Url::parse("https://api.kucoin.com/").expect("Was unable to parse base_url");
    let url: Url = base_url
        .join(endpoint)
        .expect("Was unable to join the endpoint and base_url");

    let client = reqwest::Client::new();
    if data.get_or_post == "get" {
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
        println!("{}", res);
        res
    } else if data.get_or_post == "post" {
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
    } else {
        println!("Invalid get_or_post");
        "None".to_string()
    }
}

// Cahnge the output type
fn get_tradable_coin_pairs() -> String {
    let mut rng = rand::thread_rng();
    let kucoin_request = KucoinRequest {
        get_or_post: "get".to_string(),
        get_symbols: true,
        client_id: rng.gen_range(1000..99999), // Generates new random client id
        order_amount: 0.0,
        order_price: 0.0,
        order_side: "None".to_string(),
        order_symbol: "None".to_string(),
        order_type: "None".to_string(),
    };
    kucoin_rest_api(kucoin_request, "/api/v1/market/allTickers");
    //println!("{}", none);
    "ffs".to_string()
}

fn valid_combinations_3() {
    // make all possible combinations of 3 coins here
}

fn valid_combinations_4() {
    // make all possible combinations of 4 coins here
}

fn vailid_combinations_5() {
    // make all possible combinations of 5 coins here
}

fn create_valid_pairs_catalog() {
    // Deletes old pairs catalog and makes new file to write to
    let json_file_path = cwd_plus_path("/Triangular_pairs.catalog".to_string());
    if Path::new(&json_file_path).exists() {
        remove_file(&json_file_path).expect("failed to remove Triangular_pairs.catalog");
    };
    let json_file = Path::new(&json_file_path).exists();
    let catalog_output: Vec<Vec<String>> = Vec::new(); // Holds the outputs of all Triangular pairs for printing

    let coin_pairs = get_tradable_coin_pairs();
    println!("{:?}", coin_pairs);
}

fn main() {
    let fifo_path: String = cwd_plus_path("/trades.pip".to_string());
    println! {"{}", fifo_path} // ensure this is correct
                               //create_valid_pairs_catalog()
    let mut rng = rand::thread_rng();
    let kucoin_request = KucoinRequest {
        get_or_post: "get".to_string(),
        get_symbols: true,
        client_id: rng.gen_range(1000..99999), // Generates new random client id
        order_amount: 0.0,
        order_price: 0.0,
        order_side: "None".to_string(),
        order_symbol: "None".to_string(),
        order_type: "None".to_string(),
    };
    let l = poll_fn(kucoin_rest_api(kucoin_request, "/api/v1/market/allTickers"));
    println!("{}", l);
}


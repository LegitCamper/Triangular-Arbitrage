use rand::prelude::*;
use std::env;
use std::fmt::Display;
use std::fs::{remove_file, File};
use std::path::Path;
extern crate libc;
use reqwest::header::USER_AGENT;
use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};

// The other coins cause isses, should just still to USDT
const STABLE_COINS: Vec<&str> = vec!["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"

fn file_plus_cwd(file: String) -> String {
    let mut cwd = env::current_dir();
    let cwd = cwd
        .expect("Cannot get CWD")
        .display()
        .to_string()
        .insert_str(0, &file.as_str());
    //.to_string();
    cwd
}

const FIFO_PATH: String = file_plus_cwd("/trades.pip".to_string());
//println!{"{}", fifo_path} // ensure this is correct

struct KucoinLogin {
    api_key: String,
    api_passphrase: String,
    api_timestamp: String,
    api_key_version: String,
}

fn get_api_keys() -> KucoinLogin {
    let json_file_path = file_plus_cwd("/KucoinKeys.json".to_string());
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("KucoinKeys.json not found");
    let api_keys: Vec<String> =
        serde_json::from_reader(file).expect("error while reading KucoinKeys.json");
    let api_secret: String = "".to_string();
    let api_passphrase = "".to_string(); // need to encode with base64 and encrypt with secret

    // Gets current time in milliseconds
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // Returns Login Creds
    KucoinLogin {
        api_key: api_keys[0],
        api_passphrase: api_keys[1],
        api_timestamp: api_keys[2],
        api_key_version: api_keys[3],
    }
}

struct KucoinRequest {
    endpoint: String,
    get_or_post: String,
    get_symbols: bool,
    order_type: String,
    order_amount: f64,
    order_price: f64,
    order_symbol: String,
    order_side: String,
    client_id: u32,
}

async fn kucoin_rest_api(data: KucoinRequest) {
    let mut endpoint = String::from("https://api.kucoin.com/");
    let endpoint = endpoint.insert_str(endpoint.len(), &data.endpoint);
    //.to_string();
    let api_creds = get_api_keys();

    let mut headers = reqwest::header::HeaderMap::new();
    let json = serde_json::to_string(&data).expect("Failed to make json body");

    let client = reqwest::Client::new();
    let response = if data.get_or_post == "get" {
        let result = client
            .get(data.endpoint)
            // Include all the api headers
            .header("KC-API-KEY", api_creds.api_key)
            .header("KC-API-SIGN", bas64signed)
            .header("KC-API-PASSPHRASE", api_creds.api_passphrase)
            .header("KC-API-KEY-VERSION", api_creds.api_key_version)
            .header("KC-API-TIMESTAMP", api_creds.api_timestamp)
            .json(&json) // this needs to be json of Kucoin_Request minus endpoit
            .send()
            .await;
        // Returns kucoin request
        result
    } else if data.get_or_post == "post" {
        let res = client
            .post(endpoint)
            // Include all the api headers
            .header("KC-API-KEY", api_creds.api_key)
            .header("KC-API-SIGN", bas64signed)
            .header("KC-API-PASSPHRASE", api_creds.api_passphrase)
            .header("KC-API-KEY-VERSION", api_creds.api_key_version)
            .header("KC-API-TIMESTAMP", api_creds.api_timestamp)
            .json(&json) // this needs to be json of Kucoin_Request minus endpoit
            .send()
            .await
            .unwrap();
    };
    response
}

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

fn get_tradable_coin_pairs() {
    // let mut coin_pairs;
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
    let mut rng = rand::thread_rng();
    let kucoin_request = KucoinRequest {
        endpoint: "https://api.kucoin.com/api/v1/market/allTickers".to_string(),
        get_or_post: "get".to_string(),
        get_symbols: true,
        client_id: rng.gen_range(1000..99999), // Generates new random client id
        order_amount: 0.0,
        order_price: 0.0,
        order_side: "None".to_string(),
        order_symbol: "None".to_string(),
        order_type: "None".to_string(),
    };
    let all_coin_pairs = kucoin_rest_api(kucoin_request);
    // Deletes old pairs catalog and makes new file to write to
    let json_file_path = file_plus_cwd("/Triangular_pairs.catalog".to_string());
    if Path::new("/etc/hosts").exists() {
        remove_file(json_file_path);
    };
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: Vec<Vec<String>> =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");

    let mut catalog_output: Vec<Vec<String>> = Vec::new(); // Holds the outputs of all Triangular pairs for printing
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

fn find_triangular_arbitrage() {
    let json_file_path = file_plus_cwd("/Triangular_pairs.catalog".to_string());
    //println!("{}", cwd() + "/Triangular_pairs.catalog");
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: Vec<Vec<String>> =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");
    //println!("{:?}", triangular_pairs)
}

/////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

fn new_pipe() {
    if Path::new(&FIFO_PATH).exists() {
        remove_file(FIFO_PATH);
    }
    let filename = CString::new(FIFO_PATH.clone()).unwrap();
    unsafe {
        libc::mkfifo(filename.as_ptr(), 0o644);
    }
}

fn execute_trades() {
    let mut restricted_pairs: Vec<String> = Vec::new(); // Holds pairs that I dont want to trade during runtime
    loop { // loops are infinite loops
         // read named pip and execute orders
    }
}

/////////////////////////////////////////////////////////  Main  /////////////////////////////////////////////////////////

// Runs all modules
fn main() {
    // Part 1 -- create_valid_pairs
    create_valid_pairs_catalog()
    // Part 2 -- websocket_spawner
    // Part 3 -- find_triangular_arbitrage
    // find_triangular_arbitrage()
    // Part 4 -- execute_trades
}

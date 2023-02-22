use std::env;
use std::fs::{File, remove_file};
use std::path::Path;
use rand::prelude::*;
extern crate libc;
use std::ffi::CString;
use reqwest::header::USER_AGENT;
use std::time::{SystemTime, UNIX_EPOCH};

// The other coins cause isses, should just still to USDT
const STABLE_COINS: Vec::new() = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"

fn cwd() -> String {
    let path = env::current_dir();
    path.expect("Cannot get CWD").display().to_string()
}

const fifo_path: String = cwd() + "/trades.pip";

struct Api_Login {
    api_key: String,
    api_passphrase: String,
    api_key_version: u8,
    api_timestamp: String, // Might need to be an int
}

fn get_api_keys() -> Api_Login {
    let json_file_path: String = cwd() + "/KucoinKeys.json";
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("KucoinKeys.json not found");
    let api_keys: Vec<String> =
        serde_json::from_reader(file).expect("error while reading KucoinKeys.json");
    let api_key: String = api_keys["kucoinApiKey"];
    let api_secret: String = api_keys["kucoinApiSecret"];
    let api_passphrase: String = api_keys["kucoinApiPassphrase"];
    api_passphrase = 0; // need to encode with base64 and encrypt with secret 

    // Gets current time in milliseconds
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // Returns Login Creds
    let api_login = Api_Login {
        api_key: api_key,
        api_passphrase: api_passphrase,
        api_key_version: 2,
        api_timestamp: since_the_epoch 
    };
    api_login
}

struct Kucoin_Request {
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

async fn kucoin_rest_api(data: Kucoin_Request) {
    let api_keys = get_api_keys();

    let mut headers = reqwest::header::HeaderMap::new();

    let client = reqwest::Client::new();
    let response = if data[get_or_post] == "get" {
        let result = client.get("http://httpbin.org/post")
        .header(api_keys)
        .json(&data) // this needs to be json of Kucoin_Request minus endpoit
        .send()
        .await;
        // Returns kucoin request
        result
    } else if data["get_or_post"] == "post" {
        let res = client.post("http://httpbin.org/post")
        .header(api_keys)
        .json(data) // this needs to be json of Kucoin_Request minus endpoit
        .send()
        .await;
    }
    response
}

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

fn get_tradable_coin_pairs() {
    let mut coin_pairs = Vec::new();
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
    let kucoin_request = Kucoin_Request {
        endpoint: "https://api.kucoin.com/api/v1/market/allTickers", 
        get_or_post: "get" 
        get_symbols: true,
        client_id: rng.gen_range(1000..99999), // Generates new random client id
    }
    let all_coin_pairs = kucoin_rest_api(kucoin_request);
    // Deletes old pairs catalog and makes new file to write to
    let json_file_path: String = cwd() + "/Triangular_pairs.catalog";
    if Path::new("/etc/hosts").exists() {
        remove_file(json_file_path)?;
    }
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: Vec<Vec<String>> =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");

    let mut catalog_output = Vec::new(); // Holds the outputs of all Triangular pairs for printing
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

fn find_triangular_arbitrage() {
    let json_file_path: String = cwd() + "/Triangular_pairs.catalog";
    //println!("{}", cwd() + "/Triangular_pairs.catalog");
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("Triangular_pairs.catalog not found");
    let triangular_pairs: Vec<Vec<String>> =
        serde_json::from_reader(file).expect("error while reading Triangular_pairs.catalog");
    //println!("{:?}", triangular_pairs)
}

/////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

fn new_pipe() {
    if Path::new(&fifo_path).exists() {
        remove_file(fifo_path);
    }
    let filename = CString::new(fifo_path.path.clone()).unwrap();
    unsafe {
            libc::mkfifo(filename.as_ptr(), 0o644);
    }
}

fn execute_trades() {
    let mut restricted_pairs = Vec::new(); // Holds pairs that I dont want to trade during runtime
    while true {
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

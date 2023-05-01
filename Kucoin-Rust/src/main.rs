// Libraries
use std::{sync::mpsc, thread};

// my Libraries
use kucoin_arbitrage::{
    create_valid_pairs_catalog, execute_trades, find_triangular_arbitrage, kucoin_websocket,
};

#[tokio::main]
async fn main() {
    // Gets valid pair combinations
    let pair_combinations = create_valid_pairs_catalog().await; // creates json with all the coins
    println!("Generated Valid Coin Pairs successfully");

    let (websocket_writer, websocket_reader) = mpsc::channel(); // mpsc channel for websocket and validator
    let websocket_thread = thread::Builder::new()
        .name("Websocket Thread".to_string())
        .spawn(move || {
            kucoin_websocket(websocket_writer) //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
        })
        .unwrap();
    websocket_thread.join().unwrap().await;

    let (validator_writer, validator_reader) = mpsc::channel(); // initates the channel
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
    validator_thread.join().unwrap();

    let ordering_thread = thread::Builder::new()
        .name("Ordering Thread".to_string())
        .spawn(move || execute_trades(validator_reader))
        .unwrap();
    ordering_thread.join().unwrap().await;
}

use log::info;
use simple_logger::SimpleLogger;

use tokio::signal;

use func::{create_valid_pairs_catalog, find_triangular_arbitrage, read_key};
use websocket::{start_market_websockets, start_user_websocket};

mod func;
mod interface;
mod tests;
use interface::BinanceInterface;
mod websocket;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_colors(true)
        .init()
        .unwrap();

    info!("Starting Binance Tri-Trader Cli");

    let interface = BinanceInterface::new();

    let symbols = interface.get_symbols().await.unwrap();
    let pairs = interface.get_pairs().await.unwrap();

    let pair_combinations = create_valid_pairs_catalog(pairs).await;
    let orderbook = interface.starter_orderbook(&symbols).await;
    let (ord_handle, ord_sort_handle) = start_market_websockets(orderbook.clone(), &symbols).await;
    let (user_handle, user_channel) = start_user_websocket(read_key()).await;
    // let validator_task =
    // find_triangular_arbitrage(pair_combinations, user_channel, orderbook.clone()).await;

    tokio::select! {
        _ = signal::ctrl_c() => {}
    }
    ord_handle.abort();
    for handle in ord_sort_handle.iter() {
        handle.abort()
    }
    // validator_task.abort();
    user_handle.abort();
    // ordering_task.abort();
    println!("Exiting - Bye!");
}

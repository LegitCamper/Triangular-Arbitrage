use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time,
};

use log::{info, LevelFilter::Info};
use simple_logger::SimpleLogger;
use tokio::signal;

use func::{create_valid_pairs_catalog, find_triangular_arbitrage, read_key};
use websocket::{start_market_websockets, start_order_placer};

mod func;
mod interface;
use interface::BinanceInterface;
mod websocket;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(Info)
        .with_colors(true)
        .init()
        .unwrap();

    info!("Starting Binance Tri-Trader Cli");

    let keep_running = Arc::new(AtomicBool::new(true));

    let interface = BinanceInterface::new();
    let exchange_info = interface.get_exchange_info().await.unwrap();
    let server_time = interface.get_server_time().await.unwrap();
    let symbols = interface.get_symbols().await.unwrap();
    let pairs = interface.get_pairs().await.unwrap();
    let orderbook = interface.starter_orderbook(&symbols).await;

    let pair_combinations = create_valid_pairs_catalog(pairs).await;
    let (ord_handle, ord_sort_handle) =
        start_market_websockets(keep_running.clone(), orderbook.clone(), &symbols).await;
    let (user_handle, user_channel, user_websocket_handle) = start_order_placer(
        keep_running.clone(),
        read_key(),
        &exchange_info,
        &server_time,
    )
    .await;
    let validator_task = find_triangular_arbitrage(
        pair_combinations,
        user_channel,
        orderbook.clone(),
        exchange_info.clone(),
    )
    .await;

    // Handle closing
    tokio::select! {
        _ = signal::ctrl_c() => {}
    }
    info!("Closing...");
    keep_running.store(false, std::sync::atomic::Ordering::Relaxed);
    sleep(time::Duration::from_secs(5));
    ord_handle.abort();
    for handle in ord_sort_handle.iter() {
        handle.abort()
    }
    user_handle.abort();
    user_websocket_handle.abort();
    validator_task.abort();
    info!("Exiting - Bye!");
}

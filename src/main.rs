use std::{
    process::exit,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time,
};

use log::{info, LevelFilter::Info};
use simple_logger::SimpleLogger;
use tokio::signal;

use func::{create_valid_pairs_catalog, find_tri_arb, read_key};
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

    let key = read_key();
    let interface = BinanceInterface::new(&key, false);
    let exchange_info = interface.get_exchange_info().await.unwrap();
    let trading_fees = interface.get_account_fees().await.unwrap();
    let server_time = interface.get_server_time().await.unwrap();
    let (pairs, symbols) = interface.get_pairs().await.unwrap();
    let (pairs, symbols) = (pairs.as_slice(), symbols.as_slice());
    let orderbook = interface.starter_orderbook(&pairs).await;

    let pair_combinations = create_valid_pairs_catalog(pairs).await;
    let (ord_handle, ord_sort_handle) =
        start_market_websockets(keep_running.clone(), orderbook.clone(), &symbols).await;
    let (user_handle, user_channel, user_websocket_handle) =
        start_order_placer(keep_running.clone(), key, &exchange_info, &server_time).await;
    let validator_task = find_tri_arb(
        pair_combinations,
        user_channel,
        orderbook.clone(),
        exchange_info.clone(),
        trading_fees,
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
    exit(1);
}

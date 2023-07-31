use log::info;
use simple_logger::SimpleLogger;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    signal,
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
    time::{interval, Duration},
};

mod func;
mod interface;
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

    let pair_combinations = func::create_valid_pairs_catalog(pairs).await;
    let orderbook = interface.starter_orderbook(&symbols).await;
    let (ord_handle, ord_sort_handle) =
        websocket::start_websocket(orderbook.clone(), &symbols).await;

    let (validator_writer, validator_reader) = mpsc::unbounded_channel(); // channel to execute order
    let mut interval = interval(Duration::from_millis(100)); //TODO: experiment with this
    let validator_task = task::spawn(async move {
        func::find_triangular_arbitrage(&pair_combinations, validator_writer, orderbook.clone())
            .await;
    });

    // let ordering_task =
    //     task::spawn(async move { execute_trades(binance_interface, validator_reader).await });

    tokio::select! {
        _ = signal::ctrl_c() => {}
    }
    ord_handle.abort();
    for handle in ord_sort_handle.iter() {
        handle.abort()
    }
    // websocket_task.abort();
    // validator_task.abort();
    // ordering_task.abort();
    println!("Exiting - Bye!");
}

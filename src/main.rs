use log::info;
use simple_logger::SimpleLogger;

use tokio::{signal, sync::mpsc};

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
        websocket::start_websockets(orderbook.clone(), &symbols).await;

    let (validator_writer, _validator_reader) = mpsc::unbounded_channel(); // channel to execute order
    let validator_task =
        func::find_triangular_arbitrage(pair_combinations, validator_writer, orderbook.clone())
            .await;

    // let ordering_task =
    // task::spawn(async move { execute_trades(binance_interface, validator_reader).await });

    tokio::select! {
        _ = signal::ctrl_c() => {}
    }
    ord_handle.abort();
    for handle in ord_sort_handle.iter() {
        handle.abort()
    }
    validator_task.abort();
    // ordering_task.abort();
    println!("Exiting - Bye!");
}

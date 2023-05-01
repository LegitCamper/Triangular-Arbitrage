// Libraries
use tokio::{runtime::Builder, sync::mpsc}; //, task};

// my Libraries
use kucoin_arbitrage::{
    create_valid_pairs_catalog, execute_trades, find_triangular_arbitrage, kucoin_websocket,
};

#[tokio::main]
async fn main() {
    // Gets valid pair combinations
    let pair_combinations = create_valid_pairs_catalog().await; // creates json with all the coins
    println!("Generated Valid Coin Pairs successfully");

    // build runtime - ensure tasks are being allocated their own thread
    let runtime = Builder::new_multi_thread()
        // .worker_threads(4)
        .enable_all()
        .thread_name("arbitrage-caluclator")
        .build()
        .unwrap();

    let (websocket_writer, websocket_reader) = mpsc::channel(100); // mpsc channel for websocket and validator
    let websocket_task = runtime.spawn(async move {
        kucoin_websocket(websocket_writer).await //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
    });

    let (validator_writer, validator_reader) = mpsc::channel(1); // initates the channel
    let validator_task = runtime.spawn(async move {
        find_triangular_arbitrage(
            &pair_combinations,
            // coin_fees,
            websocket_reader,
            validator_writer,
        )
        .await;
    });

    let ordering_task = runtime.spawn(async move { execute_trades(validator_reader).await });

    // await tasks
    websocket_task.await.unwrap();
    validator_task.await.unwrap();
    ordering_task.await.unwrap();
}

// Libraries
use tokio::{runtime::Builder, sync::mpsc}; //, task};

// my Libraries
// use kucoin_arbitrage::{
//     create_valid_pairs_catalog, execute_trades, find_triangular_arbitrage, get_kucoin_creds,
//     kucoin_websocket, KucoinCreds,
// };

use kucoin_arbitrage::kucoin_interface::{self as kucoin_api, KucoinInterface, KucoinRequestType};

#[tokio::main]
async fn main() {
    let kucoin_interface = KucoinInterface::new();

    // retreive temporary websocket token
    let websocket_info = &kucoin_interface
        .request(
            "/api/v1/bullet-public", // TODO: This should be private and auth with creds
            String::from(""),
            KucoinRequestType::WebsocketToken,
        )
        .await;

    // Gets valid pair combinations
    let pair_combinations = kucoin_interface.get_pairs().await; // creates json with all the coins
    println!("Generated Valid Coin Pairs successfully");
    println!("{:?}", pair_combinations);

    // // build runtime - ensure tasks are being allocated their own thread
    // let runtime = Builder::new_multi_thread()
    //     // .worker_threads(4)
    //     .enable_all()
    //     .thread_name("arbitrage-calculator")
    //     .build()
    //     .unwrap();

    // let (kucoin_writer, kucoin_reader) - mpsc::channel(100); // channel to communicate to/from api thread
    // let (websocket_writer, websocket_reader) = mpsc::channel(100); // mpsc channel for websocket and validator
    // // let (validator_writer, validator_reader) = mpsc::channel(1); // channel to execute order

    // let api_task = runtime.spawn(async move {
    //     // main api thread
    // })

    // let websocket_task = runtime.spawn(async move {
    //     kucoin_websocket(kucoin_keys, websocket_writer).await //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
    // });

    // let validator_task = runtime.spawn(async move {
    //     find_triangular_arbitrage(
    //         kucoin_keys,
    //         &pair_combinations,
    //         // coin_fees,
    //         websocket_reader,
    //         validator_writer,
    //     )
    //     .await;
    // });

    // // let ordering_task =
    //     // runtime.spawn(async move { execute_trades(kucoin_keys, validator_reader).await });

    // // await tasks
    // websocket_task.await.unwrap();
    // validator_task.await.unwrap();
    // ordering_task.await.unwrap();
}

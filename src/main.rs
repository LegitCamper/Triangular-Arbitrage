// Libraries
use std::{collections::HashMap, sync::Arc};
use tokio::{runtime::Builder, sync::mpsc}; //, task};

// // my Libraries
// use kucoin_arbitrage::{
//     create_valid_pairs_catalog, execute_trades, find_triangular_arbitrage,
//     kucoin_interface::{KucoinInterface, get_creds()}, kucoin_websocket::kucoin_websocket,
// };

use kucoin_arbitrage::kucoin_interface::get_creds;

use kucoin_rs::kucoin::client::{Credentials, Kucoin, KucoinEnv};

#[tokio::main]
async fn main() {
    let keys = get_creds();
    let api = Kucoin::new(
        KucoinEnv::Live,
        Some(Credentials::new(
            keys.api_key.as_str(),
            keys.api_passphrase.as_str(),
            keys.api_secret.as_str(),
        )),
    )
    .expect("Failed to signin");

    // api.post(
    //     "https://api.kucoin.com/api/v1/bullet-private".to_string(),
    //     None,
    //     None,
    // )
    // .await;

    println!("{:?}", api.get_symbol_list(None).await.unwrap());

    // // Retreive temporary websocket token
    // let Some(websocket_info) = kucoin_interface.get_websocket_info().await else { panic! ("Unable to Retrive Token data from Kucoin") };

    // // Get all coin info
    // let Some(pair_info) = kucoin_interface.get_pairs().await else { panic!("Unable to Retrive Coin data from Kucoin") };

    // // Gets valid pair combinations
    // let pair_combinations = create_valid_pairs_catalog(pair_info).await;
    // println!("Generated Valid Coin Pairs successfully");

    // // build runtime - ensure tasks are being allocated their own thread
    // let runtime = Builder::new_multi_thread()
    //     // .worker_threads(4)
    //     .enable_all()
    //     .thread_name("arbitrage-calculator")
    //     .build()
    //     .unwrap();

    // let (websocket_writer, websocket_reader) = mpsc::channel(100); // mpsc channel for websocket and validator
    // let (validator_writer, validator_reader) = mpsc::channel(1); // channel to execute order

    // let websocket_task = runtime.spawn(async move {
    //     kucoin_websocket(websocket_info, websocket_writer).await
    //     //  websocket_token.unwrap(), // downloads websocket data and passes it through channel to validator
    // });

    // let validator_task = runtime.spawn(async move {
    //     find_triangular_arbitrage(
    //         &pair_combinations,
    //         // coin_fees,
    //         websocket_reader,
    //         validator_writer,
    //     )
    //     .await;
    // });

    // let ordering_task =
    //     runtime.spawn(async move { execute_trades(kucoin_interface, validator_reader).await }); //execute_trades(kucoin_interface,

    // // await tasks
    // websocket_task.await.unwrap();
    // validator_task.await.unwrap();
    // ordering_task.await.unwrap();
}

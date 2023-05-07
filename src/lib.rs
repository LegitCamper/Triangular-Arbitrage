pub mod kucoin_interface;
pub mod kucoin_websocket;

use crate::{
    kucoin_interface::{
        KucoinInterface, KucoinRequestOrderPost, KucoinRequestType, KucoinResponseL1,
    },
    kucoin_websocket::{KucoinWebsocketResponseL0, KucoinWebsocketResponseL1},
};

// use futures::channel::mpsc::Receiver;
use rand::prelude::*;
// use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;

// Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI" // TODO: this is static rn so dont add to the list
const STARTING_AMOUNT: f64 = 50.0; // Staring amount in USD
const MINIMUN_PROFIT: f64 = 0.1; // in USD

/////////////////////////////////////////////////////////  create_valid_pairs_catalog  /////////////////////////////////////////////////////////

pub async fn create_valid_pairs_catalog(
    coin_pairs: KucoinResponseL1,
) -> Vec<([String; 3], [String; 6])> {
    let mut output_list: Vec<([String; 3], [String; 6])> = Vec::new();

    for pair1 in coin_pairs.ticker.iter() {
        if !pair1.symbol.contains(STABLE_COINS[0]) {
            // TODO: make dynamic incase I deal with more stable coins
            continue;
        };
        let pair1_split: [&str; 2] = pair1
            .symbol
            .split('-')
            .collect::<Vec<&str>>()
            .try_into()
            .unwrap();
        for pair2 in coin_pairs.ticker.iter() {
            if pair2.symbol == pair1.symbol || pair2.symbol.contains(STABLE_COINS[0]) {
                continue;
            };
            let pair2_split: [&str; 2] = pair2
                .symbol
                .split('-')
                .collect::<Vec<&str>>()
                .try_into()
                .unwrap();
            if pair2_split[0] != pair1_split[0]
                && pair2_split[0] != pair1_split[1]
                && pair2_split[1] != pair1_split[0]
                && pair2_split[1] != pair1_split[1]
            {
                continue;
            };
            for pair3 in coin_pairs.ticker.iter() {
                if pair3.symbol == pair2.symbol
                    || pair3.symbol == pair1.symbol
                    || !pair3.symbol.contains(STABLE_COINS[0])
                {
                    continue;
                }
                let pair3_split: [&str; 2] = pair3
                    .symbol
                    .split('-')
                    .collect::<Vec<&str>>()
                    .try_into()
                    .unwrap();
                if pair3_split[0] != pair2_split[0]
                    && pair3_split[0] != pair2_split[1]
                    && pair3_split[1] != pair2_split[0]
                    && pair3_split[1] != pair2_split[1]
                {
                    continue;
                }

                let valid_pair = (
                    [
                        pair1.symbol.clone(),
                        pair2.symbol.clone(),
                        pair3.symbol.clone(),
                    ],
                    [
                        pair1_split[0].to_string(),
                        pair1_split[1].to_string(),
                        pair2_split[0].to_string(),
                        pair2_split[1].to_string(),
                        pair3_split[0].to_string(),
                        pair3_split[1].to_string(),
                    ],
                );

                // adding check to ensure there are only two of every symbol - Last check
                let mut equal_symbols = true;
                let mut pair_count = HashMap::new();
                for pair in valid_pair.1.iter() {
                    let count = pair_count.entry(pair).or_insert(0);
                    *count += 1;
                }
                for value in pair_count.values() {
                    if value != &2 {
                        equal_symbols = false;
                    }
                }

                if equal_symbols {
                    output_list.push(valid_pair);
                }
            }
        }
    }
    output_list
}

///////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
enum ArbOrd {
    Buy(String, String), // pair1, pair2
    Sell(String, String),
}

// TODO: should calulate this during catalog build in the future to prevent waisted IO
fn find_order_order(coin_pair: Vec<String>) -> Vec<ArbOrd> {
    let mut order: Vec<ArbOrd> = vec![];

    // get first order
    if coin_pair[0] == coin_pair[2] || coin_pair[0] == coin_pair[3] {
        order.push(ArbOrd::Buy(
            coin_pair[0].to_owned(),
            coin_pair[1].to_owned(),
        ));
    } else if coin_pair[1] == coin_pair[2] || coin_pair[1] == coin_pair[3] {
        order.push(ArbOrd::Sell(
            coin_pair[0].to_owned(),
            coin_pair[1].to_owned(),
        ));
    }
    // get second order
    if coin_pair[2] == coin_pair[4] || coin_pair[2] == coin_pair[5] {
        order.push(ArbOrd::Buy(
            coin_pair[2].to_owned(),
            coin_pair[3].to_owned(),
        ));
    } else if coin_pair[3] == coin_pair[4] || coin_pair[3] == coin_pair[5] {
        order.push(ArbOrd::Sell(
            coin_pair[2].to_owned(),
            coin_pair[3].to_owned(),
        ));
    }
    // get third order
    if coin_pair[4] == coin_pair[0] || coin_pair[4] == coin_pair[1] {
        order.push(ArbOrd::Buy(
            coin_pair[4].to_owned(),
            coin_pair[5].to_owned(),
        ));
    } else if coin_pair[5] == coin_pair[0] || coin_pair[5] == coin_pair[1] {
        order.push(ArbOrd::Sell(
            coin_pair[4].to_owned(),
            coin_pair[5].to_owned(),
        ));
    }
    order
}

// TODO: This assumes they are selling more than I am buying
fn calculate_profitablity(
    //This also returns price and size
    pair_strings: [String; 3],
    order: &[ArbOrd],
    coin_storage: &HashMap<String, KucoinWebsocketResponseL1>,
) -> f64 {
    // TODO: make stable coins dynamic incase as I add more
    // transaction 1
    let mut coin_amount: f64;
    coin_amount = match &order[0] {
        ArbOrd::Buy(_, _) => STARTING_AMOUNT / coin_storage[&pair_strings[0]].bestAsk,
        ArbOrd::Sell(_, _) => STARTING_AMOUNT * coin_storage[&pair_strings[0]].bestBid,
    };
    // Transaction 2
    coin_amount = match &order[1] {
        ArbOrd::Buy(_, _) => coin_amount / coin_storage[&pair_strings[1]].bestAsk,
        ArbOrd::Sell(_, _) => coin_amount * coin_storage[&pair_strings[1]].bestBid,
    };
    // Transaction 3
    coin_amount = match &order[2] {
        ArbOrd::Buy(_, _) => coin_amount / coin_storage[&pair_strings[2]].bestAsk,
        ArbOrd::Sell(_, _) => coin_amount * coin_storage[&pair_strings[2]].bestBid,
    };
    coin_amount
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct OrderStruct {
    side: ArbOrd,
    price: f64,
    size: f64,
}

pub async fn find_triangular_arbitrage(
    valid_coin_pairs: &Vec<([String; 3], [String; 6])>,
    // coin_fees: CoinFees,
    mut websocket_reader: mpsc::Receiver<KucoinWebsocketResponseL0>,
    validator_writer: mpsc::Sender<Vec<OrderStruct>>,
) {
    // skipping caluculation for fees - assuming KCS fees are enabled
    // println!("skipping caluculation for fees - assuming KCS fees are enabled");

    // Define methode for storing current best prices
    let mut coin_storage: HashMap<String, KucoinWebsocketResponseL1> = HashMap::new();
    while let Some(msg) = websocket_reader.recv().await {
        coin_storage.insert(msg.subject, msg.data);
        // main validator loop
        for pairs_tuple in valid_coin_pairs {
            let (pairs, pairs_split) = pairs_tuple;

            // loop through data and chekc for arbs
            if coin_storage.get(&pairs[0]).is_some()
                && coin_storage.get(&pairs[1]).is_some()
                && coin_storage.get(&pairs[2]).is_some()
            {
                // anything in here has been garenteed to be in coin_storage
                // TODO: Consider checking timestamp here. future iterations
                let orders_order = find_order_order(pairs_split.to_vec());
                let profit = calculate_profitablity(pairs.clone(), &orders_order, &coin_storage)
                    - STARTING_AMOUNT;
                if profit >= MINIMUN_PROFIT {
                    let mut orders = vec![];
                    for side in orders_order {
                        // TODO: Need to implement Rounding with math.round(#, #'s place)
                        match side {
                            ArbOrd::Buy(ref p1, ref p2) => orders.push(OrderStruct {
                                side: side.clone(),
                                price: coin_storage
                                    .get(&format!("{}-{}", &p1, &p2))
                                    .unwrap()
                                    .bestAsk,
                                size: coin_storage.get(&format!("{}-{}", p1, p2)).unwrap().size,
                            }),
                            ArbOrd::Sell(ref p1, ref p2) => orders.push(OrderStruct {
                                side: side.clone(),
                                price: coin_storage.get(&format!("{}-{}", p1, p2)).unwrap().bestBid,
                                size: coin_storage.get(&format!("{}-{}", p1, p2)).unwrap().size,
                            }),
                        }
                    }
                    println!("{:?}", pairs_tuple);
                    validator_writer.send(orders).await.unwrap();
                }
            }
        }
    }
}

// /////////////////////////////////////////////////////////  execute_trades  /////////////////////////////////////////////////////////

// #[derive(Debug, Serialize)]
// struct OrderResponse {
//     order_id: f64,
// }

pub async fn execute_trades(
    kucoin_interface: Arc<KucoinInterface>,
    mut validator_reader: mpsc::Receiver<Vec<OrderStruct>>,
) {
    let mut rng = ::rand::rngs::StdRng::from_seed(rand::rngs::OsRng.gen());

    while let Some(msg) = validator_reader.recv().await {
        // Iterates through each order in msg
        for order in msg {
            let json_order = match order.side {
                ArbOrd::Buy(pair1, pair2) => KucoinRequestOrderPost {
                    timeInForce: "FOK".to_string(),
                    size: order.size,
                    price: order.price,
                    symbol: format!("{}-{}", pair1, pair2),
                    side: "buy".to_string(),
                    clientOid: rng.gen(),
                },
                ArbOrd::Sell(pair1, pair2) => KucoinRequestOrderPost {
                    timeInForce: "FOK".to_string(),
                    size: order.size,
                    price: order.price,
                    symbol: format!("{}-{}", pair1, pair2),
                    side: "sell".to_string(),
                    clientOid: rng.gen(),
                },
            };
            println!("{:?}", json_order);
            let kucoin_response = kucoin_interface.request(
                "api/v1/orders",
                serde_json::to_string(&json_order).expect("Failed to Serialize"),
                KucoinRequestType::OrderPost,
            );
            println!("Order Response: {:?}", kucoin_response.await); // TODO: Remove this
        }
    }
}

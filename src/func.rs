use binance::rest_model::OrderBook;
use itertools::Itertools;
use log::{info, trace, warn};
use rayon::prelude::*;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, sync::Arc};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
    time::{interval, Duration},
};

// // Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"
const STARTING_AMOUNT: f64 = 50.0; // Staring amount in USD
const MINIMUN_PROFIT: f64 = 0.001; // in USD

fn is_stable(symbol: &(String, String)) -> bool {
    for stable_symbol in STABLE_COINS {
        if symbol.0 == stable_symbol || symbol.1 == stable_symbol {
            return true;
        }
    }
    false
}

pub async fn create_valid_pairs_catalog(symbols: Vec<(String, String)>) -> Vec<[String; 6]> {
    trace!("Create Valid Pairs Catalog");

    symbols
        .iter()
        .combinations_with_replacement(3)
        .par_bridge()
        .map(|p| (p[0], p[1], p[2]))
        .filter(|(p1, p2, p3)| {
            [&p1.0, &p1.1, &p2.0, &p2.1, &p3.0, &p3.1]
                .iter()
                .unique()
                .count()
                == 3
        })
        .filter(|(p1, _, p3)| is_stable(p1) && is_stable(p3))
        .filter(|(p1, p2, _)| {
            p1.0 == p2.0 && p1.1 != p2.1
                || p1.1 == p2.1 && p1.0 != p2.0
                || p1.1 == p2.0 && p1.0 != p2.1
                || p1.0 == p2.1 && p1.1 != p2.0
        })
        .filter(|(_, p2, p3)| {
            p3.0 == p2.0 && p3.1 != p2.1
                || p3.1 == p2.1 && p3.0 != p2.0
                || p3.1 == p2.0 && p3.0 != p2.1
                || p3.0 == p2.1 && p3.1 != p2.0
        })
        .map(|(p1, p2, p3)| (p1.clone(), p2.clone(), p3.clone()))
        .map(|(p1, p2, p3)| [p1.0, p1.1, p2.0, p2.1, p3.0, p3.1])
        .collect()
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum ArbOrd {
    Buy(String, String), // pair1, pair2
    Sell(String, String),
}

// TODO: should calulate this during catalog build in the future to prevent wasted IO
fn find_order_order(coin_pair: &[String; 6]) -> Vec<ArbOrd> {
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
    order: &[ArbOrd],
    coin_storage: [OrderBook; 3],
) -> f64 {
    let mut coin_amount: f64;
    // transaction 1
    coin_amount = match &order[0] {
        ArbOrd::Buy(_, _) => STARTING_AMOUNT / coin_storage[0].asks[0].price,
        ArbOrd::Sell(_, _) => STARTING_AMOUNT * coin_storage[0].bids[0].price,
    };
    // Transaction 2
    coin_amount = match &order[1] {
        ArbOrd::Buy(_, _) => coin_amount / coin_storage[1].asks[0].price,
        ArbOrd::Sell(_, _) => coin_amount * coin_storage[1].bids[0].price,
    };
    // Transaction 3
    coin_amount = match &order[2] {
        ArbOrd::Buy(_, _) => coin_amount / coin_storage[2].asks[0].price,
        ArbOrd::Sell(_, _) => coin_amount * coin_storage[2].bids[0].price,
    };
    info!("Profit {:?}", coin_amount);
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
    valid_coin_pairs: Vec<[String; 6]>,
    validator_writer: mpsc::UnboundedSender<OrderStruct>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
) -> JoinHandle<()> {
    trace!("Find Triangular Arbitrage");

    task::spawn(async move {
        let mut interval = interval(Duration::from_millis(50));
        loop {
            interval.tick().await;

            'inner: for split_pairs in valid_coin_pairs.iter() {
                let pairs = [
                    format!("{}-{}", split_pairs[0], split_pairs[1]),
                    format!("{}-{}", split_pairs[2], split_pairs[3]),
                    format!("{}-{}", split_pairs[4], split_pairs[5]),
                ];

                // loop through data and check for arbs
                if let Some((pair0, pair1, pair2)) = clone_orderbook(&pairs, &orderbook).await {
                    if pair0.bids.is_empty()
                        || pair0.asks.is_empty()
                        || pair1.bids.is_empty()
                        || pair1.asks.is_empty()
                        || pair2.bids.is_empty()
                        || pair2.asks.is_empty()
                    {
                        warn!("A pair was empty");
                        continue 'inner;
                    };
                    let orders = find_order_order(split_pairs);
                    let profit = calculate_profitablity(
                        &orders,
                        [pair0.clone(), pair1.clone(), pair2.clone()],
                    ) - STARTING_AMOUNT;
                    if profit >= MINIMUN_PROFIT {
                        info!("Profit: {profit}, pairs: {:?}", split_pairs);
                        create_order((pair0, pair1, pair2), orders, &validator_writer).await;
                    }
                } else {
                    // warn!("None in orderbook");
                }
            }
        }
    })
}

async fn clone_orderbook(
    pairs: &[String; 3],
    orderbook: &Arc<Mutex<HashMap<String, OrderBook>>>,
) -> Option<(OrderBook, OrderBook, OrderBook)> {
    let orderbook = orderbook.lock().await;
    Some((
        orderbook.get(&pairs[0])?.clone(),
        orderbook.get(&pairs[1])?.clone(),
        orderbook.get(&pairs[2])?.clone(),
    ))
}

async fn create_order(
    local_orderbook: (OrderBook, OrderBook, OrderBook),
    orders_order: Vec<ArbOrd>,
    _validator_writer: &mpsc::UnboundedSender<OrderStruct>,
) {
    let mut orders = vec![];

    for (pair, side) in vec![local_orderbook.0, local_orderbook.1, local_orderbook.2]
        .iter()
        .zip(orders_order.iter())
    {
        match side {
            ArbOrd::Buy(_, _) => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    side: side.clone(),
                    price: pair.asks[0].price,
                    size: pair.asks[0].qty,
                })
            }
            ArbOrd::Sell(_, _) => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    side: side.clone(),
                    price: pair.bids[0].price,
                    size: pair.bids[0].qty,
                })
            }
        }
    }
    info!("Orders: {:?}", orders);
    // validator_writer.send(orders).await.unwrap();
}

#[derive(Debug, Deserialize)]
pub struct Key {
    pub key: String,
    pub secret: String,
}

pub fn read_key() -> Key {
    let mut file = File::open("key.json").expect("Could not read the json file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Could not deserialize the file, error");
    serde_json::from_str(&contents.as_str()).expect("Could not deserialize")
}

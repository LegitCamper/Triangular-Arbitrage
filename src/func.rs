use binance::rest_model::OrderBook;
use itertools::Itertools;
use log::{info, trace};
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

    let mut output_list: Vec<[String; 6]> = Vec::new();

    for pair1 in symbols.iter() {
        if !is_stable(pair1) {
            continue;
        };
        for pair2 in symbols.iter() {
            if pair2 == pair1 || is_stable(pair2) {
                continue;
            };
            if pair2.0 != pair1.0 && pair2.0 != pair1.1 && pair2.1 != pair1.0 && pair2.1 != pair1.1
            {
                continue;
            };
            for pair3 in symbols.iter() {
                if pair3 == pair2 || pair3 == pair1 || !is_stable(pair3) {
                    continue;
                }
                if pair3.0 != pair2.0
                    && pair3.0 != pair2.1
                    && pair3.1 != pair2.0
                    && pair3.1 != pair2.1
                {
                    continue;
                }

                let valid_pair = [
                    pair1.0.to_string(),
                    pair1.1.to_string(),
                    pair2.0.to_string(),
                    pair2.1.to_string(),
                    pair3.0.to_string(),
                    pair3.1.to_string(),
                ];

                // adding check to ensure there are only two of every symbol - Last check
                if [&pair1.0, &pair1.1, &pair2.0, &pair2.1, &pair3.0, &pair3.1]
                    .iter()
                    .unique()
                    .count()
                    == 3
                {
                    output_list.push(valid_pair);
                }
            }
        }
    }
    info!("Generated Valid Coin Pairs successfully");
    output_list
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ArbOrd {
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

// TODO: this assumes all stable coins are pegged at us dollar
fn calculate_profitablity(order: &[ArbOrd], coin_storage: [OrderBook; 3]) -> f64 {
    let mut coin_amount = 0.0;
    for pair in coin_storage.into_iter() {
        coin_amount = match &order[0] {
            ArbOrd::Buy(_, _) => {
                let amnt = STARTING_AMOUNT / pair.asks[0].price;
                if amnt > pair.asks[0].qty {
                    return 0.0;
                } else {
                    amnt
                }
            }
            ArbOrd::Sell(_, _) => {
                let amnt = STARTING_AMOUNT * pair.bids[0].price;
                if amnt > pair.bids[0].qty {
                    return 0.0;
                } else {
                    amnt
                }
            }
        }
    }
    coin_amount
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrderStruct {
    pub symbol: String,
    pub side: ArbOrd,
    pub price: f64,
    pub size: f64,
}

pub async fn find_triangular_arbitrage(
    valid_coin_pairs: Vec<[String; 6]>,
    validator_writer: mpsc::UnboundedSender<Vec<OrderStruct>>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
) -> JoinHandle<()> {
    trace!("Find Triangular Arbitrage");

    task::spawn(async move {
        let mut interval = interval(Duration::from_millis(10)); // TODO: this may need to be decreased
        loop {
            interval.tick().await;

            'inner: for split_pairs in valid_coin_pairs.iter() {
                let pairs = [
                    format!("{}{}", split_pairs[0], split_pairs[1]),
                    format!("{}{}", split_pairs[2], split_pairs[3]),
                    format!("{}{}", split_pairs[4], split_pairs[5]),
                ];
                // info!("Got: {:?}", pairs);

                // loop through data and check for arbs
                if let Some((pair0, pair1, pair2)) = clone_orderbook(&pairs, &orderbook).await {
                    if pair0.bids.is_empty()
                        || pair0.asks.is_empty()
                        || pair1.bids.is_empty()
                        || pair1.asks.is_empty()
                        || pair2.bids.is_empty()
                        || pair2.asks.is_empty()
                    {
                        // warn!("A pair was empty");
                        continue 'inner;
                    };
                    let orders = find_order_order(split_pairs);
                    let profit = calculate_profitablity(
                        &orders,
                        [pair0.clone(), pair1.clone(), pair2.clone()],
                    ) - STARTING_AMOUNT;
                    if profit >= MINIMUN_PROFIT {
                        // info!("Profit: {profit}, pairs: {:?}", split_pairs);
                        let orders = create_order(&pairs, (pair0, pair1, pair2), orders).await;

                        // removing price that led to order
                        remove_bought(&orderbook, &pairs, &orders).await;
                        validator_writer.send(orders).unwrap();
                    }
                } else {
                    // warn!("None in orderbook for: {:?}", split_pairs);
                }
            }
        }
    })
}

async fn remove_bought(
    orderbook: &Arc<Mutex<HashMap<String, OrderBook>>>,
    pairs: &[String; 3],
    orders: &Vec<OrderStruct>,
) {
    let mut orderbook = orderbook.lock().await;
    for (n, pair) in pairs.iter().enumerate() {
        let pair = orderbook.get_mut(pair).unwrap();
        match orders[n].side {
            ArbOrd::Buy(_, _) => {
                pair.asks.remove(0);
                ()
            }
            ArbOrd::Sell(_, _) => {
                pair.bids.remove(0);
                ()
            }
        }
    }
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
    pairs: &[String; 3],
    local_orderbook: (OrderBook, OrderBook, OrderBook),
    orders_order: Vec<ArbOrd>,
) -> Vec<OrderStruct> {
    let mut orders = vec![];

    for ((pair_data, pair), side) in vec![local_orderbook.0, local_orderbook.1, local_orderbook.2]
        .iter()
        .zip(pairs.iter())
        .zip(orders_order.iter())
    {
        match side {
            ArbOrd::Buy(_, _) => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: pair.clone(),
                    side: side.clone(),
                    price: pair_data.asks[0].price,
                    size: pair_data.asks[0].qty,
                })
            }
            ArbOrd::Sell(_, _) => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: pair.clone(),
                    side: side.clone(),
                    price: pair_data.bids[0].price,
                    size: pair_data.bids[0].qty,
                })
            }
        }
    }
    // info!("Orders: {:?}", orders);
    orders
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

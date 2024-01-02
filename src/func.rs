use binance::rest_model::OrderBook;
use log::{info, trace, warn};

use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
    time::{interval, Duration},
};

// // Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"
const STARTING_AMOUNT: f64 = 50.0; // Staring amount in USD
const MINIMUN_PROFIT: f64 = 0.1; // in USD

fn is_stable(symbol: &(String, String)) -> bool {
    for stable_symbol in STABLE_COINS {
        if symbol.0 == stable_symbol || symbol.1 == stable_symbol {
            return true;
        }
    }
    false
}

pub async fn create_valid_pairs_catalog(
    symbols: Vec<(String, String)>,
) -> Vec<([String; 3], [String; 6])> {
    trace!("Create Valid Pairs Catalog");
    let mut output_list: Vec<([String; 3], [String; 6])> = Vec::new();

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

                let valid_pair = (
                    [
                        format!("{}{}", pair1.0, pair1.1),
                        format!("{}{}", pair2.0, pair2.1),
                        format!("{}{}", pair3.0, pair3.1),
                    ],
                    [
                        pair1.0.to_string(),
                        pair1.1.to_string(),
                        pair2.0.to_string(),
                        pair2.1.to_string(),
                        pair3.0.to_string(),
                        pair3.1.to_string(),
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
    info!("Generated Valid Coin Pairs successfully");
    output_list
}

#[derive(Debug, Clone)]
enum ArbOrd {
    Buy(String, String), // pair1, pair2
    Sell(String, String),
}

// TODO: should calulate this during catalog build in the future to prevent wasted IO
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
    order: &[ArbOrd],
    coin_storage: [OrderBook; 3],
) -> f64 {
    // transaction 1
    let mut coin_amount: f64;
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
    coin_amount
}

#[derive(Debug)]
pub struct OrderStruct {
    side: ArbOrd,
    price: f64,
    size: f64,
}

pub async fn find_triangular_arbitrage(
    valid_coin_pairs: Vec<([String; 3], [String; 6])>,
    _validator_writer: mpsc::UnboundedSender<OrderStruct>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
) -> JoinHandle<()> {
    trace!("Find Triangular Arbitrage");

    task::spawn(async move {
        let mut interval = interval(Duration::from_millis(50));
        loop {
            interval.tick().await;

            for pairs_tuple in valid_coin_pairs.iter() {
                let (pairs, pairs_split) = pairs_tuple;

                // loop through data and check for arbs
                let orderbook = orderbook.lock().await;
                if orderbook.get(&pairs[0]).is_some()
                    && orderbook.get(&pairs[1]).is_some()
                    && orderbook.get(&pairs[2]).is_some()
                {
                    if orderbook.get(&pairs[0]).unwrap().bids.is_empty()
                        || orderbook.get(&pairs[0]).unwrap().asks.is_empty()
                        || orderbook.get(&pairs[1]).unwrap().bids.is_empty()
                        || orderbook.get(&pairs[1]).unwrap().asks.is_empty()
                        || orderbook.get(&pairs[2]).unwrap().bids.is_empty()
                        || orderbook.get(&pairs[2]).unwrap().asks.is_empty()
                    {
                        continue;
                    };

                    let orders_order = find_order_order(pairs_split.to_vec());

                    let pair1 = orderbook.get(&pairs[0]).cloned().unwrap();
                    let pair2 = orderbook.get(&pairs[1]).cloned().unwrap();
                    let pair3 = orderbook.get(&pairs[2]).cloned().unwrap();

                    let profit = calculate_profitablity(&orders_order, [pair1, pair2, pair3])
                        - STARTING_AMOUNT;
                    if profit >= MINIMUN_PROFIT {
                        info!("Profit: {profit}");
                        let mut orders = vec![];
                        for side in orders_order {
                            // TODO: Need to implement Rounding with math.round(#, #'s place)
                            match side {
                                ArbOrd::Buy(ref p1, ref p2) => {
                                    // warn!("Error, {:?}, ", orderbook);
                                    orders.push(OrderStruct {
                                        side: side.clone(),
                                        price: orderbook
                                            .get(&format!("{}{}", &p1, &p2))
                                            .unwrap()
                                            .asks[0]
                                            .price,
                                        size: orderbook.get(&format!("{}{}", p1, p2)).unwrap().asks
                                            [0]
                                        .qty,
                                    })
                                }
                                ArbOrd::Sell(ref p1, ref p2) => orders.push(OrderStruct {
                                    side: side.clone(),
                                    price: orderbook.get(&format!("{}-{}", p1, p2)).unwrap().bids
                                        [0]
                                    .price,
                                    size: orderbook.get(&format!("{}-{}", p1, p2)).unwrap().bids[0]
                                        .qty,
                                }),
                            }
                        }
                        info!("{:?}", pairs_tuple);
                        // validator_writer.send(orders).await.unwrap();
                    }
                }
            }
        }
    })
}

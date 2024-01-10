use binance::rest_model::{ExchangeInformation, Filters, OrderBook};
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
const MINIMUM_FEE_DECIMAL: f64 = 0.0057;

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
#[derive(Debug, Clone, Deserialize)]
pub enum ArbOrd {
    Buy,
    Sell,
}

// TODO: should calulate this during catalog build in the future to prevent wasted IO
fn find_order_order(coin_pair: &[String; 6]) -> [ArbOrd; 3] {
    [
        // get first order
        if coin_pair[0] == coin_pair[2] || coin_pair[0] == coin_pair[3] {
            ArbOrd::Buy
        } else if coin_pair[1] == coin_pair[2] || coin_pair[1] == coin_pair[3] {
            ArbOrd::Sell
        } else {
            unreachable!()
        },
        // get second order
        if coin_pair[2] == coin_pair[4] || coin_pair[2] == coin_pair[5] {
            ArbOrd::Buy
        } else if coin_pair[3] == coin_pair[4] || coin_pair[3] == coin_pair[5] {
            ArbOrd::Sell
        } else {
            unreachable!()
        },
        // get third order
        if coin_pair[4] == coin_pair[0] || coin_pair[4] == coin_pair[1] {
            ArbOrd::Buy
        } else if coin_pair[5] == coin_pair[0] || coin_pair[5] == coin_pair[1] {
            ArbOrd::Sell
        } else {
            unreachable!()
        },
    ]
}

// TODO: this assumes all stable coins are pegged at us dollar
// fn calculate_profitablity(
//     exchange_info: &ExchangeInformation,
//     pairs: &[String; 3],
//     order: &[ArbOrd],
//     coin_storage: [OrderBook; 3],
// ) -> (f64, f64, f64, f64) {
//     let mut coin_amount = 0.0;
//     let mut qty = vec![];
//     for (pair, symbol) in coin_storage.into_iter().zip(pairs.iter()) {
//         coin_amount = match &order[0] {
//             ArbOrd::Buy => {
//                 let size = step_size(exchange_info, symbol, pair.asks[0].qty, pair.asks[0].price);
//                 let size = STARTING_AMOUNT / pair.asks[0].price;
//                 qty.push(size);
//                 if size > pair.asks[0].qty {
//                     return (0.0, 0.0, 0.0, 0.0);
//                 } else {
//                     size
//                 }
//             }
//             ArbOrd::Sell => {
//                 let size = STARTING_AMOUNT * pair.bids[0].price;
//                 qty.push(size);
//                 if size > pair.bids[0].qty {
//                     return (0.0, 0.0, 0.0, 0.0);
//                 } else {
//                     size
//                 }
//             }
//         }
//     }
//     coin_amount -= coin_amount * (MINIMUM_FEE_DECIMAL * 3.0);
//     (coin_amount, qty[0], qty[1], qty[2])
// }

fn calculate_profitablity(
    exchange_info: &ExchangeInformation,
    pairs: &[String; 3],
    order: &[ArbOrd],
    coin_storage: [OrderBook; 3],
) -> (f64, f64, f64, f64) {
    let mut qty = vec![];
    // transaction 1
    let mut coin_amount: f64;
    coin_amount = match &order[0] {
        ArbOrd::Buy => {
            let size = step_size(
                exchange_info,
                &pairs[0],
                STARTING_AMOUNT,
                coin_storage[0].asks[0].price,
            );
            qty.push(size);
            size
        }
        ArbOrd::Sell => {
            let size = step_size(
                exchange_info,
                &pairs[0],
                STARTING_AMOUNT,
                coin_storage[0].bids[0].price,
            );
            qty.push(size);
            size
        }
    };
    // Transaction 2
    coin_amount = match &order[1] {
        ArbOrd::Buy => {
            let size = step_size(
                exchange_info,
                &pairs[1],
                coin_amount,
                coin_storage[1].asks[0].price,
            );
            qty.push(size);
            size
        }
        ArbOrd::Sell => {
            let size = step_size(
                exchange_info,
                &pairs[1],
                coin_amount,
                coin_storage[1].bids[0].price,
            );
            qty.push(size);
            size
        }
    };
    // Transaction 3
    coin_amount = match &order[2] {
        ArbOrd::Buy => {
            let size = step_size(
                exchange_info,
                &pairs[2],
                coin_amount,
                coin_storage[2].asks[0].price,
            );
            qty.push(size);
            size
        }
        ArbOrd::Sell => {
            let size = step_size(
                exchange_info,
                &pairs[2],
                coin_amount,
                coin_storage[2].bids[0].price,
            );
            qty.push(size);
            size
        }
    };
    (coin_amount, qty[0], qty[1], qty[2])
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
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
    exchange_info: ExchangeInformation,
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
                    let (mut profit, qty1, qty2, qty3) = calculate_profitablity(
                        &exchange_info,
                        &pairs,
                        &orders,
                        [pair0.clone(), pair1.clone(), pair2.clone()],
                    );
                    profit -= STARTING_AMOUNT;
                    if profit >= MINIMUN_PROFIT {
                        // info!("Profit: {profit}, pairs: {:?}", split_pairs);
                        let orders =
                            create_order(&pairs, (pair0, pair1, pair2), orders, [qty1, qty2, qty3])
                                .await;
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
            ArbOrd::Buy => {
                pair.asks.remove(0);
                ()
            }
            ArbOrd::Sell => {
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
    orders_order: [ArbOrd; 3],
    qtys: [f64; 3],
) -> Vec<OrderStruct> {
    let mut orders = vec![];

    for (c, (((pair_data, pair), side), qty)) in
        vec![local_orderbook.0, local_orderbook.1, local_orderbook.2]
            .iter()
            .zip(pairs.iter())
            .zip(orders_order.iter())
            .zip(qtys.iter())
            .enumerate()
    {
        // calculate fees
        let size = if c > 0 {
            *qty - *qty * MINIMUM_FEE_DECIMAL
        } else {
            *qty
        };
        match side {
            ArbOrd::Buy => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: pair.clone(),
                    side: side.clone(),
                    price: pair_data.asks[0].price,
                    size,
                })
            }
            ArbOrd::Sell => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: pair.clone(),
                    side: side.clone(),
                    price: pair_data.bids[0].price,
                    size,
                })
            }
        }
    }
    orders
}

fn step_size(exchange_info: &ExchangeInformation, symbol: &String, size: f64, price: f64) -> f64 {
    for exchange_symbol_data in exchange_info.symbols.iter() {
        if *symbol == exchange_symbol_data.symbol {
            for filter in &exchange_symbol_data.filters {
                if let Filters::LotSize {
                    min_qty: _,
                    max_qty: _,
                    step_size,
                } = filter
                {
                    let amount = size / price;
                    if amount % step_size != 0.0 {
                        let amount_str = format!("{}", amount);
                        let step_size_str = format!("{}", step_size);
                        let step_size_arr: Vec<&str> = step_size_str.split(".").collect();
                        let step_size_len = if step_size_arr[0] == "0" {
                            step_size_arr[1].len()
                        } else {
                            0
                        };

                        let mut amount_arr = amount_str.split(".");
                        println!(
                            "{}",
                            format!(
                                "{}.{}",
                                amount_arr.next().unwrap(),
                                amount_arr.next().unwrap().slic
                            )
                        );
                        let mut amount_arr = amount_str.split(".");
                        let amount_arr =
                            vec![amount_arr.next().unwrap(), amount_arr.next().unwrap()];
                        return price
                            * format!(
                                "{}.{}",
                                amount_arr[0],
                                amount_arr[1][0..step_size_len].to_string()
                            )
                            .parse::<f64>()
                            .unwrap();
                    } else {
                        return size;
                    }
                }
            }
        }
    }
    0.0
}

#[derive(Debug, Deserialize, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::*;

    #[tokio::test]
    async fn test_step_size() {
        let interface = BinanceInterface::new();
        let exchange_info = interface.get_exchange_info().await.unwrap();

        assert_eq!(
            50.0,
            step_size(
                &exchange_info,
                &String::from("USDCUSDT"),
                0.9996,
                50.020008003201276
            )
        );
    }
}

use binance::rest_model::{ExchangeInformation, Filters, OrderBook};
use itertools::Itertools;
use log::{trace, warn};
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

#[derive(Debug, Clone)]
pub struct Symbol {
    pub pair1: String,
    pub pair2: String,
}
impl Symbol {
    pub fn new(pair1: &str, pair2: &str) -> Self {
        Symbol {
            pair1: pair1.into(),
            pair2: pair2.into(),
        }
    }
    pub fn pair(&self) -> String {
        format!("{}{}", self.pair1, self.pair2)
    }
    #[allow(dead_code)]
    pub fn symbols<'a>(&'a self) -> (&'a String, &'a String) {
        (&self.pair1, &self.pair2)
    }
}

fn is_stable(symbol: &Symbol) -> bool {
    for stable_symbol in STABLE_COINS {
        if symbol.pair1 == stable_symbol || symbol.pair2 == stable_symbol {
            return true;
        }
    }
    false
}

pub async fn create_valid_pairs_catalog(symbols: &[Symbol]) -> Vec<[Symbol; 3]> {
    trace!("Create Valid Pairs Catalog");

    let mut output_list: Vec<[Symbol; 3]> = Vec::new();

    for pair1 in symbols.iter() {
        if !is_stable(pair1) {
            continue;
        };
        for pair2 in symbols.iter() {
            if pair2.pair() == pair1.pair() || is_stable(pair2) {
                continue;
            };
            if pair2.pair1 != pair1.pair1
                && pair2.pair1 != pair1.pair2
                && pair2.pair2 != pair1.pair1
                && pair2.pair2 != pair1.pair2
            {
                continue;
            };
            for pair3 in symbols.iter() {
                if pair3.pair() == pair2.pair() || pair3.pair() == pair1.pair() || !is_stable(pair3)
                {
                    continue;
                }
                if pair3.pair1 != pair2.pair1
                    && pair3.pair1 != pair2.pair2
                    && pair3.pair2 != pair2.pair1
                    && pair3.pair2 != pair2.pair2
                {
                    continue;
                }

                // adding check to ensure there are only two of every symbol - Last check
                if [
                    &pair1.pair1,
                    &pair1.pair2,
                    &pair2.pair1,
                    &pair2.pair2,
                    &pair3.pair1,
                    &pair3.pair2,
                ]
                .iter()
                .unique()
                .count()
                    == 3
                {
                    output_list.push([pair1.clone(), pair2.clone(), pair3.clone()]);
                }
            }
        }
    }
    output_list
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub enum ArbOrd {
    Buy,
    Sell,
}

// TODO: should calulate this during catalog build in the future to prevent wasted IO
fn find_order_order(pairs: &[Symbol; 3]) -> [(Symbol, ArbOrd); 3] {
    [
        // get first order
        if pairs[0].pair() == pairs[2].pair() || pairs[0].pair() == pairs[3].pair() {
            (pairs[0].clone(), ArbOrd::Buy)
        } else if pairs[1].pair() == pairs[2].pair() || pairs[1].pair() == pairs[3].pair() {
            (pairs[0].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
        // get second order
        if pairs[2].pair() == pairs[4].pair() || pairs[2].pair() == pairs[5].pair() {
            (pairs[1].clone(), ArbOrd::Buy)
        } else if pairs[3].pair() == pairs[4].pair() || pairs[3].pair() == pairs[5].pair() {
            (pairs[1].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
        // get third order
        if pairs[4].pair() == pairs[0].pair() || pairs[4].pair() == pairs[1].pair() {
            (pairs[2].clone(), ArbOrd::Buy)
        } else if pairs[5].pair() == pairs[0].pair() || pairs[5].pair() == pairs[1].pair() {
            (pairs[2].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
    ]
}

// TODO: this assumes all stable coins are pegged at us dollar
fn calculate_profitablity(
    exchange_info: &ExchangeInformation,
    order: &[(Symbol, ArbOrd); 3],
    coin_storage: [OrderBook; 3],
) -> (f64, Vec<f64>, Vec<f64>) {
    let mut coin_qty = STARTING_AMOUNT;
    let mut qtys = vec![];
    let mut prices = vec![];
    for (coin_data, (symbol, order)) in coin_storage.iter().zip(order.iter()) {
        coin_qty = match &order {
            ArbOrd::Buy => {
                let size = step_size(
                    exchange_info,
                    symbol,
                    coin_data.asks[0].qty,
                    coin_data.asks[0].price,
                );
                qtys.push(size);
                prices.push(coin_data.asks[0].price);
                if size > coin_data.asks[0].qty {
                    return (0.0, vec![], vec![]);
                } else {
                    size
                }
            }
            ArbOrd::Sell => {
                let size = step_size(
                    exchange_info,
                    symbol,
                    coin_data.bids[0].qty,
                    coin_data.bids[0].price,
                );
                qtys.push(size);
                prices.push(coin_data.bids[0].price);
                if size > coin_data.bids[0].qty {
                    return (0.0, vec![], vec![]);
                } else {
                    size
                }
            }
        };
        coin_qty -= coin_qty * MINIMUM_FEE_DECIMAL;
    }
    (coin_qty, qtys, prices)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct OrderStruct {
    pub symbol: String,
    pub side: ArbOrd,
    pub price: f64,
    pub size: f64,
}

pub async fn find_tri_arb(
    valid_coin_pairs: Vec<[Symbol; 3]>,
    validator_writer: mpsc::UnboundedSender<Vec<OrderStruct>>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    exchange_info: ExchangeInformation,
) -> JoinHandle<()> {
    trace!("Find Triangular Arbitrage");

    task::spawn(async move {
        let valid_coin_pairs = valid_coin_pairs.as_slice();
        let mut interval = interval(Duration::from_millis(10)); // TODO: this may need to be decreased
        loop {
            interval.tick().await;

            'inner: for pairs in valid_coin_pairs.iter() {
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
                    let orders = find_order_order(pairs);
                    let (mut profit, prices, sizes) = calculate_profitablity(
                        &exchange_info,
                        &orders,
                        [pair0.clone(), pair1.clone(), pair2.clone()],
                    );
                    profit -= STARTING_AMOUNT;
                    if profit >= MINIMUN_PROFIT {
                        // info!("Profit: {profit}, pairs: {:?}", split_pairs);
                        let orders =
                            create_order(orders, sizes.as_slice(), prices.as_slice()).await;

                        // ensure orders adhere to binance's order filters
                        if let Ok(_) = check_filters(&orders, &exchange_info) {
                            // removing price that led to order
                            remove_bought(&orderbook, &pairs, &orders).await;
                            validator_writer.send(orders).unwrap();
                        }
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
    pairs: &[Symbol; 3],
    orders: &Vec<OrderStruct>,
) {
    let mut orderbook = orderbook.lock().await;
    for (count, pair) in pairs.iter().enumerate() {
        let pair = orderbook.get_mut(&pair.pair()).unwrap();
        match orders[count].side {
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
    pairs: &[Symbol; 3],
    orderbook: &Arc<Mutex<HashMap<String, OrderBook>>>,
) -> Option<(OrderBook, OrderBook, OrderBook)> {
    let orderbook = orderbook.lock().await;
    Some((
        orderbook.get(&pairs[0].pair())?.clone(),
        orderbook.get(&pairs[1].pair())?.clone(),
        orderbook.get(&pairs[2].pair())?.clone(),
    ))
}

async fn create_order(
    orders_order: [(Symbol, ArbOrd); 3],
    qtys: &[f64],
    prices: &[f64],
) -> Vec<OrderStruct> {
    let mut orders = vec![];

    for (count, (symbol, side)) in orders_order.iter().enumerate() {
        // calculate fees
        let size = if count > 0 {
            qtys[count] - qtys[count] * MINIMUM_FEE_DECIMAL
        } else {
            qtys[count]
        };
        match side {
            ArbOrd::Buy => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: symbol.pair(),
                    side: side.clone(),
                    price: prices[count],
                    size,
                })
            }
            ArbOrd::Sell => {
                // warn!("Error, {:?}, ", orderbook);
                orders.push(OrderStruct {
                    symbol: symbol.pair(),
                    side: side.clone(),
                    price: prices[count],
                    size,
                })
            }
        }
    }
    orders
}

// this returns quantity
fn step_size(exchange_info: &ExchangeInformation, symbol: &Symbol, size: f64, price: f64) -> f64 {
    for exchange_symbol_data in exchange_info.symbols.iter() {
        if *symbol.pair() == exchange_symbol_data.symbol {
            for filter in &exchange_symbol_data.filters {
                if let Filters::LotSize {
                    min_qty: _,
                    max_qty: _,
                    step_size,
                } = filter
                {
                    let amount = size / price;
                    if amount % step_size != 0_f64 {
                        let rounded_amount = if *step_size != 1_f64 {
                            let shift = 10.0_f64.powf(step_size.fract().to_string().len() as f64);
                            let mut fract = amount.fract() * shift as f64;
                            fract -= fract.fract();
                            fract /= shift as f64;
                            fract + amount.trunc()
                        } else {
                            amount.floor()
                        };
                        return rounded_amount * price;
                    } else {
                        return size;
                    }
                }
            }
        }
    }
    size
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

fn check_filters(orders: &Vec<OrderStruct>, exchange_info: &ExchangeInformation) -> Result<(), ()> {
    for order in orders.iter() {
        for exchange_symbol_data in exchange_info.symbols.iter() {
            let exchange_symbol = &exchange_symbol_data.symbol;
            if *order.symbol == *exchange_symbol {
                let filters = &exchange_symbol_data.filters;
                for filter in filters {
                    match check_filter(order, filter) {
                        Ok(_) => (),
                        Err(_) => return Err(()),
                    }
                }
            }
        }
    }
    Ok(())
}

// TODO: improve this func
fn check_filter(order: &OrderStruct, filter: &Filters) -> Result<(), ()> {
    if order.size == 0.0 {
        return Err(());
    }
    match filter {
        Filters::LotSize {
            min_qty,
            max_qty,
            step_size,
        } => {
            if order.size < *min_qty {
                warn!(
                    "{} Requires min of {}, tried: {}",
                    order.symbol, min_qty, order.size
                );
            } else if order.size > *max_qty {
                warn!(
                    "{} Requires max of {}, tried: {}",
                    order.symbol, max_qty, order.size
                );
            } else if (order.size / order.price) % *step_size != 0.0 {
                warn!(
                    "{} Requires step size of {}: tried: {}",
                    order.symbol,
                    step_size,
                    (order.size / order.price)
                );
            }
            return Err(());
        }
        // TODO: add more checks
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use binance::rest_model::{Asks, Bids, OrderBook};
    use lazy_static::lazy_static;

    use super::*;
    use crate::interface::*;

    lazy_static! {
        static ref INTERFACE: BinanceInterface = BinanceInterface::new();
    }

    #[tokio::test]
    async fn test_step_size() {
        let exchange_info = INTERFACE.get_exchange_info().await.unwrap();

        assert_eq!(
            // 49.98,
            49.980000000000004,
            step_size(
                &exchange_info,
                &Symbol::new("USDC", "USDT"),
                50.020008003201276,
                0.9996,
            )
        );
        assert_eq!(
            // 101.28274,
            101.297533,
            step_size(
                &exchange_info,
                &Symbol::new("ADA", "USDC"),
                101.29791117420402,
                0.4931,
            )
        );
        assert_eq!(
            // 10.148192,
            101.5035328,
            step_size(
                &exchange_info,
                &Symbol::new("ADA", "USDT"),
                101.50375939849624,
                0.4912,
            )
        );
    }

    #[tokio::test]
    async fn test_calculate_profitablity() {
        let exchange_info = INTERFACE.get_exchange_info().await.unwrap();

        assert_eq!(
            (1.0, vec![49.980000000000004,], vec![0.9996, 0.4931, 0.4912],),
            calculate_profitablity(
                &exchange_info,
                &[
                    (Symbol::new("USDC", "USDT"), ArbOrd::Sell),
                    (Symbol::new("ADA", "USDC"), ArbOrd::Sell),
                    (Symbol::new("ADA", "USDT"), ArbOrd::Buy),
                ],
                [
                    OrderBook {
                        last_update_id: 1,
                        bids: vec![Bids {
                            qty: 50.020008003201276,
                            price: 0.9996,
                        }],
                        asks: vec![Asks {
                            qty: 50.020008003201276,
                            price: 0.9996,
                        }]
                    },
                    OrderBook {
                        last_update_id: 1,
                        bids: vec![Bids {
                            qty: 101.29791117420402,
                            price: 0.4931,
                        }],
                        asks: vec![Asks {
                            qty: 101.29791117420402,
                            price: 0.4931,
                        }]
                    },
                    OrderBook {
                        last_update_id: 1,
                        bids: vec![Bids {
                            qty: 101.50375939849624,
                            price: 0.4912,
                        }],
                        asks: vec![Asks {
                            qty: 101.50375939849624,
                            price: 0.4912,
                        }]
                    }
                ],
            )
        );
    }
}

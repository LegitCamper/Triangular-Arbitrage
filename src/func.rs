use binance::rest_model::{ExchangeInformation, Filters, OrderBook, TradeFees};
use itertools::Itertools;
use log::{trace, info};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, sync::Arc};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
    time::{interval, Duration},
};

// Configurations
const STABLE_COINS: [&str; 1] = ["USDT"]; // "TUSD", "BUSD", "USDC", "DAI"
const STARTING_AMOUNT: Decimal = dec!(50.0); // Staring amount in USD
const MINIMUN_PROFIT: Decimal = dec!(0.0000001); // in USD

#[derive(Debug, Deserialize, Clone)]
pub struct Key {
    pub key: String,
    pub secret: String,
}

pub fn read_key() -> Key {
    let mut file = File::open("key.json").expect("Could not read the json file");
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents)
        .expect("Could not deserialize the file, error");
    serde_json::from_str(&contents.as_str()).expect("Could not deserialize")
}

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
        if pairs[0].pair1 == pairs[1].pair1 || pairs[0].pair1 == pairs[1].pair2 {
            (pairs[0].clone(), ArbOrd::Buy)
        } else if pairs[0].pair2 == pairs[1].pair1 || pairs[0].pair2 == pairs[1].pair2 {
            (pairs[0].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
        // get second order
        if pairs[1].pair1 == pairs[2].pair1 || pairs[1].pair1 == pairs[2].pair2 {
            (pairs[1].clone(), ArbOrd::Buy)
        } else if pairs[1].pair2 == pairs[2].pair1 || pairs[1].pair2 == pairs[2].pair2 {
            (pairs[1].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
        // get third order
        if pairs[2].pair1 == pairs[0].pair1 || pairs[2].pair1 == pairs[0].pair2 {
            (pairs[2].clone(), ArbOrd::Buy)
        } else if pairs[2].pair2 == pairs[0].pair1 || pairs[2].pair2 == pairs[0].pair2 {
            (pairs[2].clone(), ArbOrd::Sell)
        } else {
            unreachable!()
        },
    ]
}

fn find_fee(trade_fees: &TradeFees, symbol: &Symbol) -> Option<Decimal> {
    for trade_fee in trade_fees {
        if trade_fee.symbol == symbol.pair() {
            // Should always be taker
            return Some(trade_fee.taker_commission);
        }
    }
    None
}

// TODO: this assumes all stable coins in list are pinned at us dollar
fn calculate_profitablity(
    trading_fees: &TradeFees,
    exchange_info: &ExchangeInformation,
    order: &[(Symbol, ArbOrd); 3],
    coin_storage: [OrderBook; 3],
    starting_amount: Option<Decimal>,
    // returns (profit, qtys, prices)
) -> Option<(Decimal, Vec<Decimal>, Vec<Decimal>)> {
    let mut coin_qty = starting_amount.unwrap_or(STARTING_AMOUNT);
    let mut qtys = vec![];
    let mut prices = vec![];
    for (coin_data, (symbol, orders)) in coin_storage.iter().zip(order.iter()) {
        let (price, qty) = match &orders {
            ArbOrd::Buy => {
                (coin_data.asks[0].price, coin_data.asks[0].qty)
            }
            ArbOrd::Sell => {
                (coin_data.bids[0].price, coin_data.bids[0].qty)
            }
        };
        let amount = adhere_filters(
            exchange_info,
            symbol,
            coin_qty, // this was qty and was WRONG
            price,
        )?;
        if qty < amount {
            // rerun with decreased starting amount
            return calculate_profitablity(trading_fees, exchange_info, order, coin_storage, Some(qty))
        } else {
            qtys.push(amount);
            prices.push(price);
            coin_qty = amount
        }

        // TODO: this might be able to be paid with the amount cut for filters
        coin_qty -= coin_qty * find_fee(&trading_fees, symbol)?;
    }
    Some((coin_qty - starting_amount.unwrap_or(STARTING_AMOUNT), qtys, prices))
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct OrderStruct {
    pub symbol: String,
    pub side: ArbOrd,
    pub price: Decimal,
    pub amount: Decimal,
}

pub async fn find_tri_arb(
    valid_coin_pairs: Vec<[Symbol; 3]>,
    validator_writer: mpsc::UnboundedSender<Vec<OrderStruct>>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    exchange_info: ExchangeInformation,
    trading_fees: TradeFees,
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
                    let (profit, prices, amounts) = match calculate_profitablity(
                        &trading_fees,
                        &exchange_info,
                        &orders,
                        [pair0.clone(), pair1.clone(), pair2.clone()],
                        None
                    ) {
                        Some((profit, prices, amounts)) => (profit, prices, amounts),
                        None => continue 'inner,
                    };
                    if profit >= MINIMUN_PROFIT {
                        info!("Profit: {profit}, pairs: {:?}", orders);
                        let orders = create_order(
                            &trading_fees,
                            orders,
                            amounts.as_slice(),
                            prices.as_slice(),
                        )
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
    trading_fees: &TradeFees,
    orders_order: [(Symbol, ArbOrd); 3],
    qtys: &[Decimal],
    prices: &[Decimal],
) -> Vec<OrderStruct> {
    let mut orders = vec![];

    for (count, (symbol, side)) in orders_order.iter().enumerate() {
        // calculate fees
        let amount = if count > 0 {
            qtys[count] - qtys[count] * find_fee(trading_fees, symbol).unwrap()
        } else {
            qtys[count]
        };
        match side {
            ArbOrd::Buy => orders.push(OrderStruct {
                symbol: symbol.pair(),
                side: side.clone(),
                price: prices[count],
                amount,
            }),
            ArbOrd::Sell => orders.push(OrderStruct {
                symbol: symbol.pair(),
                side: side.clone(),
                price: prices[count],
                amount,
            }),
        }
    }
    orders
}

fn symbol_filters(symbol: &Symbol, exchange_info: &ExchangeInformation) -> Option<Vec<Filters>> {
    for exchange_symbol_data in exchange_info.symbols.iter() {
        if *symbol.pair() == exchange_symbol_data.symbol {
            return Some(exchange_symbol_data.filters.clone());
        }
    }
    None
}

fn round_step_size(amount: Decimal, step_size: Decimal) -> Decimal {
    if step_size < dec!(1) {
        let shift = step_size.fract().normalize().scale();
        return amount.trunc() + amount.fract().trunc_with_scale(shift);
    } else {
        return amount.floor();
    };
}

// checks binance trade filters and returns new amounts in accordance
fn adhere_filters(
    exchange_info: &ExchangeInformation,
    symbol: &Symbol,
    mut amount: Decimal,
    _price: Decimal,
) -> Option<Decimal> {
    let filters = symbol_filters(symbol, exchange_info)?;
    for filter in filters {
        match filter {
            Filters::LotSize {
                min_qty,
                max_qty,
                step_size,
            } => {
                if amount < min_qty {
                    // TRIED TO BUY LESS THAN THE REQUIRED AMOUNT,
                    return None;
                }
                if amount > max_qty {
                    amount = max_qty;
                }

                if (amount % step_size) != dec!(0) {
                    return Some(round_step_size(amount, step_size));
                } else {
                    return Some(amount);
                }
            }
            // TODO: add more checks
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
   use binance::rest_model::{Asks, Bids, ExchangeInformation, Filters, OrderBook};
    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
    static ref EXCHANGE_INFORMATION: ExchangeInformation = {
        let mut file = File::open("src/test_files/exchange_information.json").unwrap();
        let mut contents = String::new();
       let _ = file.read_to_string(&mut contents);
        serde_json::from_str(&contents.as_str()).unwrap()
    };
    static ref TRADING_FEES: TradeFees= {
        let mut file = File::open("src/test_files/trading_fees.json").unwrap();
        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents);
        serde_json::from_str(&contents.as_str()).unwrap()
    };
        }

    fn get_step_size(symbol: &Symbol, exchange_info: &ExchangeInformation) -> Option<Decimal> {
        for filter in symbol_filters(symbol, exchange_info).unwrap() {
            if let Filters::LotSize {
                min_qty: _,
                max_qty: _,
                step_size,
            } = filter
            {
                return Some(step_size.normalize());
            }
        }
        None
    }

    #[tokio::test]
    async fn test_step_size() {
        let mut symbol: Symbol;

        symbol = Symbol::new("USDC", "USDT");
        assert_eq!(dec!(1), get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap());
        assert_eq!(
            dec!(50),
            round_step_size(
                dec!(50.020008003201276),
                get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
            )
        );

        symbol = Symbol::new("ADA", "USDC");
        assert_eq!(dec!(0.1), get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap());
        assert_eq!(
            dec!(101.2),
            round_step_size(
                dec!(101.29791117420402),
                get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
            )
        );

        symbol = Symbol::new("ADA", "USDT");
        assert_eq!(dec!(0.1), get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap());
        assert_eq!(
            dec!(101.5),
            round_step_size(
                dec!(101.50375939849624),
                get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
            )
        );
    }

    #[tokio::test]
    async fn test_calculate_profitablity() {
            assert_eq!(
                Some((
                    dec!(-0.8964),                           // profit
                    vec![dec!(50.0),dec!(49.7000),dec!(49.4)],// qtys
                    vec![dec!(1.0), dec!(1.0), dec!(1.0)],   // prices
                )),
                calculate_profitablity(
                    &TRADING_FEES,
                    &EXCHANGE_INFORMATION,
                    &[
                        (Symbol::new("USDC", "USDT"), ArbOrd::Sell),
                        (Symbol::new("ADA", "USDC"), ArbOrd::Sell),
                        (Symbol::new("ADA", "USDT"), ArbOrd::Buy),
                    ],
                    [
                        OrderBook {
                            last_update_id: 1,
                            bids: vec![Bids {
                                qty: dec!(50.029),
                                price: dec!(1.0),
                            }],
                            asks: vec![Asks {
                                qty: dec!(50.029),
                                price: dec!(1.0),
                            }]
                        },
                        OrderBook {
                            last_update_id: 1,
                            bids: vec![Bids {
                                qty: dec!(101.299),
                                price: dec!(1.0),
                            }],
                            asks: vec![Asks {
                                qty: dec!(101.299),
                                price: dec!(1.0),
                            }]
                        },
                        OrderBook {
                            last_update_id: 1,
                            bids: vec![Bids {
                                qty: dec!(101.503),
                                price: dec!(1.0),
                            }],
                            asks: vec![Asks {
                                qty: dec!(1001.503),
                                price: dec!(1.0),
                            }]
                        }
                    ],
                    None
                )
            );
    }
}

use binance::rest_model::{ExchangeInformation, Filters, OrderBook, TradeFees};
use itertools::Itertools;
use log::trace;
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
const MINIMUN_PROFIT: Decimal = dec!(0.001); // in USD

#[derive(Debug, Deserialize, Clone)]
pub struct Key {
    pub key: String,
    pub secret: String,
}

pub fn read_key() -> Key {
    let mut file = File::open("key.json").expect("Could not read the json file");
    let mut contents = String::new();
    let _ = file
        .read_to_string(&mut contents)
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
pub enum OrderSide {
    Buy,
    Sell,
}

// TODO: should calulate this during catalog build in the future to prevent wasted IO
fn find_order_order(pairs: &[Symbol; 3]) -> [(Symbol, OrderSide); 3] {
    [
        // get first order
        if pairs[0].pair1 == pairs[1].pair1 || pairs[0].pair1 == pairs[1].pair2 {
            (pairs[0].clone(), OrderSide::Buy)
        } else if pairs[0].pair2 == pairs[1].pair1 || pairs[0].pair2 == pairs[1].pair2 {
            (pairs[0].clone(), OrderSide::Sell)
        } else {
            unreachable!()
        },
        // get second order
        if pairs[1].pair1 == pairs[2].pair1 || pairs[1].pair1 == pairs[2].pair2 {
            (pairs[1].clone(), OrderSide::Buy)
        } else if pairs[1].pair2 == pairs[2].pair1 || pairs[1].pair2 == pairs[2].pair2 {
            (pairs[1].clone(), OrderSide::Sell)
        } else {
            unreachable!()
        },
        // get third order
        if pairs[2].pair1 == pairs[0].pair1 || pairs[2].pair1 == pairs[0].pair2 {
            (pairs[2].clone(), OrderSide::Buy)
        } else if pairs[2].pair2 == pairs[0].pair1 || pairs[2].pair2 == pairs[0].pair2 {
            (pairs[2].clone(), OrderSide::Sell)
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
    order_data: &[(Symbol, OrderSide, OrderBook)],
    starting_amount: Option<Decimal>,
    // returns (profit, fees, qtys, prices)
) -> Option<(Decimal, Decimal, [Decimal; 3], [Decimal; 3])> {
    let starting_amount = starting_amount.unwrap_or(STARTING_AMOUNT);
    let mut coin_qty = starting_amount;
    let mut fees = dec!(0.0);
    let mut qtys: [Decimal; 3] = [dec!(0.0), dec!(0.0), dec!(0.0)];
    let mut prices: [Decimal; 3] = [dec!(0.0), dec!(0.0), dec!(0.0)];

    for (counter, (symbol, order_side, orderbook)) in order_data.iter().enumerate() {
        let (price, qty) = match &order_side {
            OrderSide::Buy => (orderbook.asks[0].price, orderbook.asks[0].qty),
            OrderSide::Sell => (orderbook.bids[0].price, orderbook.bids[0].qty),
        };
        let amount = adhere_filters(exchange_info, symbol, coin_qty, price)?;
        if qty < amount {
            // rerun with decreased starting amount
            return calculate_profitablity(trading_fees, exchange_info, order_data, Some(qty));
        }

        qtys[counter] = amount;
        prices[counter] = price;
        coin_qty = amount;

        // TODO: this might be able to be paid with the amount cut for filters
        fees += coin_qty * find_fee(&trading_fees, symbol)?;
    }
    coin_qty -= fees;
    coin_qty -= starting_amount;
    Some((coin_qty, fees, qtys, prices))
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct OrderStruct {
    pub symbol: String,
    pub side: OrderSide,
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
                let new_orderbook = clone_orderbook(&pairs, &orderbook).await;
                if let Some((pair0, pair1, pair2)) = new_orderbook {
                    if pair0.bids.is_empty()
                        || pair0.asks.is_empty()
                        || pair1.bids.is_empty()
                        || pair1.asks.is_empty()
                        || pair2.bids.is_empty()
                        || pair2.asks.is_empty()
                    {
                        // A pair was empty
                        continue 'inner;
                    };

                    let order_data = find_order_order(pairs)
                        .into_iter()
                        .zip([pair0, pair1, pair2])
                        .map(|(order, pair)| (order.0, order.1, pair))
                        .collect::<Vec<(Symbol, OrderSide, OrderBook)>>();
                    let order_data = order_data.as_slice();

                    let profitablity =
                        calculate_profitablity(&trading_fees, &exchange_info, &order_data, None);
                    if let Some((profit, _, prices, amounts)) = profitablity {
                        if profit >= MINIMUN_PROFIT {
                            let orders = create_order(
                                &trading_fees,
                                &order_data,
                                amounts.as_slice(),
                                prices.as_slice(),
                            )
                            .await;

                            // removing price that led to order
                            remove_bought(orderbook.clone(), &order_data).await;
                            validator_writer.send(orders).unwrap();
                        }
                    } else {
                        continue 'inner;
                    };
                }
            }
        }
    })
}

async fn remove_bought(
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    order_data: &[(Symbol, OrderSide, OrderBook)],
) {
    let mut orderbook = orderbook.lock().await;
    for (symbol, side, _) in order_data.iter() {
        let pair = orderbook.get_mut(&symbol.pair()).unwrap();
        match side {
            OrderSide::Buy => {
                pair.asks.remove(0);
            }
            OrderSide::Sell => {
                pair.bids.remove(0);
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
    order_data: &[(Symbol, OrderSide, OrderBook)],
    qtys: &[Decimal],
    prices: &[Decimal],
) -> Vec<OrderStruct> {
    let mut orders = vec![];

    for (count, (symbol, side, _)) in order_data.iter().enumerate() {
        // calculate fees
        let amount = if count > 0 {
            qtys[count] - qtys[count] * find_fee(trading_fees, symbol).unwrap()
        } else {
            qtys[count]
        };
        match side {
            OrderSide::Buy => orders.push(OrderStruct {
                symbol: symbol.pair(),
                side: side.clone(),
                price: prices[count],
                amount,
            }),
            OrderSide::Sell => orders.push(OrderStruct {
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
    use binance::rest_model::{Asks, Bids, ExchangeInformation, Filters};
    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
        static ref EXCHANGE_INFORMATION: ExchangeInformation = {
            let mut file = File::open("src/test_files/exchange_information.json").unwrap();
            let mut contents = String::new();
            let _ = file.read_to_string(&mut contents);
            serde_json::from_str(&contents.as_str()).unwrap()
        };
        static ref TRADING_FEES: TradeFees = {
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
        assert_eq!(
            dec!(1),
            get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
        );
        assert_eq!(
            dec!(50),
            round_step_size(
                dec!(50.020008003201276),
                get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
            )
        );

        symbol = Symbol::new("ADA", "USDC");
        assert_eq!(
            dec!(0.1),
            get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
        );
        assert_eq!(
            dec!(101.2),
            round_step_size(
                dec!(101.29791117420402),
                get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
            )
        );

        symbol = Symbol::new("ADA", "USDT");
        assert_eq!(
            dec!(0.1),
            get_step_size(&symbol, &EXCHANGE_INFORMATION).unwrap()
        );
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
        let qty = dec!(50.12345);
        let price = dec!(1.0);
        let order_data = [
            (
                Symbol::new("USDC", "USDT"),
                OrderSide::Sell,
                OrderBook {
                    last_update_id: 1,
                    bids: vec![Bids { qty, price }],
                    asks: vec![Asks { qty, price }],
                },
            ),
            (
                Symbol::new("ADA", "USDC"),
                OrderSide::Sell,
                OrderBook {
                    last_update_id: 1,
                    bids: vec![Bids { qty, price }],
                    asks: vec![Asks { qty, price }],
                },
            ),
            (
                Symbol::new("ADA", "USDT"),
                OrderSide::Sell,
                OrderBook {
                    last_update_id: 1,
                    bids: vec![Bids { qty, price }],
                    asks: vec![Asks { qty, price }],
                },
            ),
        ];

        let (profit, fees, _, _) =
            calculate_profitablity(&TRADING_FEES, &EXCHANGE_INFORMATION, &order_data, None)
                .unwrap();
        assert_eq!(dec!(0.9), fees); // (0.006 * 50) * 3
        assert_eq!(dec!(-0.9), profit);
    }
}

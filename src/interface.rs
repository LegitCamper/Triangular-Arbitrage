use crate::func::Key;
use binance::account::*;
use binance::api::*;
use binance::config::*;
use binance::general::*;
use binance::market::*;
use binance::rest_model::TradeFees;
use binance::rest_model::{ExchangeInformation, OrderBook};
use binance::wallet::*;
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::func::Symbol;

#[allow(dead_code)]
pub struct BinanceInterface {
    pub general: General,
    market: Market,
    account: Account,
    wallet: Wallet,
}

impl BinanceInterface {
    pub fn new(key: &Key, test: bool) -> Self {
        if test {
            let config = Config {
                binance_us_api: true,
                ..Config::testnet()
            };
            // if config.binance_us_api != true {
            //     panic!("fake");
            // } else {
            //     panic!("bullshit");
            // }
            BinanceInterface {
                general: Binance::new_with_config(None, None, &config),
                market: Binance::new_with_config(None, None, &config),
                account: Binance::new_with_config(
                    Some(key.key.clone()),
                    Some(key.secret.clone()),
                    &config,
                ),
                wallet: Binance::new_with_config(
                    Some(key.key.clone()),
                    Some(key.secret.clone()),
                    &config,
                ),
            }
        } else {
            let config = Config {
                binance_us_api: true,
                ..Default::default()
            };
            BinanceInterface {
                general: Binance::new_with_config(None, None, &config),
                market: Binance::new_with_config(None, None, &config),
                account: Binance::new_with_config(None, None, &config),
                wallet: Binance::new_with_config(None, None, &config),
            }
        }
    }

    pub async fn get_exchange_info(&self) -> Option<ExchangeInformation> {
        self.general.exchange_info().await.ok()
    }

    pub async fn get_account_fees(&self) -> Option<TradeFees> {
        println!("{:?}", self.wallet.trade_fees(None).await);
        self.wallet.trade_fees(None).await.ok()
    }

    pub async fn get_server_time(&self) -> Option<i64> {
        self.general
            .get_server_time()
            .await
            .unwrap()
            .server_time
            .try_into()
            .ok()
    }

    pub async fn get_pairs(&self) -> Option<(Vec<Symbol>, Vec<String>)> {
        let general_info = self.general.exchange_info().await.ok()?;

        let mut symbols: Vec<String> = vec![];
        let mut pairs: Vec<Symbol> = vec![];

        for symbol in general_info.symbols.iter() {
            if symbol.base_asset.as_str() != "USD" || symbol.quote_asset.as_str() != "USD" {
                pairs.push(Symbol::new(
                    &symbol.base_asset.clone(),
                    &symbol.quote_asset.clone(),
                ));
                symbols.push(symbol.base_asset.clone());
                symbols.push(symbol.quote_asset.clone());
            }
        }

        Some((pairs, symbols.into_iter().sorted().dedup().collect()))
    }

    pub async fn starter_orderbook(
        &self,
        symbols: &[Symbol],
    ) -> Arc<Mutex<HashMap<String, OrderBook>>> {
        let mut orderbook = HashMap::new();
        for symbol in symbols.iter().filter(|s| !s.pair().contains("4")) {
            let depth = self.market.get_depth(symbol.pair()).await.unwrap();
            orderbook.insert(symbol.pair(), depth);
        }

        Arc::new(Mutex::new(orderbook))
    }
}

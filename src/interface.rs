use binance::account::*;
use binance::api::*;
use binance::general::*;
use binance::market::*;
use binance::rest_model::{ExchangeInformation, OrderBook};
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::func::Symbol;

#[allow(dead_code)]
pub struct BinanceInterface {
    pub general: General,
    market: Market,
    account: Account,
}

impl BinanceInterface {
    pub fn new() -> Self {
        BinanceInterface {
            general: Binance::new(None, None),
            market: Binance::new(None, None),
            account: Binance::new(None, None),
        }
    }

    pub async fn get_exchange_info(&self) -> Option<ExchangeInformation> {
        self.general.exchange_info().await.ok()
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

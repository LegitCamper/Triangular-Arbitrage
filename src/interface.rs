use binance::account::*;
use binance::api::*;

use binance::general::*;
use binance::market::*;

use binance::rest_model::OrderBook;

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct BinanceInterface {
    general: General,
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
    pub async fn get_symbols(&self) -> Option<Vec<String>> {
        let general_info = self.general.exchange_info().await.ok()?;

        let mut symbols: Vec<String> = Vec::new();

        for symbol in general_info.symbols.iter() {
            let symbol = symbol.symbol.clone(); //.to_lowercase();
            symbols.push(symbol);
        }

        Some(symbols)
    }

    pub async fn get_pairs(&self) -> Option<Vec<(String, String)>> {
        let general_info = self.general.exchange_info().await.ok()?;

        let mut symbols: Vec<(String, String)> = Vec::new();

        for symbol in general_info.symbols.iter() {
            if symbol.base_asset.as_str() != "USD" || symbol.quote_asset.as_str() != "USD" {
                symbols.push((symbol.base_asset.clone(), symbol.quote_asset.clone()));
            }
        }

        Some(symbols)
    }

    pub async fn starter_orderbook(
        &self,
        symbols: &Vec<String>,
    ) -> Arc<Mutex<HashMap<String, OrderBook>>> {
        let mut orderbook = HashMap::new();
        for symbol in symbols.iter() {
            let depth = self.market.get_depth(symbol).await.unwrap();
            orderbook.insert(symbol.to_owned(), depth);
        }

        Arc::new(Mutex::new(orderbook))
    }
}

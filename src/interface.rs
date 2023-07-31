use binance::account::*;
use binance::api::*;
use binance::config::Config;
use binance::general;
use binance::general::*;
use binance::market::*;
use binance::rest_model::Symbol;
use binance::rest_model::{OrderBook, OrderSide, OrderType, SymbolPrice, TimeInForce};
use log::{error, info};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

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
    // async fn account() {
    //     let market: Market = Binance::new(None, None);
    //     let account: Account = Binance::new_with_env(&Config::testnet());
    //     let symbol = "BTCUSDT";
    //     let SymbolPrice { price, .. } = market.get_price(symbol).await.unwrap();
    //     match account.get_account().await {
    //         Ok(answer) => info!("{:?}", answer.balances),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     match account.get_open_orders(symbol).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let limit_buy = OrderRequest {
    //         symbol: symbol.to_string(),
    //         quantity: Some(0.001),
    //         price: Some(price),
    //         order_type: OrderType::Limit,
    //         side: OrderSide::Buy,
    //         time_in_force: Some(TimeInForce::FOK),
    //         ..OrderRequest::default()
    //     };
    //     match account.place_order(limit_buy).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let market_buy = OrderRequest {
    //         symbol: symbol.to_string(),
    //         quantity: Some(0.001),
    //         order_type: OrderType::Market,
    //         side: OrderSide::Buy,
    //         ..OrderRequest::default()
    //     };
    //     match account.place_order(market_buy).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let limit_sell = OrderRequest {
    //         symbol: symbol.to_string(),
    //         quantity: Some(0.001),
    //         price: Some(price),
    //         order_type: OrderType::Limit,
    //         side: OrderSide::Sell,
    //         time_in_force: Some(TimeInForce::FOK),
    //         ..OrderRequest::default()
    //     };
    //     match account.place_order(limit_sell).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let market_sell = OrderRequest {
    //         symbol: symbol.to_string(),
    //         quantity: Some(0.001),
    //         order_type: OrderType::Market,
    //         side: OrderSide::Sell,
    //         ..OrderRequest::default()
    //     };
    //     match account.place_order(market_sell).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let order_id = 1_957_528;
    //     let order_status = OrderStatusRequest {
    //         symbol: symbol.to_string(),
    //         order_id: Some(order_id),
    //         ..OrderStatusRequest::default()
    //     };

    //     match account.order_status(order_status).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     let order_cancellation = OrderCancellation {
    //         symbol: symbol.to_string(),
    //         order_id: Some(order_id),
    //         ..OrderCancellation::default()
    //     };

    //     match account.cancel_order(order_cancellation).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     match account.get_balance("BTC").await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }

    //     match account.trade_history(symbol).await {
    //         Ok(answer) => info!("{:?}", answer),
    //         Err(e) => error!("Error: {e}"),
    //     }
    // }

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

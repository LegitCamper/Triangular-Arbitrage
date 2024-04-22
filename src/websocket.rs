use binance::{
    account::*,
    api::*,
    rest_model::{Asks, Bids, ExchangeInformation, OrderBook, OrderType, TimeInForce},
    userstream::*,
    websockets::*,
    ws_model::{CombinedStreamEvent, OrderUpdate, WebsocketEvent, WebsocketEventUntag},
};
// use chrono::Utc;
use log::{error, info, warn};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot, Mutex,
    },
    task::JoinHandle,
    time::timeout,
};

use crate::func::{Key, OrderStruct, OrderSide};

pub async fn start_market_websockets(
    keep_running: Arc<AtomicBool>,
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    symbols: &[String],
) -> (JoinHandle<()>, Vec<JoinHandle<()>>) {
    let (tx, mut rx) = unbounded_channel();

    let mut orderbook_handles = Vec::new();
    for stream in symbols.iter() {
        orderbook_handles.push(multiple_orderbook(
            keep_running.clone(),
            tx.clone(),
            vec![stream.to_string()],
            stream.to_string(),
        ));
    }
    let orderbook_sort_handle = tokio::spawn(async move {
        loop {
            if let Some((symbol, data)) = rx.recv().await {
                let mut orderbook = orderbook.lock().await;

                match orderbook.get(&symbol) {
                    Some(old_data) => {
                        if old_data.last_update_id < data.last_update_id {
                            orderbook.insert(symbol, sort_by_price(data)).unwrap();
                        } else {
                            warn!("New data is not newer than old data");
                        }
                    }
                    None => {
                        warn!("Symbol does not exist in orderbook");
                        orderbook.insert(symbol, data).unwrap();
                    }
                }
            } else {
                error!("orderbook websocket channel None");
            }
        }
    });

    (orderbook_sort_handle, orderbook_handles)
}

pub async fn start_order_creator(
    keep_running: Arc<AtomicBool>,
    key: Key,
    exchange_info: &ExchangeInformation,
    server_time: &i64,
) -> (
    JoinHandle<()>,
    UnboundedSender<Vec<OrderStruct>>,
    JoinHandle<()>,
) {
    let (tx, rx) = unbounded_channel::<Vec<OrderStruct>>();
    let (orders_placed_rx, user_websocket_handle) =
        user_stream(keep_running.clone(), key.clone()).await;
    let user_handle = place_orders(
        keep_running.clone(),
        key,
        exchange_info.clone(),
        server_time.clone(),
        rx,
        orders_placed_rx,
    )
    .await;

    (user_handle, tx, user_websocket_handle)
}

fn sort_by_price(mut orderbook: OrderBook) -> OrderBook {
    orderbook.bids = sort_bids(orderbook.bids);
    orderbook.asks = sort_asks(orderbook.asks);
    orderbook
}
fn sort_bids(mut vector: Vec<Bids>) -> Vec<Bids> {
    let mut swapped = true;
    while swapped {
        // No swap means array is sorted.
        swapped = false;
        for i in 1..vector.len() {
            if vector[i - 1].price < vector[i].price {
                vector.swap(i - 1, i);
                swapped = true
            }
        }
    }
    vector
}
fn sort_asks(mut vector: Vec<Asks>) -> Vec<Asks> {
    let mut swapped = true;
    while swapped {
        // No swap means array is sorted.
        swapped = false;
        for i in 1..vector.len() {
            if vector[i - 1].price > vector[i].price {
                vector.swap(i - 1, i);
                swapped = true
            }
        }
    }
    vector
}

fn multiple_orderbook(
    keep_running: Arc<AtomicBool>,
    channel: UnboundedSender<(String, OrderBook)>,
    streams: Vec<String>,
    symbol: String,
) -> JoinHandle<()> {
    info!("spawning websocket for: {symbol:?}");
    tokio::spawn(async move {
        let streams: Vec<String> = streams
            .into_iter()
            .map(|symbol| partial_book_depth_stream(symbol.to_lowercase().as_str(), 20, 100))
            .collect();

        let mut web_socket: WebSockets<'_, CombinedStreamEvent<_>> =
            WebSockets::new(|event: CombinedStreamEvent<WebsocketEventUntag>| {
                match event.data {
                    WebsocketEventUntag::WebsocketEvent(we) => {
                        warn!("Combined Orderbook Issue: {:?}", we)
                    }
                    WebsocketEventUntag::Orderbook(data) => {
                        channel.send((symbol.clone(), *data)).unwrap()
                    }
                    WebsocketEventUntag::BookTicker(bt) => {
                        info!("book tick: {:?}", bt)
                    }
                }

                Ok(())
            });

        match web_socket.connect_multiple(streams).await {
            Ok(_) => {}
            Err(e) => error!("{symbol:?} Websocket Error: {e}"),
        }
        if let Err(e) = web_socket.event_loop(&keep_running).await {
            error!("{symbol:?} Websocket Error: {e}");
        }
        web_socket.disconnect().await.unwrap();
        info!("{symbol:?} Websocket Disconnected");
    })
}

async fn place_orders(
    keep_running: Arc<AtomicBool>,
    key: Key,
    exchange_info: ExchangeInformation,
    _server_time: i64,
    mut orders: UnboundedReceiver<Vec<OrderStruct>>,
    mut placed_orders: UnboundedReceiver<OrderUpdate>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let account: Account = Binance::new(Some(key.key), Some(key.secret));
        // fix server_time_diff to be allgined with server time
        // account.server_time_diff = Utc::now().timestamp_millis() + server_time;

        loop {
            let mut created_order = false;
            match orders.try_recv() {
                Ok(orders) => {
                    info!("Received orders: {:?}", orders);
                    for order in orders.iter() {
                        let (tx, rx) = oneshot::channel::<String>();
                        if let Err(_) = timeout(Duration::from_secs(60), rx).await {
                            error!("Did not receive response from user websocket with order");
                            unwind_orders().await;
                            break;
                        }

                        info!("Placing Orders: {:?}", order);
                        place_order(order, &exchange_info, &account).await;

                        // ensure order has gone through before continuing with following orders
                        let placed_order = placed_orders
                            .recv()
                            .await
                            .expect("user websocket closed channel or order");
                        if placed_order.symbol == order.symbol && placed_order.price == order.price
                        {
                            tx.send("Recived".into()).unwrap();
                            info!("order went through succesfully");
                        } else {
                            unwind_orders().await;
                            keep_running.swap(false, std::sync::atomic::Ordering::Relaxed);
                            panic!("Order failed to proccess in the correct order or at all");
                        }
                    }
                    created_order = true;
                }
                Err(_) => (),
            }

            // drain channel of expired orders if created an order
            // maybe check how old order is and only drop it if exceeds time
            if created_order {
                while let Ok(_) = orders.try_recv() {}
            }
        }
    })
}

// TODO: design func to unwind 'stuck' orders
async fn unwind_orders() {}

async fn place_order(order: &OrderStruct, exchange_info: &ExchangeInformation, account: &Account) {
    let symbol = order.symbol.clone();
    let limit_order = match order.side {
        OrderSide::Buy => OrderRequest {
            symbol: symbol.to_string(),
            quantity: Some(order.amount),
            price: Some(order.price),
            order_type: OrderType::Limit,
            quote_order_qty: None,
            side: binance::rest_model::OrderSide::Buy,
            time_in_force: Some(TimeInForce::FOK),
            new_client_order_id: None,
            stop_price: None,
            iceberg_qty: None,
            new_order_resp_type: None,
            ..OrderRequest::default()
        },
        OrderSide::Sell => OrderRequest {
            symbol: symbol.to_string(),
            quantity: Some(order.amount),
            price: Some(order.price),
            order_type: OrderType::Limit,
            quote_order_qty: None,
            side: binance::rest_model::OrderSide::Sell,
            time_in_force: Some(TimeInForce::FOK),
            new_client_order_id: None,
            stop_price: None,
            iceberg_qty: None,
            new_order_resp_type: None,
            ..OrderRequest::default()
        },
    };
    let _precision = get_precision(&symbol, &exchange_info).unwrap();
    match account.place_order(limit_order).await {
        Ok(answer) => info!("{:?}", answer),
        Err(e) => error!("Error: {e}"),
    }
}

fn get_precision(symbol: &String, exchange_info: &ExchangeInformation) -> Option<(u8, u8)> {
    for exchange_symbol_data in exchange_info.symbols.iter() {
        let exchange_symbol = &exchange_symbol_data.symbol;
        if *symbol == *exchange_symbol {
            return Some((
                exchange_symbol_data.base_asset_precision as u8,
                exchange_symbol_data.quote_precision as u8,
            ));
        }
    }
    None
}

#[allow(dead_code)]
async fn user_stream(
    keep_running: Arc<AtomicBool>,
    key: Key,
) -> (UnboundedReceiver<OrderUpdate>, JoinHandle<()>) {
    let (tx, rx) = unbounded_channel::<OrderUpdate>();
    let user_stream: UserStream = Binance::new(Some(key.key), Some(key.secret));

    let handle = tokio::spawn(async move {
        if let Ok(answer) = user_stream.start().await {
            let listen_key = answer.listen_key;

            let mut web_socket: WebSockets<'_, WebsocketEvent> =
                WebSockets::new(|event: WebsocketEvent| {
                    if let WebsocketEvent::OrderUpdate(trade) = event {
                        info!(
                            "Symbol: {}, Side: {:?}, Price: {}, Execution Type: {:?}",
                            trade.symbol, trade.side, trade.price, trade.execution_type
                        );
                        tx.send(*trade).expect("Failed to send trade");
                    };

                    Ok(())
                });

            web_socket.connect(&listen_key).await.unwrap(); // check error

            if let Err(e) = web_socket.event_loop(&keep_running).await {
                error!("Error: {e}");
            }
            user_stream.close(&listen_key).await.unwrap();
            web_socket.disconnect().await.unwrap();
            error!("Userstrem closed and disconnected");
        } else {
            error!("Not able to start an User Stream (Check your API_KEY)");
        }
    });

    (rx, handle)
}

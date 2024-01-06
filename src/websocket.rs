use binance::account::*;
use binance::api::*;
use binance::rest_model::{Asks, Bids, OrderBook};
use binance::rest_model::{OrderSide, OrderType, TimeInForce};
// use binance::userstream::*;
use binance::websockets::*;
use binance::ws_model::{CombinedStreamEvent, WebsocketEventUntag}; // WebsocketEvent
use log::{error, info, warn};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
    time::sleep,
};

use crate::func::{self, OrderStruct};

pub async fn start_market_websockets(
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    symbols: &Vec<String>,
) -> (JoinHandle<()>, Vec<JoinHandle<()>>) {
    let (tx, mut rx) = unbounded_channel();

    let mut orderbook_handles = Vec::new();
    for stream in symbols.iter() {
        let symbol = stream.split("@").next().unwrap();
        orderbook_handles.push(multiple_orderbook(
            tx.clone(),
            vec![stream.to_string()],
            symbol.to_string(),
        ));
    }
    let orderbook_sort_handle = tokio::spawn(async move {
        loop {
            if let Some((symbol, data)) = rx.recv().await {
                let mut orderbook = orderbook.lock().await;
                // info!("Adding Symbol: {:?}", symbol);

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

pub async fn start_user_websocket(
    key: func::Key,
) -> (JoinHandle<()>, UnboundedSender<Vec<OrderStruct>>) {
    let (tx, rx) = unbounded_channel::<Vec<func::OrderStruct>>();
    let user_handle = user_stream_websocket(key, rx).await;

    (user_handle, tx)
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
    channel: UnboundedSender<(String, OrderBook)>,
    streams: Vec<String>,
    symbol: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let keep_running = AtomicBool::new(true);

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

async fn user_stream_websocket(
    key: func::Key,
    mut orders: UnboundedReceiver<Vec<func::OrderStruct>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // let keep_running = AtomicBool::new(true); // Used to control the event loop
        // let user_stream: UserStream = Binance::new(Some(key.key.clone()), Some(key.secret.clone()));
        let account: Account = Binance::new(Some(key.key), Some(key.secret));

        // if let Ok(answer) = user_stream.start().await {
        //     let listen_key = answer.listen_key;

        //     let mut web_socket: WebSockets<'_, WebsocketEvent> =
        //         WebSockets::new(|event: WebsocketEvent| {
        //             if let WebsocketEvent::OrderUpdate(trade) = event {
        //                 // info!(
        //                 //     "Symbol: {}, Side: {:?}, Price: {}, Execution Type: {:?}",
        //                 //     trade.symbol, trade.side, trade.price, trade.execution_type
        //                 // );
        //                 // orders.send(trade).expect("Failed to send trade")
        //             };
        //             Ok(())
        //         });

        //     web_socket.connect(&listen_key).await.unwrap(); // check error

        // listens for orders and passes them to the user websocket
        if let Some(orders) = orders.recv().await {
            info!("Received orders: {:?}", orders);
            // TODO: catch errors and reverse orders that fail in 2/3 or 3/3
            for order in orders.iter() {
                let symbol = order.symbol.clone();
                let limit = match order.side {
                    func::ArbOrd::Buy(_, _) => OrderRequest {
                        symbol: symbol.to_string(),
                        quantity: Some(order.size),
                        price: Some(order.price),
                        order_type: OrderType::Limit,
                        quote_order_qty: None,
                        side: OrderSide::Buy,
                        time_in_force: Some(TimeInForce::FOK),
                        // recv_window: Some(6000),
                        new_client_order_id: None,
                        stop_price: None,
                        iceberg_qty: None,
                        new_order_resp_type: None,
                        ..OrderRequest::default()
                    },
                    func::ArbOrd::Sell(_, _) => OrderRequest {
                        symbol: symbol.to_string(),
                        quantity: Some(order.size),
                        price: Some(order.price),
                        order_type: OrderType::Limit,
                        quote_order_qty: None,
                        side: OrderSide::Sell,
                        time_in_force: Some(TimeInForce::FOK),
                        // recv_window: Some(6000),
                        new_client_order_id: None,
                        stop_price: None,
                        iceberg_qty: None,
                        new_order_resp_type: None,
                        ..OrderRequest::default()
                    },
                };
                match account.place_order(limit).await {
                    Ok(answer) => info!("{:?}", answer),
                    Err(e) => error!("Error: {e}"),
                }
            }
            // sleep to ensure order goes through once at a time
            sleep(Duration::from_secs(10)).await;
        }
        // if let Err(e) = web_socket.event_loop(&keep_running).await {
        //     println!("Error: {e}");
        // }
        //     user_stream.close(&listen_key).await.unwrap();
        //     web_socket.disconnect().await.unwrap();
        //     println!("Userstrem closed and disconnected");
        // } else {
        //     println!("Not able to start an User Stream (Check your API_KEY)");
        // }
    })
}

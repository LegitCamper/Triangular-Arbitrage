use binance::rest_model::{Asks, Bids, OrderBook};
use binance::websockets::*;
use binance::ws_model::{CombinedStreamEvent, WebsocketEventUntag};
use log::{error, info, warn};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};

use super::interface;

pub async fn start_websocket(
    orderbook: Arc<Mutex<HashMap<String, OrderBook>>>,
    symbols: &Vec<String>,
) -> (JoinHandle<()>, Vec<JoinHandle<()>>) {
    let (rx, mut tx) = unbounded_channel();

    let mut orderbook_handles = Vec::new();
    for stream in symbols.iter() {
        let symbol = stream.split("@").next().unwrap();
        orderbook_handles.push(multiple_orderbook(
            rx.clone(),
            vec![stream.to_string()],
            symbol.to_string(),
        ));
    }

    let orderbook_sort_handle = tokio::spawn(async move {
        loop {
            if let Some((symbol, data)) = tx.recv().await {
                let mut orderbook = orderbook.lock().await;

                match orderbook.get(&symbol) {
                    Some(old_data) => {
                        if old_data.last_update_id < data.last_update_id {
                            orderbook.insert(symbol, sort_by_price(data)).unwrap();
                        }
                    }
                    None => {
                        orderbook.insert(symbol, data).unwrap();
                        ()
                    }
                }
            } else {
                warn!("oderbook channel error")
            }
        }
    });

    (orderbook_sort_handle, orderbook_handles)
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
            if vector[i - 1].price > vector[i].price {
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
            .map(|symbol| partial_book_depth_stream(symbol.as_str(), 5, 1000))
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

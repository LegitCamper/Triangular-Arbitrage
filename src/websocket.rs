use binance::api::*;
use binance::rest_model::{Asks, Bids, OrderBook};
use binance::userstream::*;
use binance::websockets::*;
use binance::ws_model::{CombinedStreamEvent, OrderUpdate, WebsocketEvent, WebsocketEventUntag};
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};

pub async fn start_websockets(
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
                error!("orderbook websocket channel None");
            }
        }
    });

    let (tx, rx) = unbounded_channel::<Box<OrderUpdate>>();
    user_stream_websocket(rx).await;

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
            .map(|symbol| partial_book_depth_stream(symbol.as_str(), 20, 100))
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

#[derive(Debug, Deserialize)]
struct Key {
    key: String,
}

fn read_key() -> String {
    let mut file = File::open("key.json").expect("Could not read the json file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Could not deserialize the file, error");
    serde_json::from_str(&contents.as_str()).expect("Could not deserialize")
}

#[allow(dead_code)]
async fn user_stream_websocket(mut orders: UnboundedReceiver<Box<OrderUpdate>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let keep_running = AtomicBool::new(true); // Used to control the event loop
        let api_key_user = Some(read_key().into());
        let user_stream: UserStream = Binance::new(api_key_user, None);

        if let Ok(answer) = user_stream.start().await {
            let listen_key = answer.listen_key;

            let mut web_socket: WebSockets<'_, WebsocketEvent> =
                WebSockets::new(|event: WebsocketEvent| {
                    if let WebsocketEvent::OrderUpdate(trade) = event {
                        println!(
                            "Symbol: {}, Side: {:?}, Price: {}, Execution Type: {:?}",
                            trade.symbol, trade.side, trade.price, trade.execution_type
                        );
                        // orders.send(trade).expect("Failed to send trade")
                    };

                    Ok(())
                });

            web_socket.connect(&listen_key).await.unwrap(); // check error

            // listens for orders and passes them to the user websocket
            while let Some(i) = orders.recv().await {
                info!("Received orders: {:?}", i)
            }
            if let Err(e) = web_socket.event_loop(&keep_running).await {
                println!("Error: {e}");
            }
            user_stream.close(&listen_key).await.unwrap();
            web_socket.disconnect().await.unwrap();
            println!("Userstrem closed and disconnected");
        } else {
            println!("Not able to start an User Stream (Check your API_KEY)");
        }
    })
}

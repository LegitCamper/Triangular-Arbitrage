use crate::kucoin::KucoinResponseL1 as KucoinRestResponse;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_this_or_that::as_f64;
use tokio::sync::mpsc;
use url::Url;

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct WebsocketDetailsL1 {
    data: WebsocketDetailsL2,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct WebsocketDetailsL2 {
    token: String,
    instanceServers: Vec<WebsocketDetailsL3>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct WebsocketDetailsL3 {
    endpoint: String,
    pingInterval: i32,
    pingTimeout: i32,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct KucoinWebsocketSubscription {
    id: i32,
    r#type: String,
    topic: String,
    privateChannel: String,
    response: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct KucoinWebscoketPing {
    id: i32,
    r#type: String,
}

// Kucoin websocket return - Serde
#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KucoinWebsocketResponseL0 {
    pub r#type: String,
    pub topic: String,
    pub subject: String,
    pub data: KucoinWebsocketResponseL1,
}
#[derive(Clone)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KucoinWebsocketResponseL1 {
    #[serde(deserialize_with = "as_f64")]
    pub bestAsk: f64,
    #[serde(deserialize_with = "as_f64")]
    pub bestAskSize: f64,
    #[serde(deserialize_with = "as_f64")]
    pub bestBid: f64,
    #[serde(deserialize_with = "as_f64")]
    pub bestBidSize: f64,
    #[serde(deserialize_with = "as_f64")]
    pub price: f64,
    #[serde(deserialize_with = "as_f64")]
    pub sequence: f64,
    #[serde(deserialize_with = "as_f64")]
    pub size: f64,
    #[serde(deserialize_with = "as_f64")]
    pub time: f64,
}

pub async fn kucoin_websocket(
    websocket_info: KucoinRestResponse,
    channel_writer: mpsc::Sender<KucoinWebsocketResponseL0>,
) {
    let websocket_url = Url::parse(
        format!(
            "{}?token={}",
            websocket_info.instanceServers[0].endpoint, websocket_info.token
        )
        .as_str(),
    )
    .unwrap();

    // Searilize kucoin subscription
    let kucoin_id: i32 = rand::thread_rng().gen_range(13..15);
    let subscription = json!(KucoinWebsocketSubscription {
        id: kucoin_id,
        r#type: "subscribe".to_string(),
        topic: "/market/ticker:all".to_string(),
        privateChannel: "false".to_string(),
        response: "false".to_string(),
    });
    // Searilize kucoin ping message
    let ping = json!(KucoinWebscoketPing {
        id: kucoin_id,
        r#type: "ping".to_string()
    });

    // Webscoket stuff
    let ws = workflow_websocket::client::WebSocket::new(
        websocket_url.as_ref(), // .to_string(),
        workflow_websocket::client::Options::default(),
    );
    ws.as_ref()
        .expect("Failed to connect to websocket")
        .connect(true)
        .await
        .unwrap();
    ws.as_ref()
        .expect("")
        .send(workflow_websocket::client::Message::Text(
            subscription.to_string(),
        ))
        .await
        .expect("Failed to subscribe to the websocket");

    // Send messages (Pings and subscription)
    let ws_send = ws.as_ref().expect("Could not clone ws for sender").clone();
    workflow_core::task::spawn(async move {
        loop {
            ws_send
                .send(workflow_websocket::client::Message::Text(ping.to_string()))
                .await
                .expect("Failed to send ping to websocket");
            workflow_core::task::sleep(std::time::Duration::from_millis(
                websocket_info.instanceServers[0].pingInterval,
            ))
            .await;
        }
    });

    // Recive messages (Symbol data)
    let ws_read = ws.expect("Could not clone ws for reader"); //.clone();
    workflow_core::task::spawn(async move {
        loop {
            let response = ws_read.recv();
            if let Ok(workflow_websocket::client::Message::Text(x)) = response.await {
                if x.contains("message") {
                    let res: KucoinWebsocketResponseL0 =
                        serde_json::from_str(x.as_str()).expect("Cannot desearlize websocket data");
                    channel_writer.send(res).await.expect("Failed to send");
                } else {
                    println!("Webosocket Response: {}", x);
                };
            }
        }
    });

    // wait for the tasks to finish (forever)
    tokio::time::sleep(std::time::Duration::MAX).await
}

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
pub struct KucoinWebsocketResponse {
    r#type: String,
    topic: String,
    subject: String,
    data: KucoinWebsocketResponseL1,
}
#[derive(Clone)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct KucoinWebsocketResponseL1 {
    #[serde(deserialize_with = "as_f64")]
    bestAsk: f64,
    #[serde(deserialize_with = "as_f64")]
    bestAskSize: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBid: f64,
    #[serde(deserialize_with = "as_f64")]
    bestBidSize: f64,
    #[serde(deserialize_with = "as_f64")]
    price: f64,
    #[serde(deserialize_with = "as_f64")]
    sequence: f64,
    #[serde(deserialize_with = "as_f64")]
    size: f64,
    #[serde(deserialize_with = "as_f64")]
    time: f64,
}

pub async fn kucoin_websocket(
    api_creds: KucoinCreds,
    // websocket_token: String,
    channel_writer: mpsc::Sender<KucoinWebsocketResponse>,
) {
    let empty_json_request = EmptyKucoinJson {
        string: "Nothing to see here!".to_string(),
    };
    // retreive temporary api token
    let websocket_info: WebsocketDetailsL1 = match kucoin_request(
        &api_creds,
        reqwest::Client::new(), // makes http client
        "/api/v1/bullet-public",
        serde_json::to_string(&empty_json_request).expect("Failed to Serialize"), // no json params req
        KucoinRequestType::WebsocketToken,
    )
    .await
    {
        Some(x) => serde_json::from_str(&x).expect("Cant't parse from json"),
        None => panic!("Did not get valid response from kucoin"),
    };

    let websocket_url = Url::parse(
        format!(
            "{}?token={}",
            websocket_info.data.instanceServers[0].endpoint, websocket_info.data.token
        )
        .as_str(),
    )
    .unwrap();

    // Searilize kucoin subscription
    let kucoin_id: i32 = rand::thread_rng().gen_range(13..15);
    let subscription = json!(kucoin_websocket_subscription {
        id: kucoin_id,
        r#type: "subscribe".to_string(),
        topic: "/market/ticker:all".to_string(),
        privateChannel: "false".to_string(),
        response: "false".to_string(),
    });
    // Searilize kucoin ping message
    let ping = json!(kucoin_webscoket_ping {
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
                websocket_info.data.instanceServers[0]
                    .pingInterval
                    .try_into()
                    .unwrap(),
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
                    let res: Kucoin_websocket_response =
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



#[macro_use]
extern crate lazy_static;
use axum::{routing::get, routing::post, Router, Json, extract::Path};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;
use std::{
    sync::{Arc}
};
use serde_json::json;

use futures::{sink::SinkExt, stream::StreamExt};
struct AppState {
    tx: broadcast::Sender<String>
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
struct Stock {
    symbol: String,
    price: u32
}

#[derive(Debug)]
struct StockData {
    data: HashMap<String, Stock>
}

struct WebsocketClientChannels {
    clients: Arc<Mutex<Vec<broadcast::Sender<String>>>>
}

lazy_static! {
    static ref STOCK_DATA: Mutex<StockData> = {
        let mut stock_data = Mutex::new(StockData {
            data: HashMap::new()
        });
        stock_data.lock().unwrap().data.insert("IBM".to_string(), Stock {symbol: "IBM".to_string(), price: 700});
        stock_data.lock().unwrap().data.insert("MSFT".to_string(), Stock {symbol: "MSFT".to_string(), price: 40});
        stock_data.lock().unwrap().data.insert("AMD".to_string(), Stock {symbol: "AMD".to_string(), price: 200});
        stock_data.lock().unwrap().data.insert("GOOGL".to_string(), Stock {symbol: "GOOGL".to_string(), price: 1000});
        stock_data
    };

    static ref CLIENTS: WebsocketClientChannels = {
        let clients = WebsocketClientChannels {
            clients: Arc::new(Mutex::new(Vec::new()))
        };
        clients
    };

    static ref TX_QUEUE_CHANNEL: tokio::sync::mpsc::Sender<Stock> = {
        let (tx_queue, rx_queue) = tokio::sync::mpsc::channel::<Stock>(100);
        tokio::spawn(receiver_queue(rx_queue));
        tx_queue
    };
}

async fn receiver_queue(mut rx_queue: tokio::sync::mpsc::Receiver<Stock>) {
    while let Some(stock) = rx_queue.recv().await {
        println!("Ready to broadcast change to external websockets: {:?}", stock);
        let msg = json!(stock);
        for tx in CLIENTS.clients.lock().unwrap().iter()  {
            let change_msg = format!("change_event|{}", msg.to_string());
            tx.send(change_msg);
        }

        // In any websocket error, break loop.
        //if sender.send(Message::Text(msg)).await.is_err() {
        //    break;
        //}
    };
}

#[tokio::main]
async fn main() {
    println!("{:?}", STOCK_DATA.lock().unwrap().data);
    let (tx, _rx) = broadcast::channel(100);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let app_state = Arc::new(AppState { 
                                                    tx: tx}
                                            );
    
    let app = Router::new()
    .route("/quote/:symbol", get(get_quote))
    .route("/quote", post(update_price))
    .route("/snapshot", get(get_snapshot))
    .route("/grid", get(grid))
    .route("/websocket", get(websocket_handler))
    .with_state(app_state);

    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize)]
struct StockSnapshotResponse {
    data: Vec<Stock>
}



#[derive(Serialize)]
struct StockQuoteResponse {
    symbol: String,
    price: u32,
    status: String
}

#[derive(Deserialize)]
struct ChangeStockPriceRequest {
    symbol: String,
    price: u32
}

#[derive(Serialize)]
struct ChangeStockPriceResponse {
    status: String
}

async fn get_snapshot() ->
     Result<impl IntoResponse, (StatusCode, Json<StockSnapshotResponse>)> {
    let dat_response = STOCK_DATA.lock().unwrap().data.values().cloned().collect();
    println!("{:?}", dat_response);
    Ok(
        Json(
            StockSnapshotResponse
            {
                data: dat_response
            }
        )
    )
}


async fn update_price(State(state): State<Arc<AppState>>, Json(payload): Json<ChangeStockPriceRequest>) 
    -> Result<impl IntoResponse, (StatusCode, Json<ChangeStockPriceResponse>)> {
    if  STOCK_DATA.lock().unwrap().data.contains_key(&payload.symbol) {
        STOCK_DATA.lock().unwrap().data.remove(&payload.symbol);
        let stock: Stock = Stock {symbol: payload.symbol.to_string(), price: payload.price};
        TX_QUEUE_CHANNEL.send(stock.clone()).await.unwrap();
        STOCK_DATA.lock().unwrap().data.insert(payload.symbol.to_string(), stock);
    }
    println!("{:?}", STOCK_DATA.lock().unwrap().data);
  
    Ok(Json(
        ChangeStockPriceResponse
        {
            status: "OK".to_string()
        })
    )
}


async fn grid() -> Html<&'static str> {
    Html(std::include_str!("../grid.html"))
}

async fn get_quote(Path(symbol) : Path<String>) -> Json<StockQuoteResponse> {
    // Generate a random number in range parsed from query.
    match STOCK_DATA.lock().unwrap().data.get(&symbol) {
        Some(stock) => {
            Json(
                StockQuoteResponse
                {
                    symbol: stock.symbol.to_string(), 
                    price: stock.price,                         
                    status: "OK".to_string()
                }
            )
        }
        _ => {
            Json(
                StockQuoteResponse
                {
                    symbol: "".to_string(), 
                    price: 0, 
                    status: "ERROR".to_string()
                })
        }

    }
    
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {  
    ws.on_upgrade(|socket| websocket(socket, state))
}




// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    // By splitting, we can send and receive at the same time.
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    let mut rx = state.tx.subscribe();

    // This task will receive watch messages and forward it to this connected client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            println!("send channel{}", msg);
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();
    CLIENTS.clients.lock().unwrap().push(tx.clone());
    let elems = vec![tx.clone()];
    // This task will receive messages from this client.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
            println!("receive channel {}", text.as_str());
            if text.as_str() == "get_grid" {
                let data_response:Vec<Stock> = STOCK_DATA.lock().unwrap().data.values().cloned().collect();
                let response = format!("grid reponse|{}", json!(data_response));
                tx.send(response);
            }
        }            
   });


    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}


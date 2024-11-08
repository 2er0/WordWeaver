mod objects;
mod dto;
mod admin_api;
mod game_api;
mod db;
mod utils;
mod ws_dto;

use axum::routing::post;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
// Our shared state
use objects::{GameState, Lobby};
use std::collections::HashMap;
use std::sync::RwLock;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

type SharedAppState = Arc<RwLock<HashMap<String, RwLock<Lobby>>>>;


#[derive(OpenApi)]
#[openapi(paths(
    crate::admin_api::new_game_handler,
    crate::admin_api::available_games_handler,
    crate::admin_api::start_game_handler,
    crate::admin_api::active_games_handler,
    crate::admin_api::close_game_handler,
    crate::game_api::join_game_handler,
    crate::game_api::claim_gap_handler,
    crate::game_api::fill_gap_handler,
    crate::game_api::guess_gap_handler,
))]
pub struct ApiDoc;


#[tokio::main]
async fn main() {
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();
    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    // Set up application state for use with with_state().
    // let user_set = Mutex::new(HashSet::new());
    // let (tx, _rx) = broadcast::channel(100);

    // Create a shared state for the application that can hold multiple GamesStates.
    let app_state: SharedAppState = Arc::new(HashMap::new());

    // TODO add auth guard
    let admin_routes = Router::new()
        .route("/new", post(admin_api::new_game_handler))
        .route("/available", get(admin_api::available_games_handler))
        .route("/start", post(admin_api::start_game_handler))
        .route("/active", get(admin_api::active_games_handler))
        .route("/close", post(admin_api::close_game_handler))
        .with_state(app_state.clone());

    // game routes
    let game_routes = Router::new()
        .route("/join", post(game_api::join_game_handler))
        .route("/claim", post(game_api::claim_gap_handler))
        .route("/fill", post(game_api::fill_gap_handler))
        .route("/guess", post(game_api::guess_gap_handler))
        .with_state(app_state.clone());

    let websocket_routes = Router::new()
        .route("/com", get(websocket_handler))
        .with_state(app_state.clone());

    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    let app = Router::new()
        .route("/", get(index))
        .nest("/api/admin", admin_routes)
        .nest("/api/{game_id}", game_routes)
        .nest("/websocket/{game_id}", websocket_routes)
        .merge(swagger_ui)
        .layer(TraceLayer::new_for_http()
                   .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                   .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedAppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
async fn websocket(stream: WebSocket, state: SharedAppState) {
    // By splitting, we can send and receive at the same time.
    // let (mut sender, mut receiver) = stream.split();
    //
    // // Username gets set in the receive loop, if it's valid.
    // let mut username = String::new();
    // // Loop until a text message is found.
    // while let Some(Ok(message)) = receiver.next().await {
    //     if let Message::Text(name) = message {
    //         // If username that is sent by client is not taken, fill username string.
    //         check_username(&state, &mut username, &name);
    //
    //         // If not empty we want to quit the loop else we want to quit function.
    //         if !username.is_empty() {
    //             break;
    //         } else {
    //             // Only send our client that username is taken.
    //             let _ = sender
    //                 .send(Message::Text(String::from("Username already taken.")))
    //                 .await;
    //
    //             return;
    //         }
    //     }
    // }
    //
    // // We subscribe *before* sending the "joined" message, so that we will also
    // // display it to our client.
    // let mut rx = state.tx.subscribe();
    //
    // // Now send the "joined" message to all subscribers.
    // let msg = format!("{username} joined.");
    // tracing::debug!("{msg}");
    // let _ = state.tx.send(msg);
    //
    // // Spawn the first task that will receive broadcast messages and send text
    // // messages over the websocket to our client.
    // let mut send_task = tokio::spawn(async move {
    //     while let Ok(msg) = rx.recv().await {
    //         // In any websocket error, break loop.
    //         if sender.send(Message::Text(msg)).await.is_err() {
    //             break;
    //         }
    //     }
    // });
    //
    // // Clone things we want to pass (move) to the receiving task.
    // let tx = state.tx.clone();
    // let name = username.clone();
    //
    // // Spawn a task that takes messages from the websocket, prepends the user
    // // name, and sends them to all broadcast subscribers.
    // let mut recv_task = tokio::spawn(async move {
    //     while let Some(Ok(Message::Text(text))) = receiver.next().await {
    //         // Add username before message.
    //         let _ = tx.send(format!("{name}: {text}"));
    //     }
    // });
    //
    // // If any one of the tasks run to completion, we abort the other.
    // tokio::select! {
    //     _ = &mut send_task => recv_task.abort(),
    //     _ = &mut recv_task => send_task.abort(),
    // };
    //
    // // Send "user left" message (similar to "joined" above).
    // let msg = format!("{username} left.");
    // tracing::debug!("{msg}");
    // let _ = state.tx.send(msg);
    //
    // // Remove username from map so new clients can take it again.
    // state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &Lobby, string: &mut String, name: &str) {
    // let mut user_set = state.user_set.lock().unwrap();
    //
    // if !user_set.contains(name) {
    //     user_set.insert(name.to_owned());
    //
    //     string.push_str(name);
    // }
}

// Include utf-8 file at **compile** time.
async fn index() -> Html<&'static str> {
    Html("WordWeaver")
}
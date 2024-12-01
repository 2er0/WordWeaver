mod admin_api;
mod db;
mod dto;
mod game_api;
mod objects;
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
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
// Our shared state
use crate::ws_dto::WSAuthMessage;
use axum::extract::Path;
use axum::http::{Method, StatusCode};
use axum::serve::Serve;
use axum_extra::routing;
use objects::{GameState, Lobby};
use serde_json::{from_str, json};
use std::collections::HashMap;
use std::sync::RwLock;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::http::header;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tracing::{event, Level};
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
    crate::admin_api::start_fill_handler,
    crate::game_api::hello_handler,
    crate::game_api::join_game_handler,
    crate::game_api::claim_gap_handler,
    crate::game_api::fill_gap_handler,
    crate::game_api::filled_gaps_handler,
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
    let app_state: SharedAppState = Arc::new(RwLock::new(HashMap::new()));

    // TODO add auth guard
    let admin_routes = Router::new()
        .route("/new", post(admin_api::new_game_handler))
        .route("/available", get(admin_api::available_games_handler))
        .route("/start", post(admin_api::start_game_handler))
        .route("/active", get(admin_api::active_games_handler))
        .route("/close", post(admin_api::close_game_handler))
        .route("/startfill", post(admin_api::start_fill_handler))
        .with_state(app_state.clone());

    // game routes
    let game_routes = Router::new()
        .route("/hello", get(game_api::hello_handler))
        .route("/join", post(game_api::join_game_handler))
        .route("/rejoin", post(game_api::rejoin_game_handler))
        .route("/claim", post(game_api::claim_gap_handler))
        .route("/fill", post(game_api::fill_gap_handler))
        .route("/filled", get(game_api::filled_gaps_handler))
        .route("/guess", post(game_api::guess_gap_handler))
        .with_state(app_state.clone());

    let websocket_routes = Router::new()
        .route("/com", get(websocket_handler))
        .with_state(app_state.clone());

    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let serve_dir = ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));
    let app = Router::new()
        //.route("/", Serve(dir!("assets/index.html")))
        .route(
            "/",
            axum::routing::get_service(ServeDir::new("assets")).handle_error(|_| async {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }),
        )
        .nest("/api/admin", admin_routes)
        .nest("/api/:game_id", game_routes)
        .nest("/websocket/:game_id", websocket_routes)
        .merge(swagger_ui)
        .nest_service(
            "/assets",
            axum::routing::get_service(ServeDir::new("assets/assets")),
        )
        .fallback_service(serve_dir)
        // .fallback_service(
        //     axum::routing::get_service(ServeDir::new("assets")).handle_error(|_| async {
        //         (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
        //     }),
        // )
        .layer(cors_layer)
        .layer(
            TraceLayer::new_for_http()
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
    game_id: Path<String>,
    state: State<SharedAppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, game_id, state))
}

// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
async fn websocket(
    stream: WebSocket,
    Path(game_id): Path<String>,
    State(state): State<SharedAppState>,
) {
    // check if the game exists
    if !state.read().unwrap().contains_key(&game_id) {
        return;
    }
    // By splitting, we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    // Subscribe to the broadcast channel for the game
    let mut rx = {
        let state = state.read().unwrap();
        let lobby = state.get(&game_id).unwrap();
        let read_lobby = lobby.read().unwrap();
        read_lobby.game.tx.subscribe()
    };

    // Spawn a task to send messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Spawn a task to receive messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(_) => break,
            };
            let msg = match msg {
                Message::Text(msg) => msg,
                _ => continue,
            };

            let auth_msg = from_str::<WSAuthMessage>(&msg);
            if auth_msg.is_err() {
                event!(Level::ERROR, "{}", auth_msg.unwrap_err());
                break;
            }
            let auth_msg = auth_msg.unwrap();
            if auth_msg.obj != "auth" {
                event!(Level::ERROR, "Expected auth message, got {}", auth_msg.obj);
                break;
            }
            // check if the user is in the game
            if state
                .read()
                .unwrap()
                .get(&game_id)
                .unwrap()
                .read()
                .unwrap()
                .users
                .read()
                .unwrap()
                .iter()
                .filter(|u| u.token == auth_msg.token)
                .count()
                == 1
            {
                event!(Level::INFO, "User {} joined WScom", auth_msg.token);
            } else {
                event!(Level::ERROR, "User not in game");
                break;
            }
        }
    });

    // If any one of the tasks run to completion, we abort the other.
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

// Include utf-8 file at **compile** time.
async fn index() -> Html<&'static str> {
    Html("WordWeaver")
}

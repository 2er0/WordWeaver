mod admin_api;
mod db;
mod dto;
mod game_api;
mod objects;
mod utils;
mod websocket;
mod ws_dto;

// Our shared state
use crate::admin_api::auth_check;
use crate::objects::SecurityAddon;
use crate::websocket::websocket_handler;
use axum::http::{Method, StatusCode};
use axum::routing::post;
use axum::{middleware, routing::get, Router};
use objects::Lobby;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tokio_tungstenite::tungstenite::http::header;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

type SharedAppState = Arc<RwLock<HashMap<String, RwLock<Lobby>>>>;

#[derive(OpenApi)]
#[openapi(
    paths(
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
    ),
    modifiers(&SecurityAddon)
)]
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

    // Create a shared state for the application that can hold multiple GamesStates.
    let app_state: SharedAppState = Arc::new(RwLock::new(HashMap::new()));

    let admin_routes = Router::new()
        .route("/new", post(admin_api::new_game_handler))
        .route("/available", get(admin_api::available_games_handler))
        .route("/start", post(admin_api::start_game_handler))
        .route("/active", get(admin_api::active_games_handler))
        .route("/close", post(admin_api::close_game_handler))
        .route("/startfill", post(admin_api::start_fill_handler))
        .layer(middleware::from_fn(auth_check))
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

    // websocket routes
    let websocket_routes = Router::new()
        .route("/com", get(websocket_handler))
        .with_state(app_state.clone());

    // Swagger UI
    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    // CORS
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    // Serve static files and main routing
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
        .layer(cors_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

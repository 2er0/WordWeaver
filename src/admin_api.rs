use crate::db::DB;
use crate::dto::{BaseResponse, BaseStringDTO, GameDBDTO, GameDTO, Override};
use crate::objects::Lobby;
use crate::utils::id_generator;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::to_string;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use crate::ws_dto::WSMessage;

type SharedAppState = Arc<RwLock<HashMap<String, RwLock<Lobby>>>>;

// Url: /api/admin/new
// Saves a new game to the surrealdb
// Method:OST
// Request: GameDTO{id: String, gaps: Vec<String>}
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/admin/new",
    params(
        ("force" = Option<bool>, Query, description = "Force update if game already exists")
    ),
    request_body = GameDTO,
    responses(
        (status = 200, description = "Game saved", body = BaseResponse),
        (status = 409, description = "Game already exists", body = BaseResponse),
        (status = 500, description = "Failed to check if game exists", body = BaseResponse),
        (status = 500, description = "Failed to delete game to update existing game", body = BaseResponse),
        (status = 500, description = "Failed to insert game", body = BaseResponse)
    )
)]
pub async fn new_game_handler(State(state): State<SharedAppState>,
                              Query(force): Query<Override>,
                              Json(payload): Json<GameDTO>)
                              -> impl IntoResponse {
    // Save the new game to the SurrealDB here
    let con: &Surreal<Client> = DB.get().await;

    let exists: surrealdb::Result<Option<GameDBDTO>> = con.select(("game", payload.name.clone())).await;

    if exists.is_ok() && exists.as_ref().unwrap().is_some() {
        if force.force.unwrap_or(false) {
            let delete_response: surrealdb::Result<Option<GameDBDTO>> = con.delete(("game", payload.name.clone())).await;
            if delete_response.is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR,
                        Json(BaseResponse {
                            success: false,
                            message: Some("Failed to delete game to update existing game".to_string()),
                        }));
            }
        } else {
            return (StatusCode::CONFLICT,
                    Json(BaseResponse {
                        success: false,
                        message: Some("Game already exists".to_string()),
                    }));
        }
    } else {
        if exists.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR,
                    Json(BaseResponse {
                        success: false,
                        message: Some("Failed to check if game exists".to_string()),
                    }));
        }
    }

    let insert_response: surrealdb::Result<Option<GameDBDTO>> =
        con.insert(("game", payload.name.clone())).content(payload)
            .await;
    if insert_response.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to insert game".to_string()),
                }));
    }
    (StatusCode::OK,
     Json(BaseResponse { success: true, message: Some("Game saved".to_string()) }))
}

// Url: /api/admin/available
// Returns the available games
// Method: GET
// Response: Vec<Games{id: String, gaps: Vec<String>}>
#[utoipa::path(
    get,
    path = "/api/admin/available",
    responses(
        (status = 200, description = "Available games retrieved successfully", body = [GameDTO]),
        (status = 500, description = "Failed to get games", body = BaseResponse)
    )
)]
pub async fn available_games_handler(State(state): State<SharedAppState>)
                                     -> impl IntoResponse {
    // Get the available games from the SurrealDB here
    let con: &Surreal<Client> = DB.get().await;
    let games: surrealdb::Result<Vec<GameDTO>> = con.select("game").await;
    if games.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to get games".to_string()),
                }).into_response());
    }
    (StatusCode::OK,
     Json(games.unwrap().iter().map(|game| GameDTO {
         name: game.name.clone(),
         text_section: game.text_section.clone(),
     }).collect::<Vec<GameDTO>>()).into_response())
}

// Url: /api/admin/start
// Starts a game with the specified id and loads it from db to a temporary game state in memory and
// create a random short id for the game
// Method: POST
// Request: BaseStringDTO{id: String}
// Response: BaseStringDTO{id: String}
#[utoipa::path(
    post,
    path = "/api/admin/start",
    request_body = BaseStringDTO,
    responses(
        (status = 200, description = "Game started", body = BaseStringDTO),
        (status = 404, description = "No game found", body = BaseResponse)
    )
)]
pub async fn start_game_handler(State(state): State<SharedAppState>,
                                Json(payload): Json<BaseStringDTO>)
                                -> impl IntoResponse {
    // Load the game from the SurrealDB and create a temporary game state here
    let con: &Surreal<Client> = DB.get().await;
    let game_optional: surrealdb::Result<Option<GameDBDTO>> = con.select(("game", payload.name.clone())).await;
    if game_optional.is_err() || game_optional.as_ref().unwrap().is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("No game found".to_string()),
                }).into_response());
    }

    let game = game_optional.unwrap().unwrap();
    // Create a random short id for the game here
    let id = id_generator(6);
    let lobby = Lobby::new(id.clone(), game.text_section.clone());

    let mut state = state.write().unwrap();
    state.insert(id.clone(), RwLock::new(lobby));

    // TODO generate qr code to join the game

    (StatusCode::OK,
     Json(BaseStringDTO { name: id }).into_response())
}

// Url: /api/admin/active
// Returns the active games
// Method: GET
// Response: Vec<BaseStringDTO{id: String}>
#[utoipa::path(
    get,
    path = "/api/admin/active",
    responses(
        (status = 200, description = "Active games retrieved successfully", body = [BaseStringDTO]),
        (status = 500, description = "Failed to get active games", body = BaseResponse)
    )
)]
pub async fn active_games_handler(State(state): State<SharedAppState>)
                                  -> impl IntoResponse {
    // Get the active games from the temporary game state here
    let exclusive_state = state.read().unwrap();
    let active_games = exclusive_state
        .keys()
        .map(|id| BaseStringDTO { name: id.clone() })
        .collect::<Vec<BaseStringDTO>>();
    (StatusCode::OK,
     Json(active_games).into_response())
}

// Url: /api/admin/close
// Closes a game with the specified id
// Method: POST
// Request: BaseStringDTO{id: String}
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/admin/close",
    request_body = BaseStringDTO,
    responses(
        (status = 200, description = "Game closed", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse)
    )
)]
pub async fn close_game_handler(state: State<SharedAppState>,
                                payload: Json<BaseStringDTO>)
                                -> impl IntoResponse {
    // Close the game with the specified id here
    let mut exclusive_state = state.write().unwrap();
    let option = exclusive_state.remove(&payload.name);
    if option.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    (StatusCode::OK, Json(BaseResponse { success: true, message: None }).into_response())
}

// Url /api/admin/startfill
// Starts the filling process for the specified gap
// Method: POST
// Request: BaseStringDTO{id: String}
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/admin/startfill",
    request_body = BaseStringDTO,
    responses(
        (status = 200, description = "Filling started", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse)
    )
)]
pub async fn start_fill_handler(state: State<SharedAppState>,
                                payload: Json<BaseStringDTO>)
                                -> impl IntoResponse {
    // Start the filling process for the specified gap here
    let exclusive_state = state.read().unwrap();
    let lobby = exclusive_state.get(&payload.name);
    if lobby.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    let mut lobby = lobby.unwrap().write().unwrap();
    lobby.game.view = "fill".to_string();
    // notify all users that the filling process has started
    let _ = lobby.game.tx.send(
        to_string(&WSMessage::change_view("fill".to_string())).unwrap());
    (StatusCode::OK, Json(BaseResponse { success: true, message: None }).into_response())
}
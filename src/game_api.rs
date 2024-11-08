use crate::dto::{BaseResponse, GapClaimDTO, GapFillDTO, GuessesDTO, JoinResponse, PreGapTextDTO, UserDTO};
use crate::objects::{Guess, Lobby, User};
use crate::ws_dto::UserJoined;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, to_string};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

type SharedAppState = Arc<RwLock<HashMap<String, RwLock<Lobby>>>>;

// Url: /api/{game_id}/join
// User joins the game with the specified id
// Method: POST
// Request: UserDTO{username: String, token: String}
// Response: JoinResponse
#[utoipa::path(
    post,
    path = "/api/{game_id}/join",
    request_body = UserDTO,
    responses(
        (status = 200, description = "User joined successfully", body = JoinResponse),
        (status = 404, description = "Game not found", body = BaseResponse),
        (status = 500, description = "Internal server error", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    )
)]
pub async fn join_game_handler(State(state): State<SharedAppState>,
                               Path(game_id): Path<String>,
                               Json(payload): Json<UserDTO>)
                               -> impl IntoResponse {
    // Add user to the game with the specified id here
    let read_state = state.read().unwrap();
    let opt_lobby = read_state.get(&game_id.to_string());
    if opt_lobby.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    let user = User {
        username: payload.username.clone(),
        token: payload.token.clone(),
        correct_guesses: 0,
    };
    let lobby = opt_lobby.unwrap().read().unwrap();
    lobby.users.write().unwrap().push(user);
    let send_status = lobby
        .game.tx.send(to_string(&UserJoined::new(payload.username.clone())).unwrap());
    if send_status.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to send user joined message".to_string()),
                }).into_response());
    }

    let pre_gap_text = lobby.game.gaps.iter()
        .map(|g| {
            let g_read = g.read().unwrap();
            PreGapTextDTO {
                id: g_read.id,
                text: g_read.text_section.clone(),
                gap_after: g_read.gap_after,
            }
        }).collect();

    (StatusCode::OK, Json(JoinResponse { success: true, pre_gaps_text: pre_gap_text }).into_response())
}

// Url: /api/{game_id}/claim
// User claims a gap in the game with the specified id
// Method: POST
// Request: GapClaimDTO{gap_id: u32, username: String}
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/{game_id}/claim",
    request_body = GapClaimDTO,
    responses(
        (status = 200, description = "Gap claimed successfully", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse),
        (status = 400, description = "Gap already claimed", body = BaseResponse),
        (status = 500, description = "Internal server error", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    )
)]
pub async fn claim_gap_handler(State(state): State<SharedAppState>,
                               Path(game_id): Path<String>,
                               Json(payload): Json<GapClaimDTO>)
                               -> impl IntoResponse {
    // Claim the gap in the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    let lobby = read_lobby.unwrap().read().unwrap();
    // claiming write lock on the gaps
    let gaps = &lobby.game.gaps;
    let write_gap = gaps.get(payload.gap_id as usize).unwrap().write();
    if write_gap.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to claim gap".to_string()),
                }).into_response());
    }
    let mut gap_to_claim = write_gap.unwrap();
    if gap_to_claim.filled_by.is_some() {
        return (StatusCode::BAD_REQUEST,
                Json(BaseResponse {
                    success: false,
                    message: Some("Gap already claimed".to_string()),
                }).into_response());
    }
    gap_to_claim.filled_by = Some(payload.token.clone());
    // notify all users about the claimed gap
    // todo define the message object
    let send_status = lobby.game.tx.send(json!({
        "gap_id": payload.gap_id,
    }).to_string());
    (StatusCode::OK, Json(BaseResponse { success: true, message: None }).into_response())
}

// Url: /api/{game_id}/fill
// User fills a gap in the game with the specified id
// Method: POST
// Request: GapFillDTO{gap_id: u32, username: String, content: String}
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/{game_id}/fill",
    request_body = GapFillDTO,
    responses(
        (status = 200, description = "Gap filled successfully", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse),
        (status = 400, description = "Gap not claimed or claimed by another user", body = BaseResponse),
        (status = 500, description = "Internal server error", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    )
)]
pub async fn fill_gap_handler(State(state): State<SharedAppState>,
                              Path(game_id): Path<String>,
                              Json(payload): Json<GapFillDTO>)
                              -> impl IntoResponse {
    // Fill the gap in the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    let write_gap = read_lobby.unwrap().read()
        .unwrap().game.gaps.get(payload.gap_id as usize).unwrap().write();
    if write_gap.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to fill gap".to_string()),
                }).into_response());
    }
    let mut gap_to_fill = write_gap.unwrap();
    if gap_to_fill.filled_by.is_none() {
        return (StatusCode::BAD_REQUEST,
                Json(BaseResponse {
                    success: false,
                    message: Some("Gap not claimed".to_string()),
                }).into_response());
    }
    if gap_to_fill.filled_by.as_ref().unwrap() != &payload.token {
        return (StatusCode::BAD_REQUEST,
                Json(BaseResponse {
                    success: false,
                    message: Some("Gap claimed by another user".to_string()),
                }).into_response());
    }
    gap_to_fill.value = payload.content.clone();
    // notify all users about the filled gap
    let send_status = read_lobby.unwrap().read().unwrap()
        // todo define the message object
        .game.tx.send(json!({
            "gap_id": payload.gap_id,
        }).to_string());
    (StatusCode::OK, Json(BaseResponse { success: true, message: None }).into_response())
}

// Url: /api/{game_id}/guess
// User submits guesses about which gap is filled by which user
// Method: POST
// Request: Vec<Guess{gap_id: u32, token: String}>
// Response: BaseResponse
#[utoipa::path(
    post,
    path = "/api/{game_id}/guess",
    request_body = GuessesDTO,
    responses(
        (status = 200, description = "Guesses submitted successfully", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse),
        (status = 500, description = "Internal server error", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    )
)]
pub async fn guess_gap_handler(State(state): State<SharedAppState>,
                               Path(game_id): Path<String>,
                               Json(payload): Json<GuessesDTO>)
                               -> impl IntoResponse {
    // Process the guesses for the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("Game not found".to_string()),
                }).into_response());
    }
    let read_lobby = read_lobby.unwrap().read().unwrap();
    read_lobby.game.guesses.write().unwrap().push(
        payload.guesses.iter()
            .map(|g| Guess {
                gap_id: g.gap_id,
                guess: g.token.clone(),
                guesser: payload.token.clone(),
            }).collect());
    // notify all users about the guesses
    // todo define the message object
    let send_status = read_lobby.game.tx.send(json!({
        "guessed": payload.token.clone(),
    }).to_string());
    (StatusCode::OK, Json(BaseResponse { success: true, message: None }).into_response())
}
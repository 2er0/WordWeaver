use crate::dto::{
    BaseResponse, CurrentGapTextDTO, EndGameResponse, GapClaimDTO, GapFillDTO, GapFilledDTO,
    GuessesDTO, JoinResponse, PreGapTextDTO, PreGuessingDTO, RejoinResponseDTO, TokenQuery,
    UserDTO,
};
use crate::objects::{Lobby, User};
use crate::ws_dto::{GuessScore, TempUser, WSMessage};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::to_string;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{event, Level};

type SharedAppState = Arc<RwLock<HashMap<String, RwLock<Lobby>>>>;

// Url: /api/{game_id}/hello
// Method: GET
// Response: BaseResponse
#[utoipa::path(
    get,
    path = "/api/{game_id}/hello",
    responses(
        (status = 200, description = "Game found", body = BaseResponse),
        (status = 404, description = "Game not found", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    ),
    description = "Hello endpoint"
)]
pub async fn hello_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
) -> impl IntoResponse {
    // Check if the game with the specified id is active here
    let read_state = state.read().unwrap();
    let opt_lobby = read_state.get(&game_id.to_string());
    if opt_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    (
        StatusCode::OK,
        Json(BaseResponse {
            success: true,
            message: Some("Game found".to_string()),
        })
        .into_response(),
    )
}

// Url: /api/game/{game_id}/join
// User joins the game with the specified id
// Method: POST
// Request: UserDTO{name: String, token: String}
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
    ),
    description = "Join the game with the specified id and notifies all users about \
    the new user via websocket with the message \
    {\"type\": \"user_joined\", \"token\": \"token\"}"
)]
pub async fn join_game_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Json(payload): Json<UserDTO>,
) -> impl IntoResponse {
    // Add user to the game with the specified id here
    let read_state = state.read().unwrap();
    let opt_lobby = read_state.get(&game_id.to_string());
    if opt_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    if opt_lobby.unwrap().read().unwrap().game.view != "waiting" {
        return (
            StatusCode::BAD_REQUEST,
            Json(BaseResponse {
                success: false,
                message: Some("Game can't be joined anymore.".to_string()),
            })
            .into_response(),
        );
    }
    let user = User {
        name: payload.name.clone(),
        token: payload.token.clone(),
        correct_guesses: 0,
        guessed: false,
    };
    let lobby = opt_lobby.unwrap().read().unwrap();
    lobby.users.write().unwrap().push(user);
    if lobby.game.tx.receiver_count() > 0 {
        let send_status = lobby.game.tx.send(
            to_string(&WSMessage::user_joined(
                payload.name.clone(),
                payload.token.clone(),
            ))
            .unwrap(),
        );
        if send_status.is_err() {
            event!(Level::ERROR, "{}", send_status.unwrap_err());
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to send user joined message".to_string()),
                })
                .into_response(),
            );
        }
    }

    let pre_gap_text = lobby
        .game
        .gaps
        .iter()
        .map(|g| {
            let g_read = g.read().unwrap();
            PreGapTextDTO {
                id: g_read.id,
                text: g_read.text_section.clone(),
                gap_after: g_read.gap_after,
            }
        })
        .collect();

    let current_users = lobby
        .users
        .read()
        .unwrap()
        .iter()
        .filter(|u| u.token != payload.token)
        .map(|u| UserDTO {
            name: u.name.clone(),
            token: u.token.clone(),
        })
        .collect();

    (
        StatusCode::OK,
        Json(JoinResponse {
            success: true,
            pre_gaps_text: pre_gap_text,
            current_users,
        })
        .into_response(),
    )
}

// Url: /api/{game_id}/claim
// User claims a gap in the game with the specified id
// Method: POST
// Request: GapClaimDTO{gap_id: u32, name: String}
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
    ),
    description = "Claim the gap in the game with the specified id and notifies all users about \
    the claimed gap via websocket with the message \
    {\"type\": \"gap_claimed\", \"gap_id\": gap_id}"
)]
pub async fn claim_gap_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Json(payload): Json<GapClaimDTO>,
) -> impl IntoResponse {
    // Claim the gap in the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    let lobby = read_lobby.unwrap().read().unwrap();
    // claiming write lock on the gaps
    let gaps = &lobby.game.gaps;
    let write_gap = gaps.get(payload.gap_id as usize).unwrap().write();
    if write_gap.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(BaseResponse {
                success: false,
                message: Some("Failed to claim gap".to_string()),
            })
            .into_response(),
        );
    }
    let mut gap_to_claim = write_gap.unwrap();
    if gap_to_claim.filled_by.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(BaseResponse {
                success: false,
                message: Some("Gap already claimed".to_string()),
            })
            .into_response(),
        );
    }
    gap_to_claim.filled_by = Some(payload.token.clone());
    // notify all users about the claimed gap
    let send_status = lobby
        .game
        .tx
        .send(to_string(&WSMessage::gap_claimed(payload.gap_id)).unwrap());
    (
        StatusCode::OK,
        Json(BaseResponse {
            success: true,
            message: None,
        })
        .into_response(),
    )
}

// Url: /api/{game_id}/fill
// User fills a gap in the game with the specified id
// Method: POST
// Request: GapFillDTO{gap_id: u32, name: String, content: String}
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
    ),
    description = "Fill the gap in the game with the specified id and notifies all users about \
    the filled gap via websocket with the message \
    {\"type\": \"gap_filled\", \"gap_id\": gap_id}"
)]
pub async fn fill_gap_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Json(payload): Json<GapFillDTO>,
) -> impl IntoResponse {
    // Fill the gap in the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }

    // filling the gap
    {
        let lobby = read_lobby.unwrap().read().unwrap();
        let write_gap = lobby
            .game
            .gaps
            .get(payload.gap_id as usize)
            .unwrap()
            .write();
        if write_gap.is_err() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse {
                    success: false,
                    message: Some("Failed to fill gap".to_string()),
                })
                .into_response(),
            );
        }
        let mut gap_to_fill = write_gap.unwrap();
        if gap_to_fill.filled_by.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(BaseResponse {
                    success: false,
                    message: Some("Gap not claimed".to_string()),
                })
                .into_response(),
            );
        }
        if gap_to_fill.filled_by.as_ref().unwrap() != &payload.token {
            return (
                StatusCode::BAD_REQUEST,
                Json(BaseResponse {
                    success: false,
                    message: Some("Gap claimed by another user".to_string()),
                })
                .into_response(),
            );
        }
        let mut content = payload.content.clone();
        content.truncate(140);
        gap_to_fill.value = content;
    }
    let mut all_filled = false;
    {
        let lobby = read_lobby.unwrap().read().unwrap();
        // notify all users about the filled gap
        let send_status = lobby
            .game
            .tx
            .send(to_string(&WSMessage::gap_filled(payload.gap_id)).unwrap());
        // check if all gaps are filled
        all_filled = lobby.game.gaps.iter().all(|g| {
            let g_read = g.read().unwrap();
            !g_read.gap_after || g_read.filled_by.is_some() && g_read.value.len() > 0
        });
    }
    {
        if all_filled {
            {
                read_lobby.unwrap().write().unwrap().game.view = "guess".to_string();
            }
            let send_status = read_lobby
                .unwrap()
                .read()
                .unwrap()
                .game
                .tx
                .send(to_string(&WSMessage::start_guessing(10)).unwrap());
        }
    }
    (
        StatusCode::OK,
        Json(BaseResponse {
            success: true,
            message: None,
        })
        .into_response(),
    )
}

// Url: /api/{game_id}/filled
// Get all the filled gaps in the game with the specified id
// Method: GET
// Response: PreGuessingDTO
#[utoipa::path(
    get,
    path = "/api/{game_id}/filled",
    params(
        ("token" = Option<String>, Query, description = "User token")
    ),
    responses(
        (status = 200, description = "Filled gaps retrieved successfully", body = PreGuessingDTO),
        (status = 404, description = "Game not found", body = PreGuessingDTO),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    ),
    description = "Get all the filled gaps in the game with the specified id"
)]
pub async fn filled_gaps_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> impl IntoResponse {
    // check if token is provided
    if query.token.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(BaseResponse {
                success: false,
                message: Some("Token is required".to_string()),
            })
            .into_response(),
        );
    }
    // Get all the filled gaps in the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    let lobby = read_lobby.unwrap().read().unwrap();
    // check if game is in guessing mode
    if lobby.game.view != "guess" {
        return (
            StatusCode::BAD_REQUEST,
            Json(BaseResponse {
                success: false,
                message: Some("Game is not in guessing mode".to_string()),
            })
            .into_response(),
        );
    }
    // check if is part of the game
    let token = query.token.unwrap();
    let users = &lobby.users.read().unwrap();
    let user_index = users.iter().position(|u| u.token == token);
    if user_index.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("User not found".to_string()),
            })
            .into_response(),
        );
    }
    // game found and user is part of the game
    // return the filled gaps and the users
    let filled_gaps = lobby
        .game
        .gaps
        .iter()
        .map(|g| {
            let g_read = g.read().unwrap();
            if !g_read.gap_after {
                return None;
            } else {
                return Some(GapFilledDTO {
                    gap_id: g_read.id,
                    value: g_read.value.clone(),
                });
            }
        })
        .filter(|g| g.is_some())
        .map(|g| g.unwrap())
        .collect();
    let users = users.iter().map(|u| UserDTO {
        name: u.name.clone(),
        token: u.token.clone(),
    });
    (
        StatusCode::OK,
        Json(PreGuessingDTO {
            success: true,
            gaps: filled_gaps,
            users: users.collect(),
        })
        .into_response(),
    )
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
    ),
    description = "Submit guesses about which gap is filled by which user in the game with the \
    specified id and notifies all users about the guesses via websocket with the message \
    {\"type\": \"guessed\", \"token\": \"token\"}"
)]
pub async fn guess_gap_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Json(payload): Json<GuessesDTO>,
) -> impl IntoResponse {
    // Process the guesses for the game with the specified id here
    let read_state = state.read().unwrap();
    let read_lobby = read_state.get(&game_id.to_string());
    if read_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    {
        read_lobby.unwrap().write().unwrap().game.view = "ranking".to_string();
    }
    let read_lobby = read_lobby.unwrap().read().unwrap();
    // store number of correct guesses
    let mut correct_guesses = 0;
    // process the guesses
    for guess in &payload.guesses {
        let g = read_lobby
            .game
            .gaps
            .iter()
            .find(|g| g.read().unwrap().id == guess.gap_id);
        if g.is_some() {
            let gap = g.unwrap().read().unwrap();
            if gap.gap_after
                && gap.filled_by.is_some()
                && gap.filled_by.as_ref().unwrap() == &guess.token
            {
                correct_guesses += 1;
            }
        };
    }
    // update the user's correct guesses
    {
        let mut users = read_lobby.users.write().unwrap();
        let user_index = users.iter().position(|u| u.token == payload.token);
        if user_index.is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(BaseResponse {
                    success: false,
                    message: Some("User not found".to_string()),
                })
                .into_response(),
            );
        }
        users[user_index.unwrap()].correct_guesses = correct_guesses;
        users[user_index.unwrap()].guessed = true;
    }

    // notify all users about the guesses
    if read_lobby.users.read().unwrap().iter().all(|u| u.guessed) {
        let guesses = read_lobby
            .users
            .read()
            .unwrap()
            .iter()
            .map(|u| GuessScore {
                name: u.name.clone(),
                token: u.token.clone(),
                score: u.correct_guesses,
            })
            .collect();
        let send_status = read_lobby
            .game
            .tx
            .send(to_string(&WSMessage::guess_scores(guesses)).unwrap());
    }
    (
        StatusCode::OK,
        Json(BaseResponse {
            success: true,
            message: None,
        })
        .into_response(),
    )
}

// Url: /api/{game_id}/rejoin
// User rejoins the game with the specified id
// Method: POST
// Request: UserDTO{name: String, token: String
// Response: JoinResponse
#[utoipa::path(
    post,
    path = "/api/{game_id}/rejoin",
    request_body = UserDTO,
    responses(
        (status = 200, description = "User rejoined successfully", body = RejoinResponseDTO),
        (status = 404, description = "Game not found | User not found", body = BaseResponse),
        (status = 500, description = "Internal server error", body = BaseResponse),
    ),
    params(
        ("game_id" = String, Path, description = "ID of the game")
    ),
    description = "Rejoin the game with the specified id and user"
)]
pub async fn rejoin_game_handler(
    State(state): State<SharedAppState>,
    Path(game_id): Path<String>,
    Json(payload): Json<UserDTO>,
) -> impl IntoResponse {
    // Rejoin the game with the specified id here
    let read_state = state.read().unwrap();
    let opt_lobby = read_state.get(&game_id.to_string());
    if opt_lobby.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("Game not found".to_string()),
            })
            .into_response(),
        );
    }
    let lobby = opt_lobby.unwrap().read().unwrap();
    if lobby.game.view == "ranking" {
        // game has ended
        return (
            StatusCode::OK,
            Json(EndGameResponse {
                success: true,
                view: "ranking".to_string(),
                value: lobby
                    .users
                    .read()
                    .unwrap()
                    .iter()
                    .map(|u| GuessScore {
                        name: u.name.clone(),
                        token: u.token.clone(),
                        score: u.correct_guesses,
                    })
                    .collect(),
            })
            .into_response(),
        );
    }

    let users = &lobby.users.read().unwrap();
    let user_index = users
        .iter()
        .position(|u| u.token == payload.token && u.name == payload.name);
    if user_index.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(BaseResponse {
                success: false,
                message: Some("User not found".to_string()),
            })
            .into_response(),
        );
    }

    let share_fillings = lobby.game.view != "fill";
    let pre_gap_text = lobby
        .game
        .gaps
        .iter()
        .map(|g| {
            let g_read = g.read().unwrap();
            let filled_by_current_user = g_read
                .filled_by
                .as_ref()
                .is_some_and(|u| u == &payload.token);
            let gap_value = if share_fillings {
                Some(g_read.value.clone())
            } else {
                None
            };
            CurrentGapTextDTO {
                id: g_read.id,
                text: g_read.text_section.clone(),
                gap_after: g_read.gap_after,
                claimed: g_read.filled_by.is_some(),
                filled: !g_read.value.is_empty(),
                gap_value,
                filled_by_current_user,
            }
        })
        .collect();

    (
        StatusCode::OK,
        Json(RejoinResponseDTO {
            success: true,
            current_gap_text: pre_gap_text,
            view: lobby.game.view.clone(),
            users: users
                .iter()
                //.filter(|u| u.token != payload.token)
                .map(|u| UserDTO {
                    name: u.name.clone(),
                    token: u.token.clone(),
                })
                .collect(),
        })
        .into_response(),
    )
}

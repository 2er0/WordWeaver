use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde_json::from_str;
use tracing::{event, Level};
use crate::SharedAppState;
use crate::ws_dto::WSAuthMessage;

pub async fn websocket_handler(
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

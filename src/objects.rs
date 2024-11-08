use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tokio::sync::broadcast;

pub struct User {
    pub username: String,
    pub token: String,
    pub correct_guesses: u32,
}

pub struct Gap {
    pub id: u32,
    pub text_section: String,
    pub gap_after: bool,
    pub value: String,
    pub filled_by: Option<String>,  // user token
}


#[derive(Serialize, Deserialize)]
pub struct Guess {
    pub gap_id: u32,
    pub guess: String,  // user token
    pub guesser: String,  // user token
}

pub struct GameState {
    // Channel used to send messages to all connected clients.
    pub tx: broadcast::Sender<String>,
    pub gaps: Vec<RwLock<Gap>>,
    pub guesses: RwLock<Vec<Vec<Guess>>>,
}

pub struct Lobby {
    pub id: String,
    pub users: RwLock<Vec<User>>,
    pub game: GameState,
    pub finished: bool,
}

impl Lobby {
    pub fn new(id: String, gaps: Vec<String>) -> Self {
        // Create a new game state with the specified gaps
        // The last gap should not have a gap after it
        let gaps: Vec<RwLock<Gap>> = gaps.iter()
            .map(|g|
                RwLock::new(Gap {
                    id: 0,
                    text_section: g.clone(),
                    gap_after: true,
                    value: "".to_string(),
                    filled_by: None,
                }))
            .collect();
        gaps.last().unwrap().write().unwrap().gap_after = false;
        let game_state = GameState {
            tx: broadcast::channel(10).0,
            gaps,
            guesses: RwLock::new(vec![]),
        };
        // Create a new lobby with the specified id and game state
        Lobby {
            id,
            users: RwLock::new(vec![]),
            game: game_state,
            finished: false,
        }
    }
}



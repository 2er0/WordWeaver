use std::sync::RwLock;
use tokio::sync::broadcast;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify,
};

pub struct User {
    pub name: String,
    pub token: String,
    pub correct_guesses: u32,
    pub guessed: bool,
}

pub struct Gap {
    pub id: u32,
    pub text_section: String,
    pub gap_after: bool,
    pub value: String,
    pub filled_by: Option<String>, // user token
}

pub struct GameState {
    // Channel used to send messages to all connected clients.
    pub tx: broadcast::Sender<String>,
    pub gaps: Vec<RwLock<Gap>>,
    pub view: String,
}

pub struct Lobby {
    pub users: RwLock<Vec<User>>,
    pub game: GameState,
    pub finished: bool,
}

impl Lobby {
    pub fn new(gaps: Vec<String>) -> Self {
        // Create a new game state with the specified gaps
        // The last gap should not have a gap after it
        let gaps: Vec<RwLock<Gap>> = gaps
            .iter()
            .enumerate()
            .map(|g| {
                RwLock::new(Gap {
                    id: g.0 as u32,
                    text_section: g.1.clone(),
                    gap_after: true,
                    value: "".to_string(),
                    filled_by: None,
                })
            })
            .collect();
        gaps.last().unwrap().write().unwrap().gap_after = false;
        let game_state = GameState {
            tx: broadcast::channel(100).0,
            gaps,
            view: "waiting".to_string(),
        };
        // Create a new lobby with the specified id and game state
        Lobby {
            users: RwLock::new(vec![]),
            game: game_state,
            finished: false,
        }
    }
}

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "ApiKey",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("apikey"))),
        )
    }
}

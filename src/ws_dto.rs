use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WSMessage<T> {
    pub obj: String,
    pub value: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TempUser {
    pub name: String,
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct WSAuthMessage {
    pub obj: String,
    pub token: String,
}

impl WSMessage<TempUser> {
    pub fn user_joined(name: String, token: String) -> Self {
        WSMessage {
            obj: "user_joined".to_string(),
            value: TempUser { name, token },
        }
    }
}
impl<String> WSMessage<String> {
    pub fn guessed(token: String) -> Self {
        WSMessage {
            obj: "guessed".to_string(),
            value: token,
        }
    }

    pub fn change_view(view: String) -> Self {
        WSMessage {
            obj: "change_view".to_string(),
            value: view,
        }
    }
}

impl<U32> WSMessage<U32> {
    pub fn gap_claimed(gap_id: U32) -> Self {
        WSMessage {
            obj: "gap_claimed".to_string(),
            value: gap_id,
        }
    }

    pub fn gap_filled(gap_id: U32) -> Self {
        WSMessage {
            obj: "gap_filled".to_string(),
            value: gap_id,
        }
    }

    pub fn start_guessing(delay: U32) -> Self {
        WSMessage {
            obj: "start_guessing".to_string(),
            value: delay,
        }
    }
}
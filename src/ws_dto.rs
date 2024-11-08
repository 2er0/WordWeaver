use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UserJoined {
    pub obj: String,
    pub username: String,
}

impl UserJoined {
    pub fn new(username: String) -> Self {
        UserJoined {
            obj: "user_joined".to_string(),
            username,
        }
    }
}
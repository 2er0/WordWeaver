use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use utoipa::ToSchema;
use crate::ws_dto::GuessScore;

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct GameDTO {
    pub name: String,
    pub text_section: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct  Override {
    pub force: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GameDBDTO {
    pub id: RecordId,
    pub name: String,
    pub text_section: Vec<String>,
}


#[derive(Serialize, Deserialize, ToSchema)]
pub struct BaseResponse {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct EndGameResponse {
    pub success: bool,
    pub view: String,
    pub value: Vec<GuessScore>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct BaseStringDTO {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserDTO {
    pub name: String,
    pub token: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PreGapTextDTO {
    pub id: u32,
    pub text: String,
    pub gap_after: bool,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct JoinResponse {
    pub success: bool,
    pub pre_gaps_text: Vec<PreGapTextDTO>,
    pub current_users: Vec<UserDTO>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct CurrentGapTextDTO {
    pub id: u32,
    pub text: String,
    pub gap_after: bool,
    pub claimed: bool,
    pub filled: bool,
    pub gap_value: Option<String>,
    pub filled_by_current_user: bool
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct RejoinResponseDTO {
    pub success: bool,
    pub current_gap_text: Vec<CurrentGapTextDTO>,
    pub view: String,
    pub users: Vec<UserDTO>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GapClaimDTO {
    pub gap_id: u32,
    pub token: String,  // user token
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GapFillDTO {
    pub gap_id: u32,
    pub token: String, // user token
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct TokenQuery {
    pub token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GapFilledDTO {
    pub gap_id: u32,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct PreGuessingDTO {
    pub success: bool,
    pub gaps: Vec<GapFilledDTO>,
    pub users: Vec<UserDTO>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GuessDTO {
    pub gap_id: u32,
    pub token: String, // guessed user token
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GuessesDTO {
    pub token: String, // guesser user token
    pub guesses: Vec<GuessDTO>,
}

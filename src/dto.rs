use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use utoipa::ToSchema;

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

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct BaseStringDTO {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserDTO {
    pub username: String,
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
pub struct GuessDTO {
    pub gap_id: u32,
    pub token: String, // user token
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct GuessesDTO {
    pub gap_id: u32,
    pub token: String, // user token
    pub guesses: Vec<GuessDTO>,
}

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PublicGroup {
    pub id: Uuid,
    pub conversation_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_inbox_id: String,
    pub owner_wallet: String,
    pub is_public: bool,
    pub member_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePublicGroupRequest {
    pub conversation_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_inbox_id: String,
    pub owner_wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePublicGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub is_public: Option<bool>,
    pub member_count: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct PublicGroupResponse {
    pub id: String,
    pub conversation_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_inbox_id: String,
    pub owner_wallet: String,
    pub is_public: bool,
    pub member_count: i32,
    pub created_at: String,
}

impl From<PublicGroup> for PublicGroupResponse {
    fn from(g: PublicGroup) -> Self {
        PublicGroupResponse {
            id: g.id.to_string(),
            conversation_id: g.conversation_id,
            name: g.name,
            description: g.description,
            image_url: g.image_url,
            owner_inbox_id: g.owner_inbox_id,
            owner_wallet: g.owner_wallet,
            is_public: g.is_public,
            member_count: g.member_count,
            created_at: g.created_at.to_rfc3339(),
        }
    }
}

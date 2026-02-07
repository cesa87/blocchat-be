use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserProfile {
    pub id: Uuid,
    pub wallet_address: String,
    pub inbox_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub last_username_change: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub wallet_address: String,
    pub inbox_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ClaimUsernameRequest {
    pub wallet_address: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub wallet_address: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub wallet_address: String,
    pub inbox_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<UserProfile> for ProfileResponse {
    fn from(profile: UserProfile) -> Self {
        Self {
            wallet_address: profile.wallet_address,
            inbox_id: profile.inbox_id,
            username: profile.username,
            display_name: profile.display_name,
            avatar_url: profile.avatar_url,
            bio: profile.bio,
            created_at: profile.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub wallet_address: String,
    pub inbox_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl From<UserProfile> for SearchResult {
    fn from(profile: UserProfile) -> Self {
        Self {
            wallet_address: profile.wallet_address,
            inbox_id: profile.inbox_id,
            username: profile.username,
            display_name: profile.display_name,
            avatar_url: profile.avatar_url,
        }
    }
}

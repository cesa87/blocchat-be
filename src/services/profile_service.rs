use crate::db::DbPool;
use crate::models::{CreateProfileRequest, UpdateProfileRequest, UserProfile, SearchResult};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use regex::Regex;

const USERNAME_CHANGE_COOLDOWN_DAYS: i64 = 30;

/// Validate username format
/// Rules: 3-30 characters, alphanumeric + underscore, no spaces
pub fn validate_username(username: &str) -> Result<()> {
    // Length check
    if username.len() < 3 {
        return Err(anyhow!("Username must be at least 3 characters"));
    }
    if username.len() > 30 {
        return Err(anyhow!("Username must be 30 characters or less"));
    }
    
    // Format check: alphanumeric + underscore only
    let re = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
    if !re.is_match(username) {
        return Err(anyhow!("Username can only contain letters, numbers, and underscores"));
    }
    
    // Must start with letter or number (not underscore)
    if username.starts_with('_') {
        return Err(anyhow!("Username must start with a letter or number"));
    }
    
    Ok(())
}

/// Check if username change is allowed (30-day cooldown)
fn can_change_username(last_change: Option<chrono::DateTime<Utc>>) -> Result<()> {
    if let Some(last) = last_change {
        let now = Utc::now();
        let diff = now.signed_duration_since(last);
        
        if diff < Duration::days(USERNAME_CHANGE_COOLDOWN_DAYS) {
            let days_remaining = USERNAME_CHANGE_COOLDOWN_DAYS - diff.num_days();
            return Err(anyhow!(
                "Username can only be changed once every 30 days. {} days remaining",
                days_remaining
            ));
        }
    }
    Ok(())
}

/// Create or get user profile
pub async fn get_or_create_profile(
    pool: &DbPool,
    wallet_address: &str,
    inbox_id: &str,
) -> Result<UserProfile> {
    let wallet_lower = wallet_address.to_lowercase();
    
    // Try to get existing profile
    if let Ok(profile) = get_profile_by_wallet(pool, &wallet_lower).await {
        return Ok(profile);
    }
    
    // Create new profile
    let profile = sqlx::query_as::<_, UserProfile>(
        r#"
        INSERT INTO user_profiles (wallet_address, inbox_id)
        VALUES ($1, $2)
        ON CONFLICT (wallet_address) DO UPDATE SET inbox_id = $2
        RETURNING *
        "#
    )
    .bind(&wallet_lower)
    .bind(inbox_id)
    .fetch_one(pool)
    .await?;
    
    Ok(profile)
}

/// Get profile by wallet address
pub async fn get_profile_by_wallet(pool: &DbPool, wallet_address: &str) -> Result<UserProfile> {
    let profile = sqlx::query_as::<_, UserProfile>(
        "SELECT * FROM user_profiles WHERE wallet_address = $1"
    )
    .bind(wallet_address.to_lowercase())
    .fetch_one(pool)
    .await?;
    
    Ok(profile)
}

/// Get profile by username
pub async fn get_profile_by_username(pool: &DbPool, username: &str) -> Result<UserProfile> {
    let profile = sqlx::query_as::<_, UserProfile>(
        "SELECT * FROM user_profiles WHERE LOWER(username) = LOWER($1)"
    )
    .bind(username)
    .fetch_one(pool)
    .await?;
    
    Ok(profile)
}

/// Get profile by inbox_id
pub async fn get_profile_by_inbox_id(pool: &DbPool, inbox_id: &str) -> Result<UserProfile> {
    let profile = sqlx::query_as::<_, UserProfile>(
        "SELECT * FROM user_profiles WHERE inbox_id = $1"
    )
    .bind(inbox_id)
    .fetch_one(pool)
    .await?;
    
    Ok(profile)
}

/// Check if username is available
pub async fn is_username_available(pool: &DbPool, username: &str, current_wallet: &str) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT wallet_address FROM user_profiles 
        WHERE LOWER(username) = LOWER($1) AND wallet_address != $2
        "#,
        username,
        current_wallet.to_lowercase()
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(result.is_none())
}

/// Claim or update username
pub async fn claim_username(
    pool: &DbPool,
    wallet_address: &str,
    username: &str,
) -> Result<UserProfile> {
    let wallet_lower = wallet_address.to_lowercase();
    
    // Validate username format
    validate_username(username)?;
    
    // Get current profile
    let profile = get_profile_by_wallet(pool, &wallet_lower).await?;
    
    // Check if username is changing
    if let Some(current_username) = &profile.username {
        if current_username.to_lowercase() == username.to_lowercase() {
            return Ok(profile); // No change needed
        }
        
        // Check cooldown
        can_change_username(profile.last_username_change)?;
    }
    
    // Check if username is available
    if !is_username_available(pool, username, &wallet_lower).await? {
        return Err(anyhow!("Username '{}' is already taken", username));
    }
    
    // Update username
    let updated = sqlx::query_as::<_, UserProfile>(
        r#"
        UPDATE user_profiles
        SET username = $1, last_username_change = NOW()
        WHERE wallet_address = $2
        RETURNING *
        "#
    )
    .bind(username)
    .bind(&wallet_lower)
    .fetch_one(pool)
    .await?;
    
    Ok(updated)
}

/// Update user profile (display name, avatar, bio)
pub async fn update_profile(
    pool: &DbPool,
    req: UpdateProfileRequest,
) -> Result<UserProfile> {
    let wallet_lower = req.wallet_address.to_lowercase();
    
    // Get current profile
    let profile = get_profile_by_wallet(pool, &wallet_lower).await?;
    
    // If username is being updated, validate and check cooldown
    if let Some(new_username) = &req.username {
        validate_username(new_username)?;
        
        if let Some(current_username) = &profile.username {
            if current_username.to_lowercase() != new_username.to_lowercase() {
                can_change_username(profile.last_username_change)?;
                
                if !is_username_available(pool, new_username, &wallet_lower).await? {
                    return Err(anyhow!("Username '{}' is already taken", new_username));
                }
            }
        }
    }
    
    // Build dynamic update query
    let updated = sqlx::query_as::<_, UserProfile>(
        r#"
        UPDATE user_profiles
        SET 
            username = COALESCE($1, username),
            display_name = COALESCE($2, display_name),
            avatar_url = COALESCE($3, avatar_url),
            bio = COALESCE($4, bio),
            last_username_change = CASE WHEN $1 IS NOT NULL AND $1 != username THEN NOW() ELSE last_username_change END
        WHERE wallet_address = $5
        RETURNING *
        "#
    )
    .bind(&req.username)
    .bind(&req.display_name)
    .bind(&req.avatar_url)
    .bind(&req.bio)
    .bind(&wallet_lower)
    .fetch_one(pool)
    .await?;
    
    Ok(updated)
}

/// Search profiles by username, wallet address, or inbox_id
pub async fn search_profiles(pool: &DbPool, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
    let search_pattern = format!("%{}%", query.to_lowercase());
    
    let profiles = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT * FROM user_profiles
        WHERE 
            LOWER(username) LIKE $1
            OR LOWER(display_name) LIKE $1
            OR LOWER(wallet_address) LIKE $1
            OR inbox_id = $2
        ORDER BY 
            CASE 
                WHEN inbox_id = $2 THEN 0
                WHEN LOWER(username) = $3 THEN 1 
                ELSE 2 
            END,
            created_at DESC
        LIMIT $4
        "#
    )
    .bind(&search_pattern)
    .bind(query)  // Exact match for inbox_id
    .bind(query.to_lowercase())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    Ok(profiles.into_iter().map(SearchResult::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_username() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("alice_123").is_ok());
        assert!(validate_username("user_name_2024").is_ok());
        
        assert!(validate_username("ab").is_err()); // Too short
        assert!(validate_username("a".repeat(31).as_str()).is_err()); // Too long
        assert!(validate_username("alice bob").is_err()); // Space
        assert!(validate_username("alice-bob").is_err()); // Hyphen
        assert!(validate_username("_alice").is_err()); // Starts with underscore
        assert!(validate_username("alice!").is_err()); // Special char
    }
}

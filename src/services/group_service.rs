use sqlx::PgPool;
use crate::models::group::{PublicGroup, CreatePublicGroupRequest, UpdatePublicGroupRequest};

pub async fn register_group(
    pool: &PgPool,
    req: CreatePublicGroupRequest,
) -> Result<PublicGroup, sqlx::Error> {
    sqlx::query_as::<_, PublicGroup>(
        r#"INSERT INTO public_groups (conversation_id, name, description, image_url, owner_inbox_id, owner_wallet)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (conversation_id) DO UPDATE SET
             name = EXCLUDED.name,
             description = EXCLUDED.description,
             image_url = EXCLUDED.image_url,
             is_public = true
           RETURNING *"#,
    )
    .bind(&req.conversation_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.image_url)
    .bind(&req.owner_inbox_id)
    .bind(&req.owner_wallet)
    .fetch_one(pool)
    .await
}

pub async fn get_group(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<PublicGroup, sqlx::Error> {
    sqlx::query_as::<_, PublicGroup>(
        "SELECT * FROM public_groups WHERE conversation_id = $1",
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
}

pub async fn update_group(
    pool: &PgPool,
    conversation_id: &str,
    req: UpdatePublicGroupRequest,
) -> Result<PublicGroup, sqlx::Error> {
    // Build dynamic update
    let mut updates = Vec::new();
    let mut param_idx = 2u32; // $1 is conversation_id

    if req.name.is_some() { updates.push(format!("name = ${}", param_idx)); param_idx += 1; }
    if req.description.is_some() { updates.push(format!("description = ${}", param_idx)); param_idx += 1; }
    if req.image_url.is_some() { updates.push(format!("image_url = ${}", param_idx)); param_idx += 1; }
    if req.is_public.is_some() { updates.push(format!("is_public = ${}", param_idx)); param_idx += 1; }
    if req.member_count.is_some() { updates.push(format!("member_count = ${}", param_idx)); param_idx += 1; }

    if updates.is_empty() {
        return get_group(pool, conversation_id).await;
    }

    let _ = param_idx; // suppress unused warning

    let sql = format!(
        "UPDATE public_groups SET {} WHERE conversation_id = $1 RETURNING *",
        updates.join(", ")
    );

    let mut query = sqlx::query_as::<_, PublicGroup>(&sql).bind(conversation_id);

    if let Some(ref v) = req.name { query = query.bind(v); }
    if let Some(ref v) = req.description { query = query.bind(v); }
    if let Some(ref v) = req.image_url { query = query.bind(v); }
    if let Some(ref v) = req.is_public { query = query.bind(v); }
    if let Some(ref v) = req.member_count { query = query.bind(v); }

    query.fetch_one(pool).await
}

pub async fn delete_group(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM public_groups WHERE conversation_id = $1")
        .bind(conversation_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn search_groups(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<PublicGroup>, sqlx::Error> {
    sqlx::query_as::<_, PublicGroup>(
        r#"SELECT * FROM public_groups
           WHERE is_public = true
             AND (name ILIKE $1 OR description ILIKE $1)
           ORDER BY member_count DESC, created_at DESC
           LIMIT $2"#,
    )
    .bind(format!("%{}%", query))
    .bind(limit)
    .fetch_all(pool)
    .await
}

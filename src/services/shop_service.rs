use crate::models::{
    CreateItemRequest, CreateShopRequest, ItemResponse, Shop, ShopItem, ShopResponse,
    UpdateItemRequest, UpdateShopRequest,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_shop(
    pool: &PgPool,
    conversation_id: &str,
    req: CreateShopRequest,
) -> Result<ShopResponse, sqlx::Error> {
    let shop = sqlx::query_as::<_, Shop>(
        r#"
        INSERT INTO shops (conversation_id, name, owner_address)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(conversation_id)
    .bind(&req.name)
    .bind(&req.owner_address)
    .fetch_one(pool)
    .await?;

    Ok(ShopResponse::from(shop))
}

pub async fn get_shops_by_conversation(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<Vec<ShopResponse>, sqlx::Error> {
    let shops = sqlx::query_as::<_, Shop>(
        r#"
        SELECT * FROM shops
        WHERE conversation_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    let mut shop_responses = Vec::new();

    for shop in shops {
        let items = get_shop_items(pool, &shop.id).await?;
        let mut shop_response = ShopResponse::from(shop);
        shop_response.items = items;
        shop_responses.push(shop_response);
    }

    Ok(shop_responses)
}

pub async fn get_shop(pool: &PgPool, shop_id: &Uuid) -> Result<ShopResponse, sqlx::Error> {
    let shop = sqlx::query_as::<_, Shop>(
        r#"
        SELECT * FROM shops
        WHERE id = $1
        "#,
    )
    .bind(shop_id)
    .fetch_one(pool)
    .await?;

    let items = get_shop_items(pool, shop_id).await?;
    let mut shop_response = ShopResponse::from(shop);
    shop_response.items = items;

    Ok(shop_response)
}

pub async fn update_shop(
    pool: &PgPool,
    shop_id: &Uuid,
    req: UpdateShopRequest,
) -> Result<ShopResponse, sqlx::Error> {
    let shop = sqlx::query_as::<_, Shop>(
        r#"
        UPDATE shops
        SET name = $1, updated_at = CURRENT_TIMESTAMP
        WHERE id = $2
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(shop_id)
    .fetch_one(pool)
    .await?;

    let items = get_shop_items(pool, shop_id).await?;
    let mut shop_response = ShopResponse::from(shop);
    shop_response.items = items;

    Ok(shop_response)
}

pub async fn delete_shop(pool: &PgPool, shop_id: &Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM shops
        WHERE id = $1
        "#,
    )
    .bind(shop_id)
    .execute(pool)
    .await?;

    Ok(())
}

// Shop Items Operations

pub async fn create_item(
    pool: &PgPool,
    shop_id: &Uuid,
    req: CreateItemRequest,
) -> Result<ItemResponse, sqlx::Error> {
    let item = sqlx::query_as::<_, ShopItem>(
        r#"
        INSERT INTO shop_items (shop_id, name, description, price, token_address, token_symbol, image_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(shop_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.price)
    .bind(&req.token_address)
    .bind(&req.token_symbol)
    .bind(&req.image_url)
    .fetch_one(pool)
    .await?;

    Ok(ItemResponse::from(item))
}

pub async fn get_shop_items(
    pool: &PgPool,
    shop_id: &Uuid,
) -> Result<Vec<ItemResponse>, sqlx::Error> {
    let items = sqlx::query_as::<_, ShopItem>(
        r#"
        SELECT * FROM shop_items
        WHERE shop_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(shop_id)
    .fetch_all(pool)
    .await?;

    Ok(items.into_iter().map(ItemResponse::from).collect())
}

pub async fn update_item(
    pool: &PgPool,
    item_id: &Uuid,
    req: UpdateItemRequest,
) -> Result<ItemResponse, sqlx::Error> {
    let item = sqlx::query_as::<_, ShopItem>(
        r#"
        UPDATE shop_items
        SET name = $1, description = $2, price = $3, token_address = $4, token_symbol = $5, image_url = $6, updated_at = CURRENT_TIMESTAMP
        WHERE id = $7
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.price)
    .bind(&req.token_address)
    .bind(&req.token_symbol)
    .bind(&req.image_url)
    .bind(item_id)
    .fetch_one(pool)
    .await?;

    Ok(ItemResponse::from(item))
}

pub async fn delete_item(pool: &PgPool, item_id: &Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM shop_items
        WHERE id = $1
        "#,
    )
    .bind(item_id)
    .execute(pool)
    .await?;

    Ok(())
}

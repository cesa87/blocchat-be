use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Shop {
    pub id: Uuid,
    pub conversation_id: String,
    pub name: String,
    pub owner_address: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ShopItem {
    pub id: Uuid,
    pub shop_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub image_url: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateShopRequest {
    pub name: String,
    pub owner_address: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShopRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShopResponse {
    pub id: String,
    pub conversation_id: String,
    pub name: String,
    pub owner_address: String,
    pub items: Vec<ItemResponse>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse {
    pub id: String,
    pub shop_id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub image_url: Option<String>,
    pub created_at: String,
}

impl From<Shop> for ShopResponse {
    fn from(shop: Shop) -> Self {
        ShopResponse {
            id: shop.id.to_string(),
            conversation_id: shop.conversation_id,
            name: shop.name,
            owner_address: shop.owner_address,
            items: vec![],
            created_at: shop.created_at.to_string(),
        }
    }
}

impl From<ShopItem> for ItemResponse {
    fn from(item: ShopItem) -> Self {
        ItemResponse {
            id: item.id.to_string(),
            shop_id: item.shop_id.to_string(),
            name: item.name,
            description: item.description,
            price: item.price,
            token_address: item.token_address,
            token_symbol: item.token_symbol,
            image_url: item.image_url,
            created_at: item.created_at.to_string(),
        }
    }
}

use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Scope};
use crate::models::{CreateItemRequest, CreateShopRequest, UpdateItemRequest, UpdateShopRequest};
use crate::services::shop_service;
use sqlx::PgPool;
use uuid::Uuid;

// Shop Endpoints

#[post("/conversations/{conversation_id}/shops")]
async fn create_shop(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
    req: web::Json<CreateShopRequest>,
) -> impl Responder {
    match shop_service::create_shop(&pool, &conversation_id, req.into_inner()).await {
        Ok(shop) => HttpResponse::Created().json(shop),
        Err(e) => {
            eprintln!("Failed to create shop: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create shop"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}/shops")]
async fn get_shops(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match shop_service::get_shops_by_conversation(&pool, &conversation_id).await {
        Ok(shops) => HttpResponse::Ok().json(shops),
        Err(e) => {
            eprintln!("Failed to get shops: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get shops"
            }))
        }
    }
}

#[get("/shops/{shop_id}")]
async fn get_shop(pool: web::Data<PgPool>, shop_id: web::Path<Uuid>) -> impl Responder {
    match shop_service::get_shop(&pool, &shop_id).await {
        Ok(shop) => HttpResponse::Ok().json(shop),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Shop not found"
        })),
        Err(e) => {
            eprintln!("Failed to get shop: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get shop"
            }))
        }
    }
}

#[put("/shops/{shop_id}")]
async fn update_shop(
    pool: web::Data<PgPool>,
    shop_id: web::Path<Uuid>,
    req: web::Json<UpdateShopRequest>,
) -> impl Responder {
    match shop_service::update_shop(&pool, &shop_id, req.into_inner()).await {
        Ok(shop) => HttpResponse::Ok().json(shop),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Shop not found"
        })),
        Err(e) => {
            eprintln!("Failed to update shop: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update shop"
            }))
        }
    }
}

#[delete("/shops/{shop_id}")]
async fn delete_shop(pool: web::Data<PgPool>, shop_id: web::Path<Uuid>) -> impl Responder {
    match shop_service::delete_shop(&pool, &shop_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Failed to delete shop: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete shop"
            }))
        }
    }
}

// Shop Item Endpoints

#[post("/shops/{shop_id}/items")]
async fn create_item(
    pool: web::Data<PgPool>,
    shop_id: web::Path<Uuid>,
    req: web::Json<CreateItemRequest>,
) -> impl Responder {
    match shop_service::create_item(&pool, &shop_id, req.into_inner()).await {
        Ok(item) => HttpResponse::Created().json(item),
        Err(e) => {
            eprintln!("Failed to create item: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create item"
            }))
        }
    }
}

#[get("/shops/{shop_id}/items")]
async fn get_items(pool: web::Data<PgPool>, shop_id: web::Path<Uuid>) -> impl Responder {
    match shop_service::get_shop_items(&pool, &shop_id).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            eprintln!("Failed to get items: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get items"
            }))
        }
    }
}

#[put("/items/{item_id}")]
async fn update_item(
    pool: web::Data<PgPool>,
    item_id: web::Path<Uuid>,
    req: web::Json<UpdateItemRequest>,
) -> impl Responder {
    match shop_service::update_item(&pool, &item_id, req.into_inner()).await {
        Ok(item) => HttpResponse::Ok().json(item),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Item not found"
        })),
        Err(e) => {
            eprintln!("Failed to update item: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update item"
            }))
        }
    }
}

#[delete("/items/{item_id}")]
async fn delete_item(pool: web::Data<PgPool>, item_id: web::Path<Uuid>) -> impl Responder {
    match shop_service::delete_item(&pool, &item_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Failed to delete item: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete item"
            }))
        }
    }
}

pub fn configure() -> Scope {
    web::scope("/shops")
        .service(create_shop)
        .service(get_shops)
        .service(get_shop)
        .service(update_shop)
        .service(delete_shop)
        .service(create_item)
        .service(get_items)
        .service(update_item)
        .service(delete_item)
}

use actix_web::{get, post, web, HttpResponse, Responder, Scope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

/// Shared in-memory store: conversation_id -> (inbox_id -> last_typed_at)
pub type TypingStore = RwLock<HashMap<String, HashMap<String, Instant>>>;

/// How long a typing signal stays valid (seconds)
const TYPING_EXPIRY_SECS: u64 = 5;

#[derive(Deserialize)]
pub struct TypingRequest {
    pub inbox_id: String,
}

#[derive(Serialize)]
struct TypingResponse {
    inbox_ids: Vec<String>,
}

/// POST /conversations/{conversation_id}/typing
/// Record that a user is currently typing.
#[post("/conversations/{conversation_id}/typing")]
async fn post_typing(
    store: web::Data<TypingStore>,
    conversation_id: web::Path<String>,
    body: web::Json<TypingRequest>,
) -> impl Responder {
    let conv_id = conversation_id.into_inner();
    let inbox_id = body.into_inner().inbox_id;

    if let Ok(mut map) = store.write() {
        map.entry(conv_id)
            .or_default()
            .insert(inbox_id, Instant::now());
    }

    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}

/// GET /conversations/{conversation_id}/typing
/// Return the list of inbox_ids currently typing (not expired).
#[get("/conversations/{conversation_id}/typing")]
async fn get_typing(
    store: web::Data<TypingStore>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    let conv_id = conversation_id.into_inner();
    let expiry = std::time::Duration::from_secs(TYPING_EXPIRY_SECS);

    let mut active: Vec<String> = Vec::new();

    if let Ok(mut map) = store.write() {
        if let Some(users) = map.get_mut(&conv_id) {
            // Remove expired entries and collect active ones
            users.retain(|_, ts| ts.elapsed() < expiry);
            active = users.keys().cloned().collect();
        }
    }

    HttpResponse::Ok().json(TypingResponse { inbox_ids: active })
}

pub fn configure() -> Scope {
    web::scope("/conversations")
        .service(post_typing)
        .service(get_typing)
}

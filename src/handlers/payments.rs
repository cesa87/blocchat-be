use actix_web::{get, post, web, HttpResponse, Responder, Scope};
use crate::{
    db::DbPool,
    models::{CreateTransactionRequest, TransactionResponse},
    services::payment_service,
};

pub fn configure() -> Scope {
    web::scope("/payments")
        .service(create_transaction)
        .service(get_transaction)
        .service(get_conversation_transactions)
}

#[post("/transactions")]
async fn create_transaction(
    pool: web::Data<DbPool>,
    req: web::Json<CreateTransactionRequest>,
) -> impl Responder {
    match payment_service::create_transaction(&pool, req.into_inner()).await {
        Ok(tx) => HttpResponse::Created().json(TransactionResponse::from(tx)),
        Err(e) => {
            log::error!("Failed to create transaction: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create transaction"
            }))
        }
    }
}

#[get("/transactions/{tx_hash}")]
async fn get_transaction(
    pool: web::Data<DbPool>,
    tx_hash: web::Path<String>,
) -> impl Responder {
    match payment_service::get_transaction_by_hash(&pool, &tx_hash).await {
        Ok(Some(tx)) => HttpResponse::Ok().json(TransactionResponse::from(tx)),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Transaction not found"
        })),
        Err(e) => {
            log::error!("Failed to get transaction: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get transaction"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}/transactions")]
async fn get_conversation_transactions(
    pool: web::Data<DbPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match payment_service::get_conversation_transactions(&pool, &conversation_id).await {
        Ok(transactions) => {
            let responses: Vec<TransactionResponse> = transactions
                .into_iter()
                .map(TransactionResponse::from)
                .collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            log::error!("Failed to get conversation transactions: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get transactions"
            }))
        }
    }
}

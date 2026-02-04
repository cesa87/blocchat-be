use crate::{
    db::DbPool,
    models::{CreateTransactionRequest, Transaction, TransactionStatus},
};
use anyhow::Result;
use uuid::Uuid;

pub async fn create_transaction(
    pool: &DbPool,
    req: CreateTransactionRequest,
) -> Result<Transaction> {
    let tx = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (
            id, tx_hash, from_address, to_address, amount, 
            token_address, chain_id, conversation_id, message_id, 
            status, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.tx_hash)
    .bind(&req.from_address)
    .bind(&req.to_address)
    .bind(&req.amount)
    .bind(&req.token_address)
    .bind(req.chain_id)
    .bind(&req.conversation_id)
    .bind(&req.message_id)
    .bind(TransactionStatus::Pending)
    .fetch_one(pool)
    .await?;

    Ok(tx)
}

pub async fn get_transaction_by_hash(
    pool: &DbPool,
    tx_hash: &str,
) -> Result<Option<Transaction>> {
    let tx = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE tx_hash = $1"
    )
    .bind(tx_hash)
    .fetch_optional(pool)
    .await?;

    Ok(tx)
}

pub async fn get_conversation_transactions(
    pool: &DbPool,
    conversation_id: &str,
) -> Result<Vec<Transaction>> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE conversation_id = $1 ORDER BY created_at DESC"
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(transactions)
}

pub async fn update_transaction_status(
    pool: &DbPool,
    tx_hash: &str,
    status: TransactionStatus,
    block_number: Option<i64>,
) -> Result<Transaction> {
    let tx = sqlx::query_as::<_, Transaction>(
        r#"
        UPDATE transactions 
        SET status = $1, block_number = $2, confirmed_at = CASE WHEN $1 = 'confirmed' THEN NOW() ELSE confirmed_at END
        WHERE tx_hash = $3
        RETURNING *
        "#,
    )
    .bind(status)
    .bind(block_number)
    .bind(tx_hash)
    .fetch_one(pool)
    .await?;

    Ok(tx)
}

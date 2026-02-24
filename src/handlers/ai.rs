use actix_web::{post, web, HttpResponse, Responder, Scope};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize)]
pub struct AskAIRequest {
    pub prompt: String,
    pub context_message: Option<String>,
}

#[derive(Serialize)]
pub struct AskAIResponse {
    pub answer: String,
}

#[post("/conversations/{conversation_id}/ask")]
async fn ask_ai(
    _conversation_id: web::Path<String>,
    req: web::Json<AskAIRequest>,
) -> impl Responder {
    // Mock delay to simulate thinking
    sleep(Duration::from_secs(2)).await;

    let answer = if let Some(context) = &req.context_message {
        format!(
            "🤖 **Fact Check Analysis**\n\nI analyzed the statement: \"{}\"\n\n**Verdict:** Needs Verification\n\n**Details:**\nThis claim requires checking on-chain data. Based on general knowledge, similar mechanisms exist but specifics vary by protocol.\n\n(This is a mock AI response)",
            context
        )
    } else {
        format!(
            "🤖 **AI Assistant**\n\n**Question:** {}\n\n**Answer:**\nThis is a simulated response from the AI assistant. In a production environment, this would call an LLM API to provide a detailed explanation.\n\n(This is a mock AI response)",
            req.prompt
        )
    };

    // In a real implementation, we would post this message to the conversation via XMTP or internal DB.
    // For now, we return it to the frontend to display (or frontend posts it).
    // The frontend logic I wrote earlier doesn't post the message, it just sends the request.
    // I should probably have the frontend post the "answer" as a message from the user (if "Assistant" mode is client-side)
    // or better, the backend should insert it into the message stream if possible.
    
    // Since I can't easily inject into XMTP stream from here without keys, returning it is fine.
    // The frontend `handleAskAI` catches the response but currently does nothing with it except `alert`.
    // I should update frontend `ChatWindow.tsx` to display the response or send it as a message.

    HttpResponse::Ok().json(AskAIResponse { answer })
}

pub fn configure() -> Scope {
    web::scope("/ai")
        .service(ask_ai)
}

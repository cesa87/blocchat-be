use actix_web::{post, web, HttpResponse, Responder, Scope};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

#[derive(Deserialize)]
pub struct AskAIRequest {
    pub prompt: String,
    pub context_message: Option<String>,
}

#[derive(Serialize)]
pub struct AskAIResponse {
    pub answer: String,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[post("/conversations/{conversation_id}/ask")]
async fn ask_ai(
    _conversation_id: web::Path<String>,
    req: web::Json<AskAIRequest>,
) -> impl Responder {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => return HttpResponse::InternalServerError().json(AskAIResponse {
            answer: "Error: GEMINI_API_KEY not set on server.".to_string(),
        }),
    };

    let prompt_text = if let Some(context) = &req.context_message {
        format!(
            "Context: {}\n\nUser Question: {}",
            context, req.prompt
        )
    } else {
        req.prompt.clone()
    };

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let request_body = json!({
        "contents": [{
            "parts": [{
                "text": prompt_text
            }]
        }]
    });

    match client.post(&url).json(&request_body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<GeminiResponse>().await {
                    Ok(gemini_resp) => {
                        if let Some(candidates) = gemini_resp.candidates {
                            if let Some(first_candidate) = candidates.first() {
                                if let Some(first_part) = first_candidate.content.parts.first() {
                                    return HttpResponse::Ok().json(AskAIResponse {
                                        answer: first_part.text.clone(),
                                    });
                                }
                            }
                        }
                        HttpResponse::InternalServerError().json(AskAIResponse {
                            answer: "Error: No valid response content from AI.".to_string(),
                        })
                    }
                    Err(e) => HttpResponse::InternalServerError().json(AskAIResponse {
                        answer: format!("Error parsing AI response: {}", e),
                    }),
                }
            } else {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                HttpResponse::InternalServerError().json(AskAIResponse {
                    answer: format!("Error calling AI API: {}", error_text),
                })
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(AskAIResponse {
            answer: format!("Error sending request to AI API: {}", e),
        }),
    }
}

pub fn configure() -> Scope {
    web::scope("/ai")
        .service(ask_ai)
}

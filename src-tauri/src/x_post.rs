use reqwest::Client;
use std::time::Duration;

/// Post a text tweet via X API v2 (OAuth 2.0 user access token).
pub async fn post_tweet(access_token: &str, text: &str) -> Result<String, String> {
    let url = "https://api.twitter.com/2/tweets";

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({ "text": text });

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("X API request failed: {}", e))?;

    let status = res.status();
    let text_body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("X API error ({}): {}", status, text_body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&text_body).map_err(|e| format!("Invalid X response: {}", e))?;

    json["data"]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No tweet id in X response: {}", text_body))
}

/// Verify OAuth 2.0 access token via GET /2/users/me
pub async fn verify_credentials(access_token: &str) -> Result<String, String> {
    let url = "https://api.twitter.com/2/users/me?user.fields=username";

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("X API request failed: {}", e))?;

    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("X credentials check failed ({}): {}", status, body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid X response: {}", e))?;

    let username = json["data"]["username"]
        .as_str()
        .unwrap_or("unknown");

    Ok(format!("Connected as @{}", username))
}
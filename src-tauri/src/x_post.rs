use hmac::{Hmac, Mac};
use reqwest::Client;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

pub fn extract_tweet_id_from_url(url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    for host in ["x.com/", "twitter.com/"] {
        if let Some(idx) = lower.find(host) {
            let rest = &url[idx + host.len()..];
            if let Some(status_idx) = rest.to_lowercase().find("/status/") {
                let after = &rest[status_idx + "/status/".len()..];
                let id: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !id.is_empty() {
                    return Some(id);
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct XCredentials {
    pub api_key: String,
    pub api_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn oauth_signature(
    method: &str,
    base_url: &str,
    params: &BTreeMap<String, String>,
    consumer_secret: &str,
    token_secret: &str,
) -> Result<String, String> {
    let param_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let base_string = format!(
        "{}&{}&{}",
        percent_encode(method),
        percent_encode(base_url),
        percent_encode(&param_string)
    );

    let signing_key = format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    );

    let mut mac =
        HmacSha1::new_from_slice(signing_key.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(base_string.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        result,
    ))
}

fn oauth_header(
    method: &str,
    url: &str,
    creds: &XCredentials,
    extra_params: &BTreeMap<String, String>,
) -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        .to_string();
    let nonce: String = uuid::Uuid::new_v4().to_string().replace('-', "");

    let mut oauth_params = BTreeMap::new();
    oauth_params.insert("oauth_consumer_key".to_string(), creds.api_key.clone());
    oauth_params.insert("oauth_nonce".to_string(), nonce);
    oauth_params.insert("oauth_signature_method".to_string(), "HMAC-SHA1".to_string());
    oauth_params.insert("oauth_timestamp".to_string(), timestamp);
    oauth_params.insert("oauth_token".to_string(), creds.access_token.clone());
    oauth_params.insert("oauth_version".to_string(), "1.0".to_string());

    let mut sign_params = oauth_params.clone();
    for (k, v) in extra_params {
        sign_params.insert(k.clone(), v.clone());
    }

    let signature = oauth_signature(
        method,
        url,
        &sign_params,
        &creds.api_secret,
        &creds.access_token_secret,
    )?;
    oauth_params.insert("oauth_signature".to_string(), signature);

    let auth_parts: Vec<String> = oauth_params
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", percent_encode(k), percent_encode(v)))
        .collect();

    Ok(format!("OAuth {}", auth_parts.join(", ")))
}

fn oauth_url_parts(url: &str) -> (String, BTreeMap<String, String>) {
    let mut query_params = BTreeMap::new();
    let base = if let Some(idx) = url.find('?') {
        let query = &url[idx + 1..];
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let decoded_key = urlencoding::decode(key)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| key.to_string());
            let decoded_value = urlencoding::decode(value)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| value.to_string());
            query_params.insert(decoded_key, decoded_value);
        }
        url[..idx].to_string()
    } else {
        url.to_string()
    };
    (base, query_params)
}

pub fn oauth_get_header(url: &str, creds: &XCredentials) -> Result<String, String> {
    let (base, query_params) = oauth_url_parts(url);
    oauth_header("GET", &base, creds, &query_params)
}

pub fn oauth_post_multipart_header(url: &str, creds: &XCredentials) -> Result<String, String> {
    oauth_header("POST", url, creds, &BTreeMap::new())
}

fn format_x_api_error(status: reqwest::StatusCode, body: &str) -> String {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
    if let Ok(json) = parsed {
        let problem_type = json["type"].as_str().unwrap_or("");
        let detail = json["detail"].as_str().unwrap_or(body);

        if problem_type.contains("oauth1-permissions") {
            return format!(
                "X API error ({}): {}. \
                Your app or access token does not have write permission. In the X Developer Portal: \
                (1) open your app → Settings → User authentication setup → Edit, \
                (2) enable OAuth 1.0a and set App permissions to Read and write, \
                (3) save, then regenerate Access Token and Secret under Keys and tokens, \
                (4) paste the new tokens into x-poster Settings.",
                status, detail
            );
        }

        // X API anti-spam (2026): API replies to other users often fail even when the web UI allows them.
        // This is NOT the same as a "protected" account — protected posts use a different error.
        let detail_lower = detail.to_lowercase();
        if detail_lower.contains("not been mentioned")
            || detail_lower.contains("otherwise engaged by the author")
            || (detail_lower.contains("reply to this conversation is not allowed")
                && status.as_u16() == 403)
        {
            return format!(
                "X blocked this *API* reply (not a protected-account issue). \
                Since mid-2026, X limits app/API replies to posts where the author has already \
                mentioned or engaged with your account — even when the same reply works from the X website. \
                Your draft is still pending. Open the parent post on X and paste the draft text to reply manually. \
                (Standalone posts and replies to people who have engaged you still work via the API.) \
                Details: {}",
                detail
            );
        }

        return format!("X API error ({}): {}", status, detail);
    }

    format!("X API error ({}): {}", status, body)
}

/// Ensure a reply text includes `@handle` for the parent author (web clients often do this).
/// Truncates to 280 chars if needed. No-op when author is unknown or already leading.
pub fn ensure_reply_author_mention(text: &str, author: Option<&str>) -> String {
    let handle = author
        .map(|a| a.trim().trim_start_matches('@').trim())
        .filter(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    let Some(handle) = handle else {
        return text.to_string();
    };

    let trimmed = text.trim();
    let prefix = format!("@{}", handle);
    if trimmed.to_lowercase().starts_with(&prefix.to_lowercase()) {
        return trimmed.to_string();
    }

    let with_mention = format!("{} {}", prefix, trimmed);
    if with_mention.chars().count() <= 280 {
        return with_mention;
    }

    // Prefer keeping the mention; trim body to fit.
    let mention_overhead = prefix.chars().count() + 1; // "@handle "
    let max_body = 280_usize.saturating_sub(mention_overhead);
    let body: String = trimmed.chars().take(max_body).collect();
    format!("{} {}", prefix, body)
}

/// Post a text tweet via X API v2 (OAuth 1.0a user context).
/// When `in_reply_to_tweet_id` is set, posts as a reply to that tweet.
/// `reply_author` is optional `@handle` of the parent author — prepended when missing so the
/// reply matches web-client mention behavior (does not bypass X's API engagement gate).
pub async fn post_tweet(
    creds: &XCredentials,
    text: &str,
    media_ids: &[String],
    in_reply_to_tweet_id: Option<&str>,
    reply_author: Option<&str>,
) -> Result<String, String> {
    let url = "https://api.twitter.com/2/tweets";
    let auth = oauth_header("POST", url, creds, &BTreeMap::new())?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let reply_to = in_reply_to_tweet_id.filter(|id| !id.is_empty());
    let post_text = if reply_to.is_some() {
        ensure_reply_author_mention(text, reply_author)
    } else {
        text.to_string()
    };

    if let Some(id) = reply_to {
        log::info!(
            "post_tweet: reply to tweet_id={}, author={:?}, text_len={}",
            id,
            reply_author,
            post_text.chars().count()
        );
    }

    let mut body = if media_ids.is_empty() {
        serde_json::json!({ "text": post_text })
    } else {
        serde_json::json!({
            "text": post_text,
            "media": { "media_ids": media_ids }
        })
    };

    if let Some(reply_to_id) = reply_to {
        body["reply"] = serde_json::json!({ "in_reply_to_tweet_id": reply_to_id });
    }

    let res = client
        .post(url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("X API request failed: {}", e))?;

    let status = res.status();
    let text_body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format_x_api_error(status, &text_body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&text_body).map_err(|e| format!("Invalid X response: {}", e))?;

    json["data"]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No tweet id in X response: {}", text_body))
}

/// Verify OAuth credentials via GET /2/users/me
pub async fn verify_credentials(creds: &XCredentials) -> Result<String, String> {
    let url = "https://api.twitter.com/2/users/me";
    let auth = oauth_header("GET", url, creds, &BTreeMap::new())?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .header("Authorization", auth)
        .send()
        .await
        .map_err(|e| format!("X API request failed: {}", e))?;

    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format_x_api_error(status, &body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid X response: {}", e))?;

    let username = json["data"]["username"]
        .as_str()
        .unwrap_or("unknown");

    Ok(format!("Connected as @{}", username))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tweet_id_from_url() {
        assert_eq!(
            extract_tweet_id_from_url("https://x.com/SawyerMerritt/status/1928374650123456789"),
            Some("1928374650123456789".to_string())
        );
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
    }

    #[test]
    fn test_oauth_signature_is_deterministic() {
        let mut params = BTreeMap::new();
        params.insert("oauth_consumer_key".to_string(), "key".to_string());
        params.insert("oauth_nonce".to_string(), "nonce".to_string());
        params.insert("oauth_signature_method".to_string(), "HMAC-SHA1".to_string());
        params.insert("oauth_timestamp".to_string(), "123".to_string());
        params.insert("oauth_token".to_string(), "token".to_string());
        params.insert("oauth_version".to_string(), "1.0".to_string());

        let sig1 = oauth_signature(
            "POST",
            "https://api.twitter.com/2/tweets",
            &params,
            "secret",
            "token_secret",
        )
        .unwrap();
        let sig2 = oauth_signature(
            "POST",
            "https://api.twitter.com/2/tweets",
            &params,
            "secret",
            "token_secret",
        )
        .unwrap();
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_format_x_api_error_oauth1_permissions() {
        let body = r#"{"title":"Forbidden","status":403,"detail":"Your client app is not configured with the appropriate oauth1 app permissions for this endpoint.","type":"https://api.twitter.com/2/problems/oauth1-permissions"}"#;
        let msg = format_x_api_error(reqwest::StatusCode::FORBIDDEN, body);
        assert!(msg.contains("Read and write"));
        assert!(msg.contains("regenerate Access Token"));
    }

    #[test]
    fn test_format_x_api_error_reply_engagement_gate() {
        let body = r#"{"title":"Forbidden","status":403,"detail":"Reply to this conversation is not allowed because you have not been mentioned or otherwise engaged by the author of the post you are replying to.","type":"about:blank"}"#;
        let msg = format_x_api_error(reqwest::StatusCode::FORBIDDEN, body);
        assert!(msg.contains("API"), "should clarify API vs web: {}", msg);
        assert!(msg.contains("pending") || msg.contains("manually"), "{}", msg);
        assert!(!msg.to_lowercase().contains("protected account"));
    }

    #[test]
    fn test_ensure_reply_author_mention() {
        assert_eq!(
            ensure_reply_author_mention("Great point on FSD.", Some("@SawyerMerritt")),
            "@SawyerMerritt Great point on FSD."
        );
        assert_eq!(
            ensure_reply_author_mention("@SawyerMerritt already here", Some("SawyerMerritt")),
            "@SawyerMerritt already here"
        );
        assert_eq!(
            ensure_reply_author_mention("no author", None),
            "no author"
        );
        let long = "x".repeat(270);
        let out = ensure_reply_author_mention(&long, Some("elonmusk"));
        assert!(out.starts_with("@elonmusk "));
        assert!(out.chars().count() <= 280);
    }
}
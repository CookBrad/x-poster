use hmac::{Hmac, Mac};
use reqwest::Client;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

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

/// Post a text tweet via X API v2 (OAuth 1.0a user context).
pub async fn post_tweet(creds: &XCredentials, text: &str) -> Result<String, String> {
    let url = "https://api.twitter.com/2/tweets";
    let auth = oauth_header("POST", url, creds, &BTreeMap::new())?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({ "text": text });

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
        return Err(format!("X API error ({}): {}", status, text_body));
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
        return Err(format!("X credentials check failed ({}): {}", status, body));
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
}
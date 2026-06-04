use base64::Engine;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const X_OAUTH_CALLBACK_PORT: u16 = 14555;
pub const X_OAUTH_REDIRECT_URI: &str = "http://127.0.0.1:14555/callback";
pub const X_OAUTH_SCOPES: &str = "tweet.read tweet.write users.read offline.access";

const AUTHORIZE_URL: &str = "https://twitter.com/i/oauth2/authorize";
const TOKEN_URL: &str = "https://api.twitter.com/2/oauth2/token";

pub const KEY_CLIENT_ID: &str = "x_oauth_client_id";
pub const KEY_CLIENT_SECRET: &str = "x_oauth_client_secret";
pub const KEY_ACCESS_TOKEN: &str = "x_oauth_access_token";
pub const KEY_REFRESH_TOKEN: &str = "x_oauth_refresh_token";
pub const KEY_EXPIRES_AT: &str = "x_oauth_expires_at";
pub const KEY_PKCE_VERIFIER: &str = "x_oauth_pkce_verifier";
pub const KEY_OAUTH_STATE: &str = "x_oauth_state";

#[derive(Debug, Clone)]
pub struct XOAuthAppConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub struct XOAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

pub fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn code_challenge_s256(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

pub fn generate_oauth_state() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn build_authorize_url(
    client_id: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        AUTHORIZE_URL,
        urlencoding::encode(client_id),
        urlencoding::encode(X_OAUTH_REDIRECT_URI),
        urlencoding::encode(X_OAUTH_SCOPES),
        urlencoding::encode(state),
        urlencoding::encode(code_challenge),
    )
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn basic_auth_header(client_id: &str, client_secret: &str) -> String {
    let raw = format!("{}:{}", client_id, client_secret);
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
    )
}

pub async fn exchange_code_for_tokens(
    config: &XOAuthAppConfig,
    code: &str,
    code_verifier: &str,
) -> Result<XOAuthTokens, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        urlencoding::encode(code),
        urlencoding::encode(X_OAUTH_REDIRECT_URI),
        urlencoding::encode(code_verifier),
    );

    let res = client
        .post(TOKEN_URL)
        .header("Authorization", basic_auth_header(&config.client_id, &config.client_secret))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token exchange failed: {}", e))?;

    parse_token_response(res).await
}

pub async fn refresh_access_token(
    config: &XOAuthAppConfig,
    refresh_token: &str,
) -> Result<XOAuthTokens, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}",
        urlencoding::encode(refresh_token),
        urlencoding::encode(&config.client_id),
    );

    let res = client
        .post(TOKEN_URL)
        .header("Authorization", basic_auth_header(&config.client_id, &config.client_secret))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {}", e))?;

    parse_token_response(res).await
}

async fn parse_token_response(res: reqwest::Response) -> Result<XOAuthTokens, String> {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("X OAuth token error ({}): {}", status, body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid token response: {}", e))?;

    let access_token = json["access_token"]
        .as_str()
        .ok_or_else(|| format!("No access_token in response: {}", body))?
        .to_string();

    let refresh_token = json["refresh_token"].as_str().map(|s| s.to_string());

    let expires_at = json["expires_in"]
        .as_u64()
        .map(|secs| now_unix() + secs as i64);

    Ok(XOAuthTokens {
        access_token,
        refresh_token,
        expires_at,
    })
}

pub fn open_authorize_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err("Unsupported platform for opening browser".to_string());
    }
    Ok(())
}

/// Parse `code` and `state` from the first line of an HTTP GET request.
pub fn parse_callback_request(request: &str) -> Option<(String, String)> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next().unwrap_or("");
        let decoded = urlencoding::decode(value).ok()?.into_owned();
        match key {
            "code" => code = Some(decoded),
            "state" => state = Some(decoded),
            _ => {}
        }
    }
    Some((code?, state?))
}

const CALLBACK_SUCCESS_HTML: &str = r#"HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Connection: close

<!DOCTYPE html><html><body><h1>x-poster connected</h1><p>You can close this tab and return to the app.</p></body></html>"#;

const CALLBACK_ERROR_HTML: &str = r#"HTTP/1.1 400 Bad Request
Content-Type: text/html; charset=utf-8
Connection: close

<!DOCTYPE html><html><body><h1>Authorization failed</h1><p>Return to x-poster and try again.</p></body></html>"#;

pub async fn wait_for_oauth_callback(
    expected_state: &str,
    timeout: Duration,
) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", X_OAUTH_CALLBACK_PORT))
        .await
        .map_err(|e| {
            format!(
                "Could not bind callback server on {} (is another app using this port?): {}",
                X_OAUTH_REDIRECT_URI, e
            )
        })?;

    let accept = tokio::time::timeout(timeout, async {
        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|e| format!("Callback accept failed: {}", e))?;

            let mut buf = vec![0u8; 8192];
            let n = stream
                .read(&mut buf)
                .await
                .map_err(|e| format!("Callback read failed: {}", e))?;
            let request = String::from_utf8_lossy(&buf[..n]);

            if let Some((code, state)) = parse_callback_request(&request) {
                if state != expected_state {
                    let _ = stream.write_all(CALLBACK_ERROR_HTML.as_bytes()).await;
                    return Err("OAuth state mismatch — possible CSRF. Try connecting again.".to_string());
                }
                let _ = stream.write_all(CALLBACK_SUCCESS_HTML.as_bytes()).await;
                return Ok(code);
            }

            let _ = stream.write_all(CALLBACK_ERROR_HTML.as_bytes()).await;
        }
    })
    .await;

    match accept {
        Ok(Ok(code)) => Ok(code),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Timed out waiting for X authorization. Try Connect again.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_challenge_s256_is_deterministic_url_safe() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1J45ARJ6Xw";
        let c1 = code_challenge_s256(verifier);
        let c2 = code_challenge_s256(verifier);
        assert_eq!(c1, c2);
        assert_eq!(c1, "qq37U0BvscNoy-v4YXiC2RT7QA97nblEf-tWLmedozU");
        assert!(!c1.contains('+'));
        assert!(!c1.contains('/'));
        assert!(!c1.contains('='));
    }

    #[test]
    fn test_parse_callback_request() {
        let req = "GET /callback?state=abc&code=xyz123 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let (code, state) = parse_callback_request(req).unwrap();
        assert_eq!(code, "xyz123");
        assert_eq!(state, "abc");
    }

    #[test]
    fn test_build_authorize_url_contains_pkce() {
        let url = build_authorize_url("client", "state1", "challenge1");
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("client_id=client"));
    }
}
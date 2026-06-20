use crate::research::ResearchSource;
use crate::x_post::{extract_tweet_id_from_url, XCredentials};
use reqwest::multipart;
use reqwest::Client;
use std::time::Duration;

pub fn author_handle(source: &ResearchSource) -> Option<String> {
    let name = source.source_name.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.trim_start_matches('@').to_string())
}

/// Replace parenthetical handles like (SawyerMerritt) with @SawyerMerritt for known sources.
pub fn normalize_source_mentions(text: &str, sources: &[ResearchSource]) -> String {
    let mut result = text.to_string();
    for source in sources {
        let Some(author) = author_handle(source) else {
            continue;
        };
        let patterns = [
            format!("({})", author),
            format!("({})", author.to_lowercase()),
        ];
        let replacement = format!("@{}", author);
        for pat in patterns {
            if result.contains(&pat) {
                result = result.replace(&pat, &replacement);
            }
        }
    }
    result
}

fn significant_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '$' && c != '.')
                .to_lowercase()
        })
        .filter(|w| w.len() >= 4)
        .collect()
}

fn extract_distinctive_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let lower = text.to_lowercase();
    for word in lower.split_whitespace() {
        let cleaned: String = word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '$' || *c == '.')
            .collect();
        if cleaned.contains('$')
            || cleaned.chars().any(|c| c.is_ascii_digit())
            || (cleaned.len() >= 6 && !["tesla", "spacex", "musk", "grok"].contains(&cleaned.as_str()))
        {
            if cleaned.len() >= 3 {
                tokens.push(cleaned);
            }
        }
    }
    tokens
}

pub fn source_match_score(text: &str, source: &ResearchSource) -> usize {
    let text_lower = text.to_lowercase();
    let haystack = format!(
        "{} {}",
        source.title.to_lowercase(),
        source.content.chars().take(400).collect::<String>().to_lowercase()
    );

    let mut score = 0usize;

    if let Some(author) = author_handle(source) {
        let author_lower = author.to_lowercase();
        if text_lower.contains(&format!("@{}", author_lower)) {
            score += 3;
        }
        if text_lower.contains(&format!("({})", author_lower)) {
            score += 3;
        }
    }

    for token in significant_tokens(&text_lower) {
        if haystack.contains(&token) {
            score += 2;
        }
    }

    for token in extract_distinctive_tokens(&text_lower) {
        if haystack.contains(&token) {
            score += 5;
        }
    }

    score
}

/// Pick the single research source that best matches this draft's text.
pub fn match_primary_source<'a>(
    text: &str,
    sources: &'a [ResearchSource],
) -> Option<&'a ResearchSource> {
    if sources.is_empty() {
        return None;
    }
    if sources.len() == 1 {
        return Some(&sources[0]);
    }

    let mut best: Option<(&ResearchSource, usize)> = None;
    for source in sources {
        let score = source_match_score(text, source);
        if score == 0 {
            continue;
        }
        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((source, score)),
        }
    }

    best.map(|(s, _)| s)
}

pub async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    if crate::draft_image::is_local_image_path(url) {
        return std::fs::read(url).map_err(|e| format!("Failed to read local image: {}", e));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Image download failed ({})", res.status()));
    }

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    if bytes.is_empty() {
        return Err("Downloaded image was empty".to_string());
    }

    Ok(bytes.to_vec())
}

#[derive(Debug, Clone)]
pub struct TweetSourceDetails {
    pub text: String,
    pub author: String,
    pub media_url: Option<String>,
    pub tweet_id: Option<String>,
}

fn photo_url_from_tweet_json(json: &serde_json::Value) -> Option<String> {
    let media = json["includes"]["media"].as_array();
    let keys = json["data"]["attachments"]["media_keys"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if let Some(media_arr) = media {
        for key in keys {
            let key_str = key.as_str().unwrap_or("");
            for item in media_arr {
                if item["media_key"].as_str() == Some(key_str) {
                    let media_type = item["type"].as_str().unwrap_or("");
                    if media_type == "photo" {
                        if let Some(url) = item["url"].as_str() {
                            return Some(url.to_string());
                        }
                        if let Some(url) = item["preview_image_url"].as_str() {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }
        for item in media_arr {
            if item["type"].as_str() == Some("photo") {
                if let Some(url) = item["url"].as_str() {
                    return Some(url.to_string());
                }
            }
        }
    }

    None
}

fn author_from_tweet_json(json: &serde_json::Value) -> String {
    let author_id = json["data"]["author_id"].as_str().unwrap_or("");
    if let Some(users) = json["includes"]["users"].as_array() {
        for user in users {
            if user["id"].as_str() == Some(author_id) {
                if let Some(username) = user["username"].as_str() {
                    return format!("@{username}");
                }
            }
        }
    }
    "@unknown".to_string()
}

async fn fetch_tweet_lookup_json(
    creds: &XCredentials,
    tweet_id: &str,
    fields: &str,
) -> Result<Option<serde_json::Value>, String> {
    let url = format!(
        "https://api.twitter.com/2/tweets/{}?{}",
        tweet_id, fields
    );
    let auth = crate::x_post::oauth_get_header(&url, creds)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(&url)
        .header("Authorization", auth)
        .send()
        .await
        .map_err(|e| format!("Tweet lookup failed: {}", e))?;

    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        log::warn!(
            "fetch_tweet_lookup_json: tweet {} lookup failed ({}): {}",
            tweet_id,
            status,
            body
        );
        return Ok(None);
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid tweet lookup response: {}", e))?;

    Ok(Some(json))
}

pub async fn fetch_tweet_source_details(
    creds: &XCredentials,
    tweet_id: &str,
) -> Result<Option<TweetSourceDetails>, String> {
    let fields = "tweet.fields=text,attachments&expansions=attachments.media_keys,author_id&media.fields=url,preview_image_url,type&user.fields=username";
    let json = fetch_tweet_lookup_json(creds, tweet_id, fields).await?;
    let Some(json) = json else {
        return Ok(None);
    };

    let text = json["data"]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(text) = text else {
        return Ok(None);
    };

    Ok(Some(TweetSourceDetails {
        author: author_from_tweet_json(&json),
        media_url: photo_url_from_tweet_json(&json),
        text,
        tweet_id: Some(tweet_id.to_string()),
    }))
}

pub async fn fetch_tweet_photo_url(
    creds: &XCredentials,
    tweet_id: &str,
) -> Result<Option<String>, String> {
    let fields = "tweet.fields=attachments&expansions=attachments.media_keys&media.fields=url,preview_image_url,type";
    let json = fetch_tweet_lookup_json(creds, tweet_id, fields).await?;
    Ok(json.as_ref().and_then(photo_url_from_tweet_json))
}

pub async fn upload_media_image(creds: &XCredentials, bytes: &[u8]) -> Result<String, String> {
    let url = "https://upload.twitter.com/1.1/media/upload.json";
    let auth = crate::x_post::oauth_post_multipart_header(url, creds)?;

    let part = multipart::Part::bytes(bytes.to_vec())
        .file_name("image.jpg")
        .mime_str("image/jpeg")
        .map_err(|e| e.to_string())?;
    let form = multipart::Form::new().part("media", part);

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(url)
        .header("Authorization", auth)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Media upload failed: {}", e))?;

    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Media upload error ({}): {}", status, body));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid media upload response: {}", e))?;

    json["media_id_string"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No media_id_string in upload response: {}", body))
}

pub fn preview_image_from_source(source: Option<&ResearchSource>) -> Option<String> {
    source
        .and_then(|s| s.media_url.as_ref())
        .filter(|u| !u.is_empty())
        .cloned()
}

pub fn tweet_id_from_source(source: &ResearchSource) -> Option<String> {
    source
        .original_id
        .as_ref()
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
        .cloned()
        .or_else(|| extract_tweet_id_from_url(&source.url))
}

/// Resolve a displayable HTTPS image URL for app preview and posting.
pub async fn resolve_preview_image_url(
    creds: Option<&XCredentials>,
    draft_image_url: Option<&str>,
    primary_source: Option<&ResearchSource>,
) -> Result<Option<String>, String> {
    if let Some(url) = draft_image_url.filter(|u| !u.is_empty()) {
        return Ok(Some(url.to_string()));
    }

    if let Some(url) = preview_image_from_source(primary_source) {
        return Ok(Some(url));
    }

    let Some(source) = primary_source else {
        return Ok(None);
    };

    let Some(tweet_id) = tweet_id_from_source(source) else {
        return Ok(None);
    };

    let Some(creds) = creds else {
        return Ok(None);
    };

    fetch_tweet_photo_url(creds, &tweet_id).await
}

pub async fn resolve_post_media(
    creds: &XCredentials,
    draft_image_url: Option<&str>,
    primary_source: Option<&ResearchSource>,
) -> Result<Option<String>, String> {
    let preview =
        resolve_preview_image_url(Some(creds), draft_image_url, primary_source).await?;

    let Some(url) = preview else {
        return Ok(None);
    };

    let bytes = download_image(&url).await?;
    Ok(Some(upload_media_image(creds, &bytes).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn x_source(author: &str, url: &str) -> ResearchSource {
        ResearchSource {
            id: "1".into(),
            title: "title".into(),
            content: "content".into(),
            url: url.into(),
            published_at: Some(Utc::now()),
            source_name: format!("@{}", author),
            source_type: "x_grok".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: Some("1234567890".into()),
            media_url: None,
            used_at: None,
        }
    }

    #[test]
    fn test_preview_image_from_source() {
        let source = x_source("SawyerMerritt", "https://x.com/SawyerMerritt/status/1");
        let mut with_media = source.clone();
        with_media.media_url = Some("https://pbs.twimg.com/media/abc.jpg".into());
        assert_eq!(
            preview_image_from_source(Some(&with_media)),
            Some("https://pbs.twimg.com/media/abc.jpg".to_string())
        );
        assert_eq!(preview_image_from_source(Some(&source)), None);
    }

    #[test]
    fn test_normalize_source_mentions() {
        let sources = vec![x_source("SawyerMerritt", "https://x.com/SawyerMerritt/status/1")];
        let text = "Maui sale (SawyerMerritt) signals premium solar adoption.";
        let out = normalize_source_mentions(text, &sources);
        assert!(out.contains("@SawyerMerritt"));
        assert!(!out.contains("(SawyerMerritt)"));
    }

    fn mut_source(author: &str, title: &str, content: &str, url: &str) -> ResearchSource {
        ResearchSource {
            title: title.into(),
            content: content.into(),
            url: url.into(),
            original_id: extract_tweet_id_from_url(url),
            ..x_source(author, url)
        }
    }

    #[test]
    fn test_match_primary_source_picks_by_content_not_first() {
        let sources = vec![
            mut_source(
                "SawyerMerritt",
                "Cybercabs in Houston",
                "Tons of Tesla Cybercabs spotted in Houston Texas",
                "https://x.com/SawyerMerritt/status/111",
            ),
            mut_source(
                "SawyerMerritt",
                "Maui $26.5M home",
                "A home in Maui sold for $26.5 million with a $1.4M Tesla Solar Tile Roof",
                "https://x.com/SawyerMerritt/status/222",
            ),
        ];
        let text =
            "Maui's $26.5M home sale with a $1.4M Tesla Solar Tile roof (@SawyerMerritt) signals premium solar adoption.";
        let primary = match_primary_source(text, &sources).unwrap();
        assert_eq!(
            primary.url,
            "https://x.com/SawyerMerritt/status/222"
        );
    }

    #[test]
    fn test_single_source_array_returns_that_source() {
        let sources = vec![x_source("SawyerMerritt", "https://x.com/SawyerMerritt/status/1")];
        let primary = match_primary_source("unrelated text", &sources).unwrap();
        assert_eq!(author_handle(primary), Some("SawyerMerritt".to_string()));
    }
}
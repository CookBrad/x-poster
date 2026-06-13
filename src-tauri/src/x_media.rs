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

pub fn primary_x_source<'a>(text: &str, sources: &'a [ResearchSource]) -> Option<&'a ResearchSource> {
    let x_sources: Vec<&ResearchSource> = sources
        .iter()
        .filter(|s| s.source_type == "x_grok" || s.source_type == "x")
        .collect();

    if x_sources.is_empty() {
        return None;
    }

    let text_lower = text.to_lowercase();
    for source in &x_sources {
        if let Some(author) = author_handle(source) {
            let author_lower = author.to_lowercase();
            if text_lower.contains(&format!("@{}", author_lower))
                || text_lower.contains(&format!("({})", author_lower))
                || text_lower.contains(&author_lower)
            {
                return Some(source);
            }
        }
    }

    x_sources.into_iter().next()
}

pub async fn download_image(url: &str) -> Result<Vec<u8>, String> {
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

pub async fn fetch_tweet_photo_url(
    creds: &XCredentials,
    tweet_id: &str,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://api.twitter.com/2/tweets/{}?tweet.fields=attachments&expansions=attachments.media_keys&media.fields=url,preview_image_url,type",
        tweet_id
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
            "fetch_tweet_photo_url: tweet {} lookup failed ({}): {}",
            tweet_id,
            status,
            body
        );
        return Ok(None);
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid tweet lookup response: {}", e))?;

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
                            return Ok(Some(url.to_string()));
                        }
                        if let Some(url) = item["preview_image_url"].as_str() {
                            return Ok(Some(url.to_string()));
                        }
                    }
                }
            }
        }
        // Fallback: first photo in includes
        for item in media_arr {
            if item["type"].as_str() == Some("photo") {
                if let Some(url) = item["url"].as_str() {
                    return Ok(Some(url.to_string()));
                }
            }
        }
    }

    Ok(None)
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

pub async fn resolve_post_media(
    creds: &XCredentials,
    draft_image_url: Option<&str>,
    primary_source: Option<&ResearchSource>,
) -> Result<Option<String>, String> {
    if let Some(url) = draft_image_url.filter(|u| !u.is_empty()) {
        match download_image(url).await {
            Ok(bytes) => return Ok(Some(upload_media_image(creds, &bytes).await?)),
            Err(e) => log::warn!("resolve_post_media: draft image_url failed: {}", e),
        }
    }

    let Some(source) = primary_source else {
        return Ok(None);
    };

    if let Some(url) = source.media_url.as_ref().filter(|u| !u.is_empty()) {
        match download_image(url).await {
            Ok(bytes) => return Ok(Some(upload_media_image(creds, &bytes).await?)),
            Err(e) => log::warn!("resolve_post_media: source media_url failed: {}", e),
        }
    }

    let tweet_id = source
        .original_id
        .as_ref()
        .filter(|id| id.chars().all(|c| c.is_ascii_digit()))
        .cloned()
        .or_else(|| extract_tweet_id_from_url(&source.url));

    let Some(tweet_id) = tweet_id else {
        return Ok(None);
    };

    let photo_url = fetch_tweet_photo_url(creds, &tweet_id).await?;
    let Some(photo_url) = photo_url else {
        return Ok(None);
    };

    let bytes = download_image(&photo_url).await?;
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
        }
    }

    #[test]
    fn test_normalize_source_mentions() {
        let sources = vec![x_source("SawyerMerritt", "https://x.com/SawyerMerritt/status/1")];
        let text = "Maui sale (SawyerMerritt) signals premium solar adoption.";
        let out = normalize_source_mentions(text, &sources);
        assert!(out.contains("@SawyerMerritt"));
        assert!(!out.contains("(SawyerMerritt)"));
    }

    #[test]
    fn test_primary_x_source_matches_author_in_text() {
        let sources = vec![
            x_source("WholeMarsBlog", "https://x.com/WholeMarsBlog/status/1"),
            x_source("SawyerMerritt", "https://x.com/SawyerMerritt/status/2"),
        ];
        let text = "Maui home sale (SawyerMerritt) validates solar tiles.";
        let primary = primary_x_source(text, &sources).unwrap();
        assert_eq!(author_handle(primary), Some("SawyerMerritt".to_string()));
    }
}
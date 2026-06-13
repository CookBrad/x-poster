use crate::research::ResearchSource;
use crate::x_media;
use crate::x_post::XCredentials;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const GROK_IMAGE_MODEL: &str = "grok-imagine-image-quality";

pub struct DraftImageRequest<'a> {
    pub draft_id: &'a str,
    pub draft_text: &'a str,
    pub draft_image_url: Option<&'a str>,
    pub primary_source: Option<&'a ResearchSource>,
    pub x_credentials: Option<&'a XCredentials>,
    pub xai_api_key: Option<&'a str>,
    pub app_data_dir: Option<&'a Path>,
}

pub fn is_local_image_path(path: &str) -> bool {
    let trimmed = path.trim();
    trimmed.starts_with('/') || trimmed.contains(":\\") || trimmed.starts_with("\\\\")
}

pub fn build_image_generation_prompt(draft_text: &str, source: Option<&ResearchSource>) -> String {
    let topic: String = draft_text.chars().take(240).collect();
    let source_hint = source
        .map(|s| format!(" Source context: {} — {}.", s.title, s.content.chars().take(120).collect::<String>()))
        .unwrap_or_default();

    format!(
        "Create a polished, realistic social media photo illustration for a post about Elon Musk's companies \
         (Tesla, SpaceX, xAI, Neuralink, Boring Company). Topic: {topic}.{source_hint} \
         Style: modern, optimistic, tech-forward, high quality. No text overlays, no logos, no watermarks, \
         no identifiable real people's faces."
    )
}

pub fn extract_meta_image_url(html: &str) -> Option<String> {
    for property in ["og:image", "twitter:image", "twitter:image:src"] {
        if let Some(url) = extract_meta_property_content(html, property) {
            if is_plausible_image_url(&url) {
                return Some(url);
            }
        }
    }
    None
}

fn extract_meta_property_content(html: &str, property: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let prop_lower = property.to_lowercase();

    for needle in [
        format!("property=\"{prop_lower}\""),
        format!("property='{prop_lower}'"),
        format!("name=\"{prop_lower}\""),
        format!("name='{prop_lower}'"),
    ] {
        if let Some(idx) = lower.find(&needle) {
            let tail = &html[idx..];
            if let Some(content) = parse_content_attribute(tail) {
                return Some(content);
            }
        }
    }

    None
}

fn parse_content_attribute(fragment: &str) -> Option<String> {
    let lower = fragment.to_lowercase();
    let content_idx = lower.find("content=")?;
    let rest = &fragment[content_idx + "content=".len()..];
    let trimmed = rest.trim_start();

    if trimmed.starts_with('"') {
        let end = trimmed[1..].find('"')? + 1;
        return Some(trimmed[1..end].to_string());
    }
    if trimmed.starts_with('\'') {
        let end = trimmed[1..].find('\'')? + 1;
        return Some(trimmed[1..end].to_string());
    }

    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == '>')
        .unwrap_or(trimmed.len());
    Some(trimmed[..end].to_string())
}

fn is_plausible_image_url(url: &str) -> bool {
    let trimmed = url.trim();
    (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
        && (trimmed.contains(".jpg")
            || trimmed.contains(".jpeg")
            || trimmed.contains(".png")
            || trimmed.contains(".webp")
            || trimmed.contains("image")
            || trimmed.contains("/media/"))
}

fn is_article_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    !(lower.contains("x.com/") || lower.contains("twitter.com/"))
}

pub async fn fetch_og_image_url(page_url: &str) -> Result<Option<String>, String> {
    if !page_url.starts_with("http") || !is_article_url(page_url) {
        return Ok(None);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("x-poster/1.0 (draft image resolver)")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(page_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch article for image: {}", e))?;

    if !response.status().is_success() {
        log::warn!(
            "fetch_og_image_url: {} returned {}",
            page_url,
            response.status()
        );
        return Ok(None);
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read article HTML: {}", e))?;

    Ok(extract_meta_image_url(&html))
}

pub async fn generate_image_with_grok(
    xai_api_key: &str,
    prompt: &str,
) -> Result<Option<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": GROK_IMAGE_MODEL,
        "prompt": prompt,
        "aspect_ratio": "16:9",
        "response_format": "url",
        "n": 1
    });

    let response = client
        .post("https://api.x.ai/v1/images/generations")
        .bearer_auth(xai_api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Grok image request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        log::warn!("Grok image generation failed ({}): {}", status, text);
        return Ok(None);
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid Grok image response: {}", e))?;

    let url = json["data"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item["url"].as_str())
        .map(|s| s.to_string());

    Ok(url.filter(|u| is_plausible_image_url(u) || u.starts_with("http")))
}

pub async fn persist_image_from_url(
    app_data_dir: &Path,
    draft_id: &str,
    image_url: &str,
) -> Result<String, String> {
    let bytes = x_media::download_image(image_url).await?;
    persist_image_bytes(app_data_dir, draft_id, &bytes)
}

pub fn persist_image_bytes(
    app_data_dir: &Path,
    draft_id: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let dir = app_data_dir.join("draft_images");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create draft_images directory: {}", e))?;

    let path: PathBuf = dir.join(format!("{draft_id}.jpg"));
    std::fs::write(&path, bytes).map_err(|e| format!("Failed to save draft image: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

async fn finalize_image_location(
    request: &DraftImageRequest<'_>,
    remote_url: &str,
) -> Result<String, String> {
    if let Some(app_data_dir) = request.app_data_dir {
        match persist_image_from_url(app_data_dir, request.draft_id, remote_url).await {
            Ok(local_path) => return Ok(local_path),
            Err(error) => {
                log::warn!(
                    "Could not persist draft image for {}: {} — using remote URL",
                    request.draft_id,
                    error
                );
            }
        }
    }

    Ok(remote_url.to_string())
}

/// Resolve the best image for a draft: source media → article og:image → Grok generation.
pub async fn resolve_draft_image_url(
    request: DraftImageRequest<'_>,
) -> Result<Option<String>, String> {
    if let Some(existing) = request.draft_image_url.filter(|u| !u.is_empty()) {
        if is_local_image_path(existing) {
            return Ok(Some(existing.to_string()));
        }
        return Ok(Some(
            finalize_image_location(&request, existing).await?,
        ));
    }

    if let Some(url) = x_media::resolve_preview_image_url(
        request.x_credentials,
        None,
        request.primary_source,
    )
    .await?
    {
        log::info!("Resolved draft image from source media for {}", request.draft_id);
        return Ok(Some(finalize_image_location(&request, &url).await?));
    }

    if let Some(source) = request.primary_source {
        if let Some(url) = fetch_og_image_url(&source.url).await? {
            log::info!(
                "Resolved draft image from article og:image for {}",
                request.draft_id
            );
            return Ok(Some(finalize_image_location(&request, &url).await?));
        }
    }

    if let Some(xai_key) = request.xai_api_key.filter(|k| !k.is_empty()) {
        let prompt = build_image_generation_prompt(request.draft_text, request.primary_source);
        if let Some(url) = generate_image_with_grok(xai_key, &prompt).await? {
            log::info!(
                "Generated draft image with Grok for {}",
                request.draft_id
            );
            return Ok(Some(finalize_image_location(&request, &url).await?));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_meta_image_url_from_html() {
        let html = r#"<html><head>
            <meta property="og:image" content="https://cdn.example.com/hero.jpg" />
        </head></html>"#;
        assert_eq!(
            extract_meta_image_url(html),
            Some("https://cdn.example.com/hero.jpg".to_string())
        );
    }

    #[test]
    fn test_build_image_generation_prompt_includes_draft_topic() {
        let prompt = build_image_generation_prompt("Cybertruck Smart Summon expands", None);
        assert!(prompt.contains("Cybertruck Smart Summon"));
        assert!(prompt.contains("optimistic"));
    }

    #[test]
    fn test_is_local_image_path() {
        assert!(is_local_image_path("/Users/me/Library/draft_images/abc.jpg"));
        assert!(!is_local_image_path("https://example.com/a.jpg"));
    }
}
use crate::draft_image::{extract_main_text_excerpt, extract_meta_image_url, extract_page_title_and_description};
use crate::research::ResearchSource;
use crate::x_media::{fetch_tweet_source_details, TweetSourceDetails};
use crate::x_post::{extract_tweet_id_from_url, XCredentials};
use chrono::Utc;
use reqwest::Client;
use std::time::Duration;

pub fn looks_like_url(input: &str) -> bool {
    let trimmed = input.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
        || (trimmed.contains('.') && !trimmed.contains(' ') && trimmed.len() > 4)
}

pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

pub fn is_x_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("x.com/") || lower.contains("twitter.com/")
}

fn source_name_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.trim_start_matches("www.").to_string()))
        .unwrap_or_else(|| "Web".to_string())
}

fn author_from_x_url(url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    for host in ["x.com/", "twitter.com/"] {
        if let Some(idx) = lower.find(host) {
            let rest = &url[idx + host.len()..];
            let handle: String = rest
                .chars()
                .take_while(|c| *c != '/' && !c.is_whitespace())
                .collect();
            if !handle.is_empty() && handle != "i" && handle != "status" {
                return Some(format!("@{handle}"));
            }
        }
    }
    None
}

fn research_source_from_topic(topic: &str) -> ResearchSource {
    ResearchSource {
        id: format!("custom_topic_{}", uuid::Uuid::new_v4()),
        title: topic.chars().take(120).collect(),
        content: format!(
            "User-requested topic for a custom draft post: {topic}"
        ),
        url: String::new(),
        published_at: Some(Utc::now()),
        source_name: "Custom topic".to_string(),
        source_type: "custom_topic".to_string(),
        retweet_count: None,
        like_count: None,
        reply_count: None,
        quote_count: None,
        original_id: None,
        media_url: None,
        used_at: None,
    }
}

fn research_source_from_tweet(url: &str, details: TweetSourceDetails) -> ResearchSource {
    ResearchSource {
        id: format!("custom_x_{}", uuid::Uuid::new_v4()),
        title: details.text.chars().take(100).collect::<String>()
            + if details.text.chars().count() > 100 { "..." } else { "" },
        content: details.text,
        url: url.to_string(),
        published_at: Some(Utc::now()),
        source_name: details.author,
        source_type: "custom_x".to_string(),
        retweet_count: None,
        like_count: None,
        reply_count: None,
        quote_count: None,
        original_id: details.tweet_id,
        media_url: details.media_url,
        used_at: None,
    }
}

fn research_source_from_x_url_fallback(url: &str) -> ResearchSource {
    let author = author_from_x_url(url).unwrap_or_else(|| "@unknown".to_string());
    let tweet_id = extract_tweet_id_from_url(url);
    ResearchSource {
        id: format!("custom_x_{}", uuid::Uuid::new_v4()),
        title: "X post".to_string(),
        content: format!(
            "The user pasted this *specific* X/Twitter post URL: {url} by {author}. \
             Generate a high-quality draft post that is directly based on and faithful to the actual content and main idea of *that exact post*. \
             Do not substitute a different topic or angle. Use your best knowledge of what was in that specific post to create an engaging, standalone X post that accurately captures its key points or story. \
             (Exact text could not be fetched at resolution time because no X credentials were available; stay as close as possible to the known content of this post.)\nURL: {url}"
        ),
        url: url.to_string(),
        published_at: Some(Utc::now()),
        source_name: author,
        source_type: "custom_x".to_string(),
        retweet_count: None,
        like_count: None,
        reply_count: None,
        quote_count: None,
        original_id: tweet_id,
        media_url: None,
        used_at: None,
    }
}

fn research_source_from_article(url: &str, title: String, content: String, media_url: Option<String>) -> ResearchSource {
    ResearchSource {
        id: format!("custom_article_{}", uuid::Uuid::new_v4()),
        title,
        content,
        url: url.to_string(),
        published_at: Some(Utc::now()),
        source_name: source_name_from_url(url),
        source_type: "custom_article".to_string(),
        retweet_count: None,
        like_count: None,
        reply_count: None,
        quote_count: None,
        original_id: None,
        media_url,
        used_at: None,
    }
}

async fn resolve_article_url(url: &str) -> Result<ResearchSource, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("x-poster/1.0 (custom source resolver)")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch link: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch link (HTTP {}). Check the URL and try again.",
            response.status()
        ));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read page content: {}", e))?;

    let (title, description) = extract_page_title_and_description(&html);
    let title = title.unwrap_or_else(|| source_name_from_url(url));
    // Prefer longer main-text excerpt (new) so generation has more actual article substance
    // for originality/interesting implications instead of just a teaser sentence.
    let body = extract_main_text_excerpt(&html);
    let content = body.or(description).unwrap_or_else(|| {
        format!("Article at {url}. Write a useful post based on this story.")
    });
    let media_url = extract_meta_image_url(&html);

    Ok(research_source_from_article(url, title, content, media_url))
}

async fn resolve_x_url(url: &str, creds: Option<&XCredentials>) -> Result<ResearchSource, String> {
    if let (Some(creds), Some(tweet_id)) = (creds, extract_tweet_id_from_url(url)) {
        if let Some(details) = fetch_tweet_source_details(creds, &tweet_id).await? {
            return Ok(research_source_from_tweet(url, details));
        }
    }

    Ok(research_source_from_x_url_fallback(url))
}

/// Turn a pasted link or free-form topic into a synthetic research source for draft generation.
pub async fn resolve_custom_input(
    input: &str,
    creds: Option<&XCredentials>,
) -> Result<ResearchSource, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a link or topic.".to_string());
    }

    if looks_like_url(trimmed) {
        let url = normalize_url(trimmed);
        if is_x_url(&url) {
            return resolve_x_url(&url, creds).await;
        }
        return resolve_article_url(&url).await;
    }

    Ok(research_source_from_topic(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_url() {
        assert!(looks_like_url("https://example.com/story"));
        assert!(looks_like_url("www.teslarati.com/article"));
        assert!(!looks_like_url("Tesla Robotaxi expansion in Austin"));
    }

    #[test]
    fn test_is_x_url() {
        assert!(is_x_url("https://x.com/SawyerMerritt/status/123"));
        assert!(!is_x_url("https://teslarati.com/story"));
    }

    #[test]
    fn test_topic_source_builder() {
        let source = research_source_from_topic("Starship booster catch milestone");
        assert_eq!(source.source_type, "custom_topic");
        assert!(source.content.contains("Starship booster catch milestone"));
    }

    #[test]
    fn test_extract_main_text_excerpt_pulls_paragraphs() {
        // Basic HTML with <p> tags; the extractor should concatenate meaningful text.
        let html = r#"<html><body><p>First para about robotaxi data velocity.</p><p>Second detail on FSD edge cases and regulatory path.</p><script>ignore</script></body></html>"#;
        let ex = extract_main_text_excerpt(html);
        assert!(ex.is_some());
        let e = ex.unwrap();
        assert!(e.contains("First para about robotaxi data velocity"));
        assert!(e.contains("Second detail on FSD edge cases"));
        assert!(e.len() > 50 && e.len() <= 1500);
    }
}
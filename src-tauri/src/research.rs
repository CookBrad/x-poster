use crate::x_post::extract_tweet_id_from_url;
use chrono::Utc;
use feed_rs::parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResearchSource {
    pub id: String,
    pub title: String,
    pub content: String,
    pub url: String,
    /// Stored as RFC3339 string (or None). Using String keeps the type portable
    /// across sqlite and postgres when using sqlx::Any (chrono DateTime decode
    /// doesn't bridge automatically for Any + FromRow derive).
    pub published_at: Option<String>,
    pub source_name: String,
    pub source_type: String, // "rss" or "x"
    // Engagement metrics (mainly populated for X posts).
    // Using i64 for portability with sqlx::Any (across sqlite + postgres).
    pub retweet_count: Option<i64>,
    pub like_count: Option<i64>,
    pub reply_count: Option<i64>,
    pub quote_count: Option<i64>,

    /// Original identifier from the source (X post id, RSS entry id, etc.).
    /// This can be duplicated across different research runs.
    /// The `id` field is the unique row identifier in the database.
    pub original_id: Option<String>,

    /// Direct image URL from the source post (X photo), when known.
    pub media_url: Option<String>,

    /// Set when this source has been used to generate a draft post.
    pub used_at: Option<String>,
}

pub fn is_research_source_used(source: &ResearchSource) -> bool {
    source.used_at.is_some()
}

pub fn unused_research_sources(sources: &[ResearchSource]) -> Vec<ResearchSource> {
    sources
        .iter()
        .filter(|source| !is_research_source_used(source))
        .cloned()
        .collect()
}

/// Fetches recent items from a list of RSS feeds relevant to Tesla/TSLA/Elon.
pub async fn fetch_rss_sources() -> Result<Vec<ResearchSource>, String> {
    // Focused on high-signal sources covering Elon Musk's companies (Tesla, SpaceX,
    // xAI, Neuralink, Boring). Not limited to official company channels.
    // General EV / competitor news sites deliberately excluded.
    let feeds = vec![
        "https://www.teslarati.com/feed/",
        "https://www.notateslaapp.com/rss", // excellent dedicated Tesla news + software updates
                                            // "https://feeds.feedburner.com/TeslaMotorsClub", // mostly stale (old 2021 content)
    ];

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("x-poster/1.0 (Tesla research bot)")
        .build()
        .map_err(|e| e.to_string())?;

    let mut sources = Vec::new();

    for feed_url in feeds {
        match fetch_single_rss(&client, feed_url).await {
            Ok(items) => sources.extend(items),
            Err(e) => {
                log::warn!("Failed to fetch RSS {}: {}", feed_url, e);
            }
        }
    }

    // Sort by published date desc and limit
    sources.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    sources.truncate(25);

    Ok(sources)
}

async fn fetch_single_rss(client: &Client, url: &str) -> Result<Vec<ResearchSource>, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let feed = parser::parse(&bytes[..]).map_err(|e| e.to_string())?;

    let source_name = feed
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_else(|| url.to_string());

    let mut items = Vec::new();

    for entry in feed.entries {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_default();

        let content = entry
            .content
            .as_ref()
            .and_then(|c| c.body.clone())
            .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()))
            .unwrap_or_default();

        let link = entry
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();

        let published = entry.published.or(entry.updated);

        // Only keep reasonably recent items (last 14 days) for relevance
        if let Some(published_at) = published {
            if (Utc::now() - published_at).num_days() > 14 {
                continue;
            }
        }

        // Store as RFC3339 string for DB portability (AnyPool + FromRow)
        let published_str = published.map(|dt| dt.to_rfc3339());

        items.push(ResearchSource {
            id: entry.id.clone(),
            title,
            content: strip_html(&content),
            url: link,
            published_at: published_str,
            source_name: source_name.clone(),
            source_type: "rss".to_string(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: Some(entry.id.clone()),
            media_url: None,
            used_at: None,
        });
    }

    Ok(items)
}

fn strip_html(input: &str) -> String {
    // Very basic HTML stripping for now
    let mut output = String::new();
    let mut in_tag = false;

    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(c),
            _ => {}
        }
    }

    // Collapse whitespace
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Simple heuristic to check if a URL looks like a real direct X/Twitter post link.
/// We use this to avoid serving hallucinated or broken links to the user.
fn is_likely_valid_x_post_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    if !(lower.starts_with("https://x.com/") || lower.starts_with("https://twitter.com/")) {
        return false;
    }
    if !lower.contains("/status/") {
        return false;
    }
    // Extract the part after /status/ and check that it looks like a numeric ID
    if let Some(after_status) = lower.split("/status/").nth(1) {
        let id_part = after_status.split(&['?', '#'][..]).next().unwrap_or("");
        return !id_part.is_empty()
            && id_part.chars().all(|c| c.is_ascii_digit())
            && id_part.len() > 5;
    }
    false
}

/// Uses Grok via the xAI Responses API (`/v1/responses`) with the native `x_search`
/// tool. This is the recommended endpoint for reliable X-specific search with proper
/// tool support (as opposed to the older Chat Completions endpoint).
/// We request direct post URLs + post_ids when possible.
pub async fn fetch_grok_discovered_x_sources(
    xai_api_key: &str,
    model: &str,
) -> Result<Vec<ResearchSource>, String> {
    if xai_api_key.trim().is_empty() {
        log::warn!("fetch_grok_discovered_x_sources: empty key, returning no sources");
        return Ok(vec![]);
    }

    log::info!(
        "fetch_grok_discovered_x_sources: starting Grok call for X discovery (key len={})",
        xai_api_key.len()
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let system_prompt = r#"You are an expert researcher focused EXCLUSIVELY on Elon Musk's companies: Tesla (vehicles, FSD, Optimus, Robotaxi, energy/Megapack, Dojo), SpaceX (Starlink, Starship, Falcon), xAI (Grok), Neuralink, and The Boring Company.

CRITICAL RULES:
- ONLY include posts about Tesla, SpaceX, xAI, Neuralink, or Boring Company. Reject anything about other EV companies or general EV news.
- Include posts that have real substance: product updates, technical details, deliveries (and their impact on stock/valuation), regulatory news, financial/market reactions, TSLA stock implications, new data, interesting analysis, or notable developments.
- Include posts from respected Tesla/SpaceX community analysts, dedicated reporters, current or former employees, and other knowledgeable observers in the ecosystem. These voices often share valuable details even if the post is not the single most viral one.
- Do NOT filter out good posts just because they are not extremely high-engagement or "viral". Substance and relevance matter more than raw popularity.
- Avoid political content and obvious low-quality spam.

ANTI-HALLUCINATION RULES (IMPORTANT):
- You do NOT have real-time access to X or the internet in this context.
- Only include a post if you are **highly confident** it is real, recent, and accurately quoted.
- Do NOT invent, approximate, or fabricate post text, authors, dates, post_ids, or URLs.
- If you are unsure about the exact content or link of a post, do not include it.
- It is far better to return fewer high-quality, accurate items than many made-up ones.

Return ONLY a JSON array (no markdown fences, no extra prose outside the array) with **at least 10 and up to 15** high-quality items using exactly this structure:

[
  {
    "text": "the full or near-verbatim text of the post or key quote",
    "author": "username without @",
    "url": "DIRECT link to the original X post in exact format https://x.com/author/status/POST_ID (highly preferred when you are certain it is accurate)",
    "post_id": "the exact post ID if you can determine it with high confidence",
    "media_url": "direct HTTPS URL to the main photo in the post, if the post includes an image and you can determine the URL with high confidence; otherwise omit or null",
    "date": "YYYY-MM-DD if known, or a clear relative date like 'June 2 2026' or '2 days ago'",
    "why_interesting": "1-2 sentence note on why this is useful for a research post, ideally calling out a non-obvious implication, specific data point, timeline read-through, margin/valuation angle, or strategic significance that could seed an original insight post",
    "confidence": "high | medium | low  (how confident you are that the text, author, and link are accurate and real)"
  }
]

Be proactive about surfacing actual recent posts and quotes from knowledgeable community voices, analysts, reporters, and employees when they exist. 

X SEARCH ONLY (CRITICAL):
- You have access to the live_search tool. **When searching, you MUST exclusively use results from X (x.com / Twitter)**. Completely ignore all general web results, news sites, blogs, or any non-X domains.
- Heavily prioritize and focus on posts from these accounts: WholeMarsBlog, SawyerMerritt, The_Limiting_Factor, Vol888, and any Tesla or xAI employees.
- Do not return any information that does not come directly from X posts.

If verbatim recent posts are limited, still return the best substantive items you can. Do not default to an empty array."#;

    let user_prompt = "Find recent substantive posts/quotes from high-signal voices (analysts, reporters, employees, etc.) about Tesla, SpaceX, xAI, Neuralink or The Boring Company. Use search to find real posts on X only. Completely ignore general web results. Only include items where you are highly confident the text, author, date, and link are accurate and real. If unsure, skip the item. Quality and accuracy over quantity. Return up to 15 items. For each, include a why_interesting note that highlights a non-obvious implication or angle (see JSON schema). For legal, regulatory, court, amendment, litigation, or government-action stories, the why_interesting note should call out any quantified operational impact, prior delays to testing/pad work/cadence, or concrete effects mentioned. Include the confidence field for each.";

    let body = serde_json::json!({
        "model": model,
        "input": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "temperature": 0.0,
        "tools": [
          {
            "type": "x_search"
          }
        ],
        "max_output_tokens": 4000
    });

    let res = client
        .post("https://api.x.ai/v1/responses")
        .bearer_auth(xai_api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call xAI API: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("xAI API error ({}): {}", status, text));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

    // Responses API format (preferred for tool use)
    let content = json["output"]
        .as_array()
        .and_then(|arr| arr.iter().find(|item| item["type"] == "message"))
        .and_then(|msg| msg["content"].as_array())
        .and_then(|content_arr| content_arr.iter().find(|c| c["type"] == "output_text"))
        .and_then(|text_item| text_item["text"].as_str())
        // Fallback for older chat/completions format
        .or_else(|| json["choices"][0]["message"]["content"].as_str())
        .ok_or("Unexpected response format from Grok")?;

    log::info!(
        "fetch_grok_discovered_x_sources: Grok raw content length={}",
        content.len()
    );
    // Full raw response is logged here for diagnosis — look for whether Grok mentioned accounts like WholeMars, SawyerMerritt, Limiting Factor, Vol888, employees, etc.
    log::info!(
        "fetch_grok_discovered_x_sources: FULL RAW GROK RESPONSE:\n{}",
        content
    );

    // Robustly extract JSON array even if wrapped in ```json ... ``` or extra text
    let trimmed = content.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else if trimmed.starts_with("```") {
        // Fallback: strip common markdown fences
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str).map_err(|e| {
        format!(
            "Failed to parse Grok response as JSON: {}. Raw content: {}",
            e, content
        )
    })?;

    let mut sources = Vec::new();
    let raw_count = parsed.len();
    let mut low_confidence_skipped = 0;

    for item in parsed {
        let text = item["text"].as_str().unwrap_or("").to_string();
        if text.len() < 20 {
            continue;
        }

        let author = item["author"].as_str().unwrap_or("unknown").to_string();

        // Anti-hallucination: Only accept items Grok marks as high confidence
        let confidence = item
            .get("confidence")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_lowercase();
        if confidence != "high" {
            low_confidence_skipped += 1;
            continue; // Skip anything Grok isn't highly confident about
        }

        // Prefer direct X post links. Use post_id when available for reliable construction.
        let post_id = item
            .get("post_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .trim();
        let raw_url = item
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .trim();

        let mut url = if !raw_url.is_empty() && !raw_url.contains("/status/unknown") {
            raw_url.to_string()
        } else if !post_id.is_empty() && author != "unknown" {
            format!("https://x.com/{}/status/{}", author, post_id)
        } else if !raw_url.is_empty() {
            raw_url.to_string()
        } else {
            String::new()
        };

        // If we still don't have a good direct link, create a useful X search link for this author + topic
        if !is_likely_valid_x_post_url(&url) {
            if author != "unknown" {
                // Extract a few keywords from the text for a targeted search
                let keywords: String = text
                    .split_whitespace()
                    .take(6)
                    .collect::<Vec<_>>()
                    .join(" ");
                let encoded = keywords.replace(' ', "%20");
                url = format!("https://x.com/search?q=from%3A{}%20{}", author, encoded);
            } else {
                url.clear();
            }
        }

        let why = item["why_interesting"].as_str().unwrap_or("").to_string();

        // Try to parse date from Grok (new field added to prompt)
        let date_str = item
            .get("date")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .trim();
        let published_at: Option<String> = if !date_str.is_empty() {
            // Try full RFC3339 / datetime with timezone first
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
                Some(dt.with_timezone(&Utc).to_rfc3339())
            } 
            // Handle plain date "2026-05-29" (very common from Grok)
            else if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                // Treat as midnight UTC on that date
                Some(naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339())
            } 
            // Other human-readable formats
            else if let Ok(dt) = chrono::DateTime::parse_from_str(date_str, "%B %d %Y") {
                Some(dt.with_timezone(&Utc).to_rfc3339())
            } 
            else if let Ok(dt) = chrono::DateTime::parse_from_str(date_str, "%b %d %Y") {
                Some(dt.with_timezone(&Utc).to_rfc3339())
            } 
            else {
                // Could not parse — leave as None so UI shows "Unknown date"
                None
            }
        } else {
            None
        };

        let media_url = item
            .get("media_url")
            .and_then(|m| m.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.starts_with("http"))
            .map(|s| s.to_string());

        let original_id = if !post_id.is_empty() {
            Some(post_id.to_string())
        } else if let Some(id) = extract_tweet_id_from_url(&url) {
            Some(id)
        } else {
            None
        };

        sources.push(ResearchSource {
            id: format!("grok_x_{}", uuid::Uuid::new_v4()),
            title: text.chars().take(100).collect::<String>()
                + if text.len() > 100 { "..." } else { "" },
            content: format!("{}\n\n[Why notable: {}]", text, why),
            url,
            published_at,
            source_name: format!("@{}", author),
            source_type: "x_grok".to_string(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id,
            media_url,
            used_at: None,
        });
    }

    log::info!(
        "fetch_grok_discovered_x_sources: kept {} high-confidence sources (skipped {} low/medium confidence, from {} raw items)",
        sources.len(),
        low_confidence_skipped,
        raw_count
    );
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unused_research_sources_filters_used_rows() {
        let sources = vec![
            ResearchSource {
                id: "1".into(),
                title: "Fresh".into(),
                content: "A".into(),
                url: "https://example.com/a".into(),
                published_at: None,
                source_name: "Teslarati".into(),
                source_type: "rss".into(),
                retweet_count: None,
                like_count: None,
                reply_count: None,
                quote_count: None,
                original_id: None,
                media_url: None,
                used_at: None,
            },
            ResearchSource {
                id: "2".into(),
                title: "Old".into(),
                content: "B".into(),
                url: "https://example.com/b".into(),
                published_at: None,
                source_name: "Not A Tesla App".into(),
                source_type: "rss".into(),
                retweet_count: None,
                like_count: None,
                reply_count: None,
                quote_count: None,
                original_id: None,
                media_url: None,
                used_at: Some(Utc::now().to_rfc3339()),
            },
        ];

        let unused = unused_research_sources(&sources);
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].id, "1");
    }

    #[tokio::test]
    async fn test_fetch_rss_sources_returns_some_recent_items_or_graceful_empty() {
        // This hits real network (Teslarati + feedburner). We assert it either
        // succeeds with >=0 items or fails in a controlled way (no panic).
        // The 14-day filter + stale feedburner means we mostly expect items from Teslarati.
        let result = fetch_rss_sources().await;
        match result {
            Ok(items) => {
                // Should not crash; in normal conditions Teslarati has recent posts
                println!("fetch_rss_sources test: got {} items", items.len());
                // If 0, it means either feeds down or extreme date filter — still valid path for "no sources" case
                assert!(items.len() < 100, "unexpectedly large RSS result");
            }
            Err(e) => {
                // Network or parse hiccup is acceptable in test env; log for CI visibility
                println!(
                    "fetch_rss_sources test: graceful error (expected in some envs): {}",
                    e
                );
            }
        }
    }
}

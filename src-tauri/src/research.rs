use crate::x_post::extract_tweet_id_from_url;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use feed_rs::parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Hard max age for research subjects: a few days. Older stories are dropped.
pub const RESEARCH_MAX_AGE_HOURS: i64 = 72;
/// Prefer subjects from the last ~day-and-a-half (hours-old when possible).
pub const RESEARCH_PREFERRED_AGE_HOURS: i64 = 36;

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

/// Parse a published_at string into UTC. Supports RFC3339, plain dates, and
/// common relative phrases Grok returns ("2 hours ago", "yesterday", etc.).
pub fn parse_published_at(raw: &str) -> Option<DateTime<Utc>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return naive_date
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc());
    }
    if let Ok(dt) = DateTime::parse_from_str(raw, "%B %d %Y") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_str(raw, "%b %d %Y") {
        return Some(dt.with_timezone(&Utc));
    }

    parse_relative_published_at(raw, Utc::now())
}

/// Relative date phrases relative to `now` (testable).
pub fn parse_relative_published_at(raw: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let lower = raw.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    if lower == "just now" || lower == "now" || lower == "moments ago" {
        return Some(now);
    }
    if lower == "today" {
        return Some(now);
    }
    if lower == "yesterday" {
        return Some(now - ChronoDuration::days(1));
    }

    // "N hour(s) ago" / "Nh ago" / "N hr ago"
    if let Some(hours) = parse_relative_count(&lower, &["hours ago", "hour ago", "hrs ago", "hr ago", "h ago"]) {
        return Some(now - ChronoDuration::hours(hours));
    }
    // "N minute(s) ago" / "Nm ago"
    if let Some(mins) = parse_relative_count(&lower, &["minutes ago", "minute ago", "mins ago", "min ago", "m ago"]) {
        return Some(now - ChronoDuration::minutes(mins));
    }
    // "N day(s) ago" / "Nd ago"
    if let Some(days) = parse_relative_count(&lower, &["days ago", "day ago", "d ago"]) {
        return Some(now - ChronoDuration::days(days));
    }

    None
}

fn parse_relative_count(lower: &str, suffixes: &[&str]) -> Option<i64> {
    for suffix in suffixes {
        if let Some(prefix) = lower.strip_suffix(suffix) {
            let num_str = prefix.trim();
            // allow forms like "2" or "about 2"
            let digits: String = num_str
                .split_whitespace()
                .last()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = digits.parse::<i64>() {
                if n >= 0 {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Age of a source in whole hours, if `published_at` is parseable.
pub fn source_age_hours(source: &ResearchSource, now: DateTime<Utc>) -> Option<i64> {
    let raw = source.published_at.as_deref()?;
    let published = parse_published_at(raw)?;
    let age = now.signed_duration_since(published);
    // Future timestamps (clock skew / bad parse): treat as brand new (0 hours).
    if age.num_seconds() < 0 {
        return Some(0);
    }
    Some(age.num_hours())
}

/// True when the source is within the hard max age, or has no parseable date
/// (undated items are kept but ranked last — RSS/X sometimes omit timestamps).
pub fn source_is_within_max_age(source: &ResearchSource, now: DateTime<Utc>) -> bool {
    match source_age_hours(source, now) {
        Some(hours) => hours <= RESEARCH_MAX_AGE_HOURS,
        None => true,
    }
}

/// True when the source is in the preferred hours-old window.
pub fn source_is_preferred_fresh(source: &ResearchSource, now: DateTime<Utc>) -> bool {
    match source_age_hours(source, now) {
        Some(hours) => hours <= RESEARCH_PREFERRED_AGE_HOURS,
        None => false,
    }
}

/// Drop sources older than RESEARCH_MAX_AGE_HOURS, then rank: preferred hours-old
/// first, then by published_at desc, undated last.
pub fn filter_and_rank_recent_sources(
    sources: Vec<ResearchSource>,
    now: DateTime<Utc>,
) -> Vec<ResearchSource> {
    let mut kept: Vec<ResearchSource> = sources
        .into_iter()
        .filter(|s| source_is_within_max_age(s, now))
        .collect();

    kept.sort_by(|a, b| {
        let a_pref = source_is_preferred_fresh(a, now);
        let b_pref = source_is_preferred_fresh(b, now);
        match (a_pref, b_pref) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        // Newer first among same preference band (smaller age hours = fresher)
        let a_age = source_age_hours(a, now);
        let b_age = source_age_hours(b, now);
        match (a_age, b_age) {
            (Some(aa), Some(bb)) => aa.cmp(&bb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.published_at.cmp(&a.published_at),
        }
    });

    kept
}

/// Unused sources that still pass the recency bar, ranked freshest first.
pub fn unused_recent_research_sources(
    sources: &[ResearchSource],
    now: DateTime<Utc>,
) -> Vec<ResearchSource> {
    filter_and_rank_recent_sources(unused_research_sources(sources), now)
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

    // Prefer hours-old, keep only within max age, limit
    let mut sources = filter_and_rank_recent_sources(sources, Utc::now());
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

        // Hard recency bar: prefer hours-old, max RESEARCH_MAX_AGE_HOURS (a few days).
        if let Some(published_at) = published {
            let age_hours = (Utc::now() - published_at).num_hours();
            if age_hours > RESEARCH_MAX_AGE_HOURS {
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

    // Prefer hours-old items; drop anything that somehow exceeded the max age.
    Ok(filter_and_rank_recent_sources(items, Utc::now()))
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

    let system_prompt = format!(
        r#"You are an expert researcher focused EXCLUSIVELY on Elon Musk's companies: Tesla (vehicles, FSD, Optimus, Robotaxi, energy/Megapack, Dojo), SpaceX (Starlink, Starship, Falcon), xAI (Grok), Neuralink, and The Boring Company.

CRITICAL RULES:
- ONLY include posts about Tesla, SpaceX, xAI, Neuralink, or Boring Company. Reject anything about other EV companies or general EV news.
- Include posts that have real substance: product updates, technical details, deliveries (and their impact on stock/valuation), regulatory news, financial/market reactions, TSLA stock implications, new data, interesting analysis, or notable developments.
- Include posts from respected Tesla/SpaceX community analysts, dedicated reporters, current or former employees, and other knowledgeable observers in the ecosystem. These voices often share valuable details even if the post is not the single most viral one.
- Do NOT filter out good posts just because they are not extremely high-engagement or "viral". Substance and relevance matter more than raw popularity.
- Avoid political content and obvious low-quality spam.

RECENCY (MANDATORY — subjects must be fresh):
- Prefer posts from the last few **hours** when available (breaking / same-day news).
- Hard max: only include posts from the last {max_hours} hours (~{max_days} days). Reject week-old or older items.
- Prefer items under ~{pref_hours} hours old over multi-day-old items when both exist.
- Every item MUST include an accurate `date` (prefer ISO datetime or "N hours ago" / "YYYY-MM-DD").

ANTI-HALLUCINATION RULES (IMPORTANT):
- Only include a post if you are **highly confident** it is real, recent, and accurately quoted.
- Do NOT invent, approximate, or fabricate post text, authors, dates, post_ids, or URLs.
- If you are unsure about the exact content or link of a post, do not include it.
- It is far better to return fewer high-quality, accurate items than many made-up ones.

Return ONLY a JSON array (no markdown fences, no extra prose outside the array) with **at least 10 and up to 15** high-quality items using exactly this structure:

[
  {{
    "text": "the full or near-verbatim text of the post or key quote",
    "author": "username without @",
    "url": "DIRECT link to the original X post in exact format https://x.com/author/status/POST_ID (highly preferred when you are certain it is accurate)",
    "post_id": "the exact post ID if you can determine it with high confidence",
    "media_url": "direct HTTPS URL to the main photo in the post, if the post includes an image and you can determine the URL with high confidence; otherwise omit or null",
    "date": "prefer RFC3339 or 'N hours ago' / 'YYYY-MM-DD' / 'N days ago' — must reflect real post time",
    "why_interesting": "1-2 sentence note on why this is useful for a research post, ideally calling out a non-obvious implication, specific data point, timeline read-through, margin/valuation angle, or strategic significance that could seed an original insight post",
    "confidence": "high | medium | low  (how confident you are that the text, author, and link are accurate and real)"
  }}
]

Be proactive about surfacing actual recent (hours-old preferred) posts and quotes from knowledgeable community voices, analysts, reporters, and employees when they exist.

X SEARCH ONLY (CRITICAL):
- You have access to the x_search / live search tool. **When searching, you MUST exclusively use results from X (x.com / Twitter)**. Completely ignore all general web results, news sites, blogs, or any non-X domains.
- Bias search toward the latest posts (last hours to last 1-2 days), not evergreen or historical threads.
- Heavily prioritize and focus on posts from these accounts: WholeMarsBlog, SawyerMerritt, The_Limiting_Factor, Vol888, and any Tesla or xAI employees.
- Do not return any information that does not come directly from X posts.

If verbatim hours-old posts are limited, still return the best substantive items from the last {max_days} days. Do not default to an empty array. Never include posts older than {max_hours} hours."#,
        max_hours = RESEARCH_MAX_AGE_HOURS,
        max_days = RESEARCH_MAX_AGE_HOURS / 24,
        pref_hours = RESEARCH_PREFERRED_AGE_HOURS,
    );

    let user_prompt = format!(
        "Find the most recent substantive posts/quotes (prefer last few hours; max last {max_hours} hours / ~{max_days} days) from high-signal voices (analysts, reporters, employees, etc.) about Tesla, SpaceX, xAI, Neuralink or The Boring Company. Use search to find real posts on X only. Completely ignore general web results. Only include items where you are highly confident the text, author, date, and link are accurate and real. Skip anything older than {max_hours} hours. Prefer hours-old breaking/same-day items over multi-day-old coverage. Quality and accuracy over quantity. Return up to 15 items. For each, include an accurate date and a why_interesting note that highlights a non-obvious implication or angle (see JSON schema). For legal, regulatory, court, amendment, litigation, or government-action stories, the why_interesting note should call out any quantified operational impact, prior delays to testing/pad work/cadence, or concrete effects mentioned. Include the confidence field for each.",
        max_hours = RESEARCH_MAX_AGE_HOURS,
        max_days = RESEARCH_MAX_AGE_HOURS / 24,
    );

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

        // Parse date from Grok (RFC3339, YYYY-MM-DD, relative "N hours ago", etc.)
        let date_str = item
            .get("date")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .trim();
        let published_at: Option<String> = if !date_str.is_empty() {
            parse_published_at(date_str).map(|dt| dt.to_rfc3339())
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

    let before_recency = sources.len();
    let sources = filter_and_rank_recent_sources(sources, Utc::now());
    let dropped_stale = before_recency.saturating_sub(sources.len());

    log::info!(
        "fetch_grok_discovered_x_sources: kept {} high-confidence recent sources (skipped {} low/medium confidence, dropped {} older than {}h, from {} raw items)",
        sources.len(),
        low_confidence_skipped,
        dropped_stale,
        RESEARCH_MAX_AGE_HOURS,
        raw_count
    );
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_source(id: &str, published_at: Option<String>, used_at: Option<String>) -> ResearchSource {
        ResearchSource {
            id: id.into(),
            title: format!("Title {id}"),
            content: "content".into(),
            url: format!("https://example.com/{id}"),
            published_at,
            source_name: "Teslarati".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
            used_at,
        }
    }

    #[test]
    fn test_unused_research_sources_filters_used_rows() {
        let sources = vec![
            sample_source("1", None, None),
            sample_source("2", None, Some(Utc::now().to_rfc3339())),
        ];

        let unused = unused_research_sources(&sources);
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].id, "1");
    }

    #[test]
    fn test_parse_published_at_rfc3339_and_plain_date() {
        let rfc = parse_published_at("2026-07-13T10:00:00Z").expect("rfc");
        assert_eq!(rfc.to_rfc3339(), "2026-07-13T10:00:00+00:00");

        let plain = parse_published_at("2026-07-12").expect("plain");
        assert_eq!(plain.date_naive().to_string(), "2026-07-12");
    }

    #[test]
    fn test_parse_relative_published_at_hours_and_days() {
        let now = DateTime::parse_from_rfc3339("2026-07-13T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let two_hours = parse_relative_published_at("2 hours ago", now).expect("2h");
        assert_eq!((now - two_hours).num_hours(), 2);

        let five_h = parse_relative_published_at("5h ago", now).expect("5h");
        assert_eq!((now - five_h).num_hours(), 5);

        let yesterday = parse_relative_published_at("yesterday", now).expect("yesterday");
        assert_eq!((now - yesterday).num_days(), 1);

        let two_days = parse_relative_published_at("2 days ago", now).expect("2d");
        assert_eq!((now - two_days).num_days(), 2);

        let just_now = parse_relative_published_at("just now", now).expect("now");
        assert_eq!(just_now, now);
    }

    #[test]
    fn test_filter_and_rank_recent_sources_drops_stale_prefers_hours_old() {
        let now = DateTime::parse_from_rfc3339("2026-07-13T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let hours_old = sample_source(
            "hours",
            Some((now - ChronoDuration::hours(6)).to_rfc3339()),
            None,
        );
        let days_old_ok = sample_source(
            "days_ok",
            Some((now - ChronoDuration::hours(48)).to_rfc3339()),
            None,
        );
        let too_old = sample_source(
            "stale",
            Some((now - ChronoDuration::hours(RESEARCH_MAX_AGE_HOURS + 12)).to_rfc3339()),
            None,
        );
        let undated = sample_source("undated", None, None);

        let ranked = filter_and_rank_recent_sources(
            vec![too_old, days_old_ok, undated, hours_old],
            now,
        );

        // Stale dropped
        assert!(ranked.iter().all(|s| s.id != "stale"));
        assert_eq!(ranked.len(), 3);
        // Preferred hours-old first
        assert_eq!(ranked[0].id, "hours");
        // Preferred band: hours (6h) before days_ok (48h) which may still be preferred if <=36...
        // 48h > PREFERRED 36 → not preferred; undated not preferred.
        // Order: hours (pref) then days_ok (younger than undated's unknown) then undated
        assert_eq!(ranked[1].id, "days_ok");
        assert_eq!(ranked[2].id, "undated");

        assert!(source_is_preferred_fresh(&ranked[0], now));
        assert!(!source_is_preferred_fresh(&ranked[1], now));
        assert!(source_is_within_max_age(&ranked[1], now));
        assert!(!source_is_within_max_age(
            &sample_source(
                "stale2",
                Some((now - ChronoDuration::hours(RESEARCH_MAX_AGE_HOURS + 1)).to_rfc3339()),
                None
            ),
            now
        ));
    }

    #[test]
    fn test_unused_recent_research_sources_combines_unused_and_recency() {
        let now = Utc::now();
        let fresh_unused = sample_source(
            "fresh",
            Some((now - ChronoDuration::hours(3)).to_rfc3339()),
            None,
        );
        let stale_unused = sample_source(
            "stale",
            Some((now - ChronoDuration::hours(RESEARCH_MAX_AGE_HOURS + 5)).to_rfc3339()),
            None,
        );
        let fresh_used = sample_source(
            "used",
            Some((now - ChronoDuration::hours(2)).to_rfc3339()),
            Some(now.to_rfc3339()),
        );

        let result = unused_recent_research_sources(
            &[fresh_unused, stale_unused, fresh_used],
            now,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "fresh");
    }

    #[test]
    fn test_recency_constants_prefer_hours_over_week() {
        // Subjects should be days-max, hours-preferred — not week-old news.
        assert!(RESEARCH_MAX_AGE_HOURS <= 72 * 2); // at most ~6 days hard cap in spirit
        assert!(RESEARCH_MAX_AGE_HOURS >= 24);
        assert!(RESEARCH_PREFERRED_AGE_HOURS < RESEARCH_MAX_AGE_HOURS);
        assert!(RESEARCH_PREFERRED_AGE_HOURS <= 48);
    }

    #[tokio::test]
    async fn test_fetch_rss_sources_returns_some_recent_items_or_graceful_empty() {
        // Hits real network. Assert controlled success/empty under hours/days recency bar.
        let result = fetch_rss_sources().await;
        match result {
            Ok(items) => {
                println!("fetch_rss_sources test: got {} items", items.len());
                assert!(items.len() < 100, "unexpectedly large RSS result");
                let now = Utc::now();
                for item in &items {
                    if let Some(age) = source_age_hours(item, now) {
                        assert!(
                            age <= RESEARCH_MAX_AGE_HOURS,
                            "RSS item {} age {}h exceeds max {}h",
                            item.id,
                            age,
                            RESEARCH_MAX_AGE_HOURS
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "fetch_rss_sources test: graceful error (expected in some envs): {}",
                    e
                );
            }
        }
    }
}

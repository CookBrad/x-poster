use chrono::{DateTime, Utc};
use feed_rs::parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub id: String,
    pub title: String,
    pub content: String,
    pub url: String,
    pub published_at: Option<DateTime<Utc>>,
    pub source_name: String,
    pub source_type: String, // "rss" or "x"
}

/// Fetches recent items from a list of RSS feeds relevant to Tesla/TSLA/Elon.
pub async fn fetch_rss_sources() -> Result<Vec<ResearchSource>, String> {
    let feeds = vec![
        "https://www.teslarati.com/feed/",
        "https://insideevs.com/feed/",
        "https://feeds.feedburner.com/TeslaMotorsClub",
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
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

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

        items.push(ResearchSource {
            id: entry.id.clone(),
            title,
            content: strip_html(&content),
            url: link,
            published_at: published,
            source_name: source_name.clone(),
            source_type: "rss".to_string(),
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

/// Fetches recent posts from X using the provided Bearer Token.
/// Query example: "(Tesla OR TSLA OR Cybertruck OR Optimus) -is:retweet lang:en"
pub async fn fetch_x_sources(
    bearer_token: &str,
    query: &str,
) -> Result<Vec<ResearchSource>, String> {
    if bearer_token.trim().is_empty() {
        return Ok(vec![]); // No token configured yet
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("x-poster/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.twitter.com/2/tweets/search/recent";

    let response = client
        .get(url)
        .bearer_auth(bearer_token)
        .query(&[
            ("query", query),
            ("max_results", "10"),
            ("tweet.fields", "created_at,public_metrics"),
            ("expansions", "author_id"),
            ("user.fields", "username"),
        ])
        .send()
        .await
        .map_err(|e| format!("X API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("X API error ({}): {}", status, text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse X response: {}", e))?;

    let mut sources = Vec::new();

    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        let empty_users = vec![];
        let users: &Vec<serde_json::Value> = json
            .get("includes")
            .and_then(|i| i.get("users"))
            .and_then(|u| u.as_array())
            .unwrap_or(&empty_users);

        let user_map: std::collections::HashMap<String, String> = users
            .iter()
            .filter_map(|u| {
                let id = u.get("id")?.as_str()?.to_string();
                let username = u.get("username")?.as_str()?.to_string();
                Some((id, username))
            })
            .collect();

        for tweet in data {
            let id = tweet.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let created_at = tweet
                .get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let author_id = tweet.get("author_id").and_then(|v| v.as_str()).unwrap_or("");
            let username = user_map.get(author_id).cloned().unwrap_or_else(|| "unknown".to_string());

            sources.push(ResearchSource {
                id: format!("x_{}", id),
                title: text.chars().take(80).collect::<String>() + if text.len() > 80 { "..." } else { "" },
                content: text,
                url: format!("https://x.com/{}/status/{}", username, id),
                published_at: created_at,
                source_name: format!("@{}", username),
                source_type: "x".to_string(),
            });
        }
    }

    Ok(sources)
}
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
        "https://electrek.co/feed/",
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

// Placeholder for future X research
pub async fn fetch_x_sources(_query: &str) -> Result<Vec<ResearchSource>, String> {
    // TODO: Implement X API search (keyword + semantic) using stored credentials
    Ok(vec![])
}
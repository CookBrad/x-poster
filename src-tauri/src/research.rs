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
    // Engagement metrics (mainly populated for X posts)
    pub retweet_count: Option<u32>,
    pub like_count: Option<u32>,
    pub reply_count: Option<u32>,
    pub quote_count: Option<u32>,
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
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
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
/// Uses Grok (via xAI API) to discover high-signal, trending, or interesting
/// Tesla/Elon-related posts on X. This is now the primary method for X content
/// because raw keyword search produces too much noise.
pub async fn fetch_grok_discovered_x_sources(xai_api_key: &str) -> Result<Vec<ResearchSource>, String> {
    if xai_api_key.trim().is_empty() {
        return Ok(vec![]);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let system_prompt = r#"You are an expert researcher focused on Tesla, SpaceX, Elon Musk's companies, and related technology (FSD, Optimus, Cybertruck, Robotaxi, energy, etc.).

Your job is to find the most substantive, high-signal, and interesting recent posts on X (Twitter) about these topics.

Rules (strict):
- Only include posts that offer real information, analysis, implications, or novel angles.
- Strongly prefer "fresh takes" over simple news reposts or hype.
- Avoid low-quality spam, memes without substance, political content, or obvious engagement bait.
- Prioritize posts from credible or high-signal accounts when possible.
- Focus on company/technology developments rather than pure stock price movement unless there's significant analysis.

Return ONLY a JSON array of objects with this exact structure (no extra text):

[
  {
    "text": "the full post text",
    "author": "username (without @)",
    "url": "https://x.com/username/status/1234567890",
    "why_interesting": "1-2 sentence explanation of why this post is notable or worth turning into original commentary"
  }
]

If you cannot find any high-quality posts, return an empty array []."#;

    let user_prompt = "Find the most interesting and substantive recent posts (last 48-72 hours) about Tesla, its products, technology, or Elon Musk's related companies on X. Focus on high-signal content.";

    let body = serde_json::json!({
        "model": "grok-3",  // or grok-3-mini if we want cheaper/faster
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 4000
    });

    let res = client
        .post("https://api.x.ai/v1/chat/completions")
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

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Unexpected response format from Grok")?;

    // Try to extract JSON from the response (Grok sometimes wraps it in markdown)
    let json_str = if let Some(start) = content.find('[') {
        if let Some(end) = content.rfind(']') {
            &content[start..=end]
        } else {
            content
        }
    } else {
        content
    };

    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse Grok response as JSON: {}. Raw: {}", e, content))?;

    let mut sources = Vec::new();

    for item in parsed {
        let text = item["text"].as_str().unwrap_or("").to_string();
        if text.len() < 20 { continue; }

        let author = item["author"].as_str().unwrap_or("unknown").to_string();
        let url = item.get("url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("https://x.com/{}/status/unknown", author));

        let why = item["why_interesting"].as_str().unwrap_or("").to_string();

        sources.push(ResearchSource {
            id: format!("grok_x_{}", uuid::Uuid::new_v4()),
            title: text.chars().take(100).collect::<String>() + if text.len() > 100 { "..." } else { "" },
            content: format!("{}\n\n[Why notable: {}]", text, why),
            url,
            published_at: None, // Grok doesn't always give exact timestamps
            source_name: format!("@{}", author),
            source_type: "x_grok".to_string(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
        });
    }

    Ok(sources)
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
            ("max_results", "100"),
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

            // Extract engagement metrics
            let (retweet_count, like_count, reply_count, quote_count) = if let Some(metrics) = tweet.get("public_metrics") {
                (
                    metrics.get("retweet_count").and_then(|v| v.as_u64()).map(|v| v as u32),
                    metrics.get("like_count").and_then(|v| v.as_u64()).map(|v| v as u32),
                    metrics.get("reply_count").and_then(|v| v.as_u64()).map(|v| v as u32),
                    metrics.get("quote_count").and_then(|v| v.as_u64()).map(|v| v as u32),
                )
            } else {
                (None, None, None, None)
            };

            sources.push(ResearchSource {
                id: format!("x_{}", id),
                title: text.chars().take(80).collect::<String>() + if text.len() > 80 { "..." } else { "" },
                content: text,
                url: format!("https://x.com/{}/status/{}", username, id),
                published_at: created_at,
                source_name: format!("@{}", username),
                source_type: "x".to_string(),
                retweet_count,
                like_count,
                reply_count,
                quote_count,
            });
        }
    }

    // Sort by popularity (engagement) instead of recency
    sources.sort_by(|a, b| {
        let score_a = a.retweet_count.unwrap_or(0) * 3
            + a.like_count.unwrap_or(0)
            + a.reply_count.unwrap_or(0)
            + a.quote_count.unwrap_or(0);
        let score_b = b.retweet_count.unwrap_or(0) * 3
            + b.like_count.unwrap_or(0)
            + b.reply_count.unwrap_or(0)
            + b.quote_count.unwrap_or(0);
        score_b.cmp(&score_a)  // descending popularity
    });

    Ok(sources)
}
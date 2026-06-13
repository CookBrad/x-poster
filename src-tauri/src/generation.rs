use crate::commands::{create_draft_db, get_drafts_db, CreateDraftInput, Draft};
use crate::research::ResearchSource;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDraftItem {
    pub text: String,
    pub rationale: Option<String>,
    pub primary_author: Option<String>,
    pub primary_source_index: Option<u32>,
}

/// Build system prompt enforcing insight, attribution, bullish framing, and stock tags.
pub fn build_generation_system_prompt() -> &'static str {
    r#"You are an expert social media analyst writing posts for a human who covers Elon Musk's companies (Tesla, SpaceX, xAI, Neuralink, Boring Company).

CRITICAL RULES:
1. USEFUL INSIGHT REQUIRED — NOT REGURGITATION:
   - Every post must add value beyond the source headline: implications, second-order effects, what bulls/bears miss, competitive context, timeline read-through, margin/capital angle, or strategic significance.
   - Do NOT restate, summarize, or closely paraphrase the source. If someone could read the source title and learn the same thing from your post, it fails.
   - Do NOT write empty hype or press-release filler.

2. CONSTRUCTIVELY BULLISH FRAMING (DEFAULT):
   - Lean positive on Elon and his companies: highlight execution strengths, innovation, strategic upside, and why developments are meaningful when grounded in facts.
   - Avoid bearish pile-on, doom narratives, cynical dunking, or "Tesla is doomed" angles.
   - You may acknowledge risks, but the net framing should be optimistic about these companies' trajectories and leadership.

3. STOCK TAGS WHEN MARKET-RELEVANT:
   - If the post relates to Tesla as a public company (deliveries, earnings, valuation, stock reaction, FSD/Robotaxi as TSLA catalysts, energy business, etc.), include the cashtag $TSLA in the post text.
   - SpaceX, xAI, Neuralink, and Boring Company are private — do not invent tickers for them.
   - Place $TSLA naturally (often at the end). Do not spam multiple cashtags.

4. INLINE ATTRIBUTION:
   - When you use a specific fact from a source, attribute it inline with an @ mention (e.g. "As @SawyerMerritt noted..." or "Per @Teslarati...").
   - Never use parenthetical handles like (SawyerMerritt).

5. Avoid repeating themes from the user's RECENT POSTS list below — find a different angle.
6. Non-political, company/tech focus only. No partisan takes.
7. Each post must be under 280 characters (single tweet). Count $TSLA toward the limit.

GOOD: "Per @Teslarati, Austin Robotaxi geofence widened again — the read-through for $TSLA isn't the headline, it's faster real-world miles accruing toward regulatory confidence on unsupervised FSD."
BAD (regurgitation): "Teslarati reports Tesla expanded Robotaxi in Austin." (just repeats the source)
BAD (no insight): "Tesla is doing great things with FSD!" (empty hype)
BAD (bearish): "Another Robotaxi delay — Tesla keeps overpromising." (negative framing we don't want)

Return ONLY a JSON array (no markdown fences), each object:
{
  "text": "the tweet/post text (include $TSLA when Tesla/stock-relevant)",
  "rationale": "1 sentence on what useful insight you added beyond the source",
  "primary_author": "username without @ for the main source this draft draws from, or null for RSS-only",
  "primary_source_index": 3
}

`primary_source_index` is REQUIRED: the 1-based number from the Sources list above that this draft mainly draws from. Each draft must use a different index when possible."#
}

pub fn build_generation_user_prompt(
    sources: &[ResearchSource],
    recent_posted_texts: &[String],
    count: u32,
) -> String {
    let mut source_lines = Vec::new();
    for (i, s) in sources.iter().take(20).enumerate() {
        let excerpt: String = s.content.chars().take(400).collect();
        source_lines.push(format!(
            "{}. [{}] {} (@{}) — {}\n   URL: {}",
            i + 1,
            s.source_type,
            s.title,
            s.source_name.trim_start_matches('@'),
            excerpt,
            s.url
        ));
    }

    let recent = if recent_posted_texts.is_empty() {
        "(none)".to_string()
    } else {
        recent_posted_texts
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Generate exactly {} draft post(s) from these research sources.\n\n\
         Requirements for each draft:\n\
         - Add genuine insight (implications, read-through, what the market or observers miss) — never just repeat the source.\n\
         - Frame constructively and bullishly toward Elon and his companies while staying factual.\n\
         - Include $TSLA when the topic is Tesla/stock/market relevant.\n\n\
         ## Sources\n{}\n\n\
         ## User's recent posted drafts (DO NOT repeat these angles)\n{}\n",
        count,
        source_lines.join("\n"),
        recent
    )
}

const TESLA_STOCK_TAG: &str = "$TSLA";

/// Whether this draft topic warrants a Tesla cashtag based on post text and linked sources.
pub fn relates_to_tesla_stock(text: &str, sources: &[ResearchSource]) -> bool {
    let mut haystack = text.to_lowercase();
    for source in sources {
        haystack.push(' ');
        haystack.push_str(&source.title.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&source.content.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&source.source_name.to_lowercase());
    }

    const TESLA_SIGNALS: &[&str] = &[
        "tesla",
        "tsla",
        "cybertruck",
        "fsd",
        "robotaxi",
        "megapack",
        "optimus",
        "gigafactory",
        "supercharger",
        "elon musk",
        "delivery",
        "deliveries",
        "earnings",
        "valuation",
        "stock",
        "shares",
        "market cap",
    ];

    TESLA_SIGNALS.iter().any(|signal| haystack.contains(signal))
}

/// Append $TSLA when Tesla-related and not already present, respecting the 280-char limit.
pub fn ensure_stock_tag(text: &str, tag: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let upper = trimmed.to_uppercase();
    if upper.contains(tag) || upper.contains("$TSLA") {
        return trimmed.to_string();
    }

    let with_tag = format!("{trimmed} {tag}");
    if with_tag.len() <= 280 {
        with_tag
    } else {
        trimmed.to_string()
    }
}

pub fn finalize_draft_text(text: &str, sources: &[ResearchSource]) -> String {
    let normalized = text.trim().to_string();
    if relates_to_tesla_stock(&normalized, sources) {
        ensure_stock_tag(&normalized, TESLA_STOCK_TAG)
    } else {
        normalized
    }
}

pub fn parse_generated_drafts(content: &str) -> Result<Vec<GeneratedDraftItem>, String> {
    let trimmed = content.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse Grok JSON: {}", e))?;

    let mut items = Vec::new();
    for v in parsed {
        let text = v["text"].as_str().unwrap_or("").trim().to_string();
        if text.len() < 10 {
            continue;
        }
        items.push(GeneratedDraftItem {
            text,
            rationale: v["rationale"].as_str().map(|s| s.to_string()),
            primary_author: v["primary_author"]
                .as_str()
                .map(|s| s.trim().trim_start_matches('@').to_string())
                .filter(|s| !s.is_empty()),
            primary_source_index: v["primary_source_index"]
                .as_u64()
                .map(|n| n as u32)
                .or_else(|| v["primary_source_index"].as_str().and_then(|s| s.parse().ok())),
        });
    }

    if items.is_empty() {
        return Err("Grok returned no usable draft posts.".to_string());
    }

    Ok(items)
}

pub async fn call_grok_for_drafts(
    xai_api_key: &str,
    model: &str,
    sources: &[ResearchSource],
    recent_posted_texts: &[String],
    count: u32,
) -> Result<Vec<GeneratedDraftItem>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "input": [
            {"role": "system", "content": build_generation_system_prompt()},
            {"role": "user", "content": build_generation_user_prompt(sources, recent_posted_texts, count)}
        ],
        "temperature": 0.7,
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

    let content = json["output"]
        .as_array()
        .and_then(|arr| arr.iter().find(|item| item["type"] == "message"))
        .and_then(|msg| msg["content"].as_array())
        .and_then(|content_arr| content_arr.iter().find(|c| c["type"] == "output_text"))
        .and_then(|text_item| text_item["text"].as_str())
        .or_else(|| json["choices"][0]["message"]["content"].as_str())
        .ok_or("Unexpected response format from Grok during draft generation")?;

    parse_generated_drafts(content)
}

fn pick_sources_for_draft(
    item: &GeneratedDraftItem,
    all_sources: &[ResearchSource],
) -> Vec<ResearchSource> {
    if let Some(idx) = item.primary_source_index {
        let i = idx as usize;
        if i >= 1 && i <= all_sources.len() {
            return vec![all_sources[i - 1].clone()];
        }
    }

    if let Some(matched) = crate::x_media::match_primary_source(&item.text, all_sources) {
        return vec![matched.clone()];
    }

    if let Some(author) = &item.primary_author {
        let narrowed: Vec<ResearchSource> = all_sources
            .iter()
            .filter(|s| {
                s.source_name
                    .trim_start_matches('@')
                    .eq_ignore_ascii_case(author)
            })
            .cloned()
            .collect();
        if !narrowed.is_empty() {
            if let Some(matched) = crate::x_media::match_primary_source(&item.text, &narrowed) {
                return vec![matched.clone()];
            }
            if narrowed.len() == 1 {
                return narrowed;
            }
        }
    }

    vec![]
}

pub async fn generate_drafts_from_sources_db(
    db: &SqlitePool,
    sources: &[ResearchSource],
    xai_api_key: &str,
    model: &str,
    count: u32,
) -> Result<Vec<Draft>, String> {
    let recent = get_drafts_db(db, Some("posted".to_string())).await?;
    let recent_texts: Vec<String> = recent
        .into_iter()
        .take(8)
        .map(|d| d.text)
        .collect();

    let generated = call_grok_for_drafts(xai_api_key, model, sources, &recent_texts, count).await?;

    let mut drafts = Vec::new();
    for item in generated {
        let draft_sources = pick_sources_for_draft(&item, sources);
        let finalized = finalize_draft_text(&item.text, &draft_sources);
        let text = crate::x_media::normalize_source_mentions(&finalized, &draft_sources);
        let primary = crate::x_media::match_primary_source(&text, &draft_sources);
        let image_url = primary.and_then(|s| s.media_url.clone());

        let sources_json =
            serde_json::to_string(&draft_sources).map_err(|e| e.to_string())?;

        let input = CreateDraftInput {
            text,
            sources_json,
            image_url,
        };
        let draft = create_draft_db(db, input).await?;
        drafts.push(draft);
    }

    Ok(drafts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_generated_drafts_json() {
        let raw = r#"```json
[{"text": "As @Tesla hinted, energy storage attach rates matter more than deliveries this quarter — bears miss the margin story.", "rationale": "Connects delivery news to margin angle"}]
```"#;
        let items = parse_generated_drafts(raw).expect("parse");
        assert_eq!(items.len(), 1);
        assert!(items[0].text.contains("@Tesla"));
    }

    #[test]
    fn test_system_prompt_requires_insight_and_stock_tags() {
        let prompt = build_generation_system_prompt();
        assert!(prompt.contains("USEFUL INSIGHT"));
        assert!(prompt.contains("$TSLA"));
        assert!(prompt.contains("BULLISH"));
        assert!(prompt.contains("NOT REGURGITATION"));
    }

    #[test]
    fn test_relates_to_tesla_stock_detects_tesla_topics() {
        assert!(relates_to_tesla_stock(
            "Robotaxi miles are the real catalyst here",
            &[]
        ));
        assert!(!relates_to_tesla_stock(
            "Starship static fire completed successfully",
            &[]
        ));
    }

    #[test]
    fn test_ensure_stock_tag_appends_tsla_when_missing() {
        let text = "As @Teslarati noted, energy attach rates are accelerating";
        let result = ensure_stock_tag(text, TESLA_STOCK_TAG);
        assert!(result.ends_with("$TSLA"));
    }

    #[test]
    fn test_ensure_stock_tag_skips_when_already_present() {
        let text = "Delivery beat matters for $TSLA margin story";
        assert_eq!(ensure_stock_tag(text, TESLA_STOCK_TAG), text);
    }

    #[test]
    fn test_finalize_draft_text_adds_tag_for_tesla_content() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Tesla Q2 deliveries".into(),
            content: "Beat estimates".into(),
            url: "https://example.com".into(),
            published_at: None,
            source_name: "Teslarati".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
        }];
        let result = finalize_draft_text(
            "Per @Teslarati, deliveries beat — the insight is energy margin mix, not the headline number",
            &sources,
        );
        assert!(result.contains("$TSLA"));
    }

    #[test]
    fn test_build_user_prompt_includes_recent() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Robotaxi update".into(),
            content: "Details here".into(),
            url: "https://example.com".into(),
            published_at: None,
            source_name: "Teslarati".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
        }];
        let prompt = build_generation_user_prompt(&sources, &["Old post about Cybertruck".into()], 2);
        assert!(prompt.contains("Robotaxi"));
        assert!(prompt.contains("Cybertruck"));
        assert!(prompt.contains("exactly 2"));
        assert!(prompt.contains("$TSLA"));
        assert!(prompt.contains("insight"));
    }
}
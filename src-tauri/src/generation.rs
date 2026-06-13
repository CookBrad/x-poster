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
}

/// Build system prompt enforcing fresh-take + inline attribution (T-005 / T-015).
pub fn build_generation_system_prompt() -> &'static str {
    r#"You are an expert social media analyst writing posts for a human who covers Elon Musk's companies (Tesla, SpaceX, xAI, Neuralink, Boring Company).

CRITICAL RULES:
1. FRESH TAKE REQUIRED: Provide original analysis, implications, connections, or a novel angle. Do NOT restate or closely paraphrase what sources already said. Do NOT write generic hype.
2. INLINE ATTRIBUTION: When you use a specific fact from a source, attribute it inline with an @ mention (e.g. "As @SawyerMerritt noted..." or "Per @Teslarati..."). Never use parenthetical handles like (SawyerMerritt).
3. Avoid repeating themes from the user's RECENT POSTS list below — find a different angle.
4. Non-political, company/tech focus only. No partisan takes.
5. Each post must be under 280 characters unless clearly marked needs_thread (we prefer single tweets under 280).
6. Tone: informed, concise, human — not press-release bland.

GOOD example: "Teslarati flagged Robotaxi geofence expansion in Austin — the interesting bit is what this implies for FSD v13 validation timelines, not the headline itself."
BAD example: "Tesla is doing great things with FSD!" (no fresh take, no attribution)

Return ONLY a JSON array (no markdown fences), each object:
{
  "text": "the tweet/post text",
  "rationale": "1 sentence on what fresh angle you added",
  "primary_author": "username without @ for the main X source this draft draws from, or null for RSS-only"
}"#
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
        "Generate exactly {} draft post(s) from these research sources.\n\n## Sources\n{}\n\n## User's recent posted drafts (DO NOT repeat these angles)\n{}\n",
        count,
        source_lines.join("\n"),
        recent
    )
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

    let sources_json = serde_json::to_string(sources).map_err(|e| e.to_string())?;

    let mut drafts = Vec::new();
    for item in generated {
        let text = crate::x_media::normalize_source_mentions(&item.text, sources);
        let image_url = item
            .primary_author
            .as_ref()
            .and_then(|author| {
                sources.iter().find(|s| {
                    s.source_name
                        .trim_start_matches('@')
                        .eq_ignore_ascii_case(author)
                })
            })
            .and_then(|s| s.media_url.clone());

        let input = CreateDraftInput {
            text,
            sources_json: sources_json.clone(),
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
    }
}
use crate::commands::{
    create_draft_db, get_drafts_db, update_draft_db, CreateDraftInput, Draft, UpdateDraftInput,
};
use crate::constants::DraftStyle;
use crate::draft_image;
use crate::research::ResearchSource;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDraftItem {
    pub text: String,
    pub rationale: Option<String>,
    pub primary_author: Option<String>,
    pub primary_source_index: Option<u32>,
}

pub fn is_custom_source(source: &ResearchSource) -> bool {
    source.source_type.starts_with("custom_")
}

pub fn sources_are_user_provided(sources: &[ResearchSource]) -> bool {
    !sources.is_empty() && sources.iter().all(is_custom_source)
}

fn shared_generation_rules(user_provided: bool) -> &'static str {
    if user_provided {
        r#"SHARED RULES (user-requested link or topic):
- MANDATORY: You MUST always generate a post from the provided source. Never refuse, return empty output, or explain why you cannot write it.
- The user intentionally pasted this link or topic — cover what the source is actually about, including political, controversial, or non-Musk-company content.
- Stay factual and on-topic to the source. Do not force a Tesla/SpaceX angle when the source is unrelated.
- When the source is about Musk companies, lean constructively bullish. Otherwise use neutral, informative framing.
- STOCK TAGS WHEN MARKET-RELEVANT (ONE CASHTAG MAX):
  - Include at most ONE cashtag per post — never use both $TSLA and $SPCX together.
  - Only add $TSLA or $SPCX when the source topic is genuinely stock-relevant; skip cashtags for unrelated political posts.
- ATTRIBUTION POLICY + SUPPORTING FACTS FROM KNOWLEDGE + STANDALONE STORY STRUCTURE:
  - Only attribute with "As @Handle noted..." (for X/custom_x) or "source: Publication Name" (for articles) when the specific claim, quote, data point, or recent development comes directly from the provided source AND is not generally known public information.
  - DO NOT quote or attribute the source for generally known or established facts (e.g. "Tesla makes electric vehicles", "FSD is Tesla's driver assistance technology", basic company background, widely understood industry context, or timeless technical explanations). State these plainly as supporting facts drawn from general knowledge.
  - You MAY (and should) add relevant supporting facts, background, or context from your pre-trained general knowledge to make the post more interesting and self-contained. Never fabricate recent events, specific numbers, quotes, or time-sensitive developments not present in the source.
  - The post must read as a **self-contained independent story or analysis**, not a reply. A reader who has never seen the input link or topic must still fully understand the situation.
  - When the insight references an external event or context, briefly establish what it was using details from the source (or clearly note it is general knowledge).
  - Ground the insight with concrete facts/numbers where available.
  - Never use parenthetical handles like (SawyerMerritt).
- Avoid repeating themes from the user's RECENT POSTS list below.
- Each post must be under 280 characters (single tweet). Count cashtags toward the limit."#
    } else {
        r#"SHARED RULES (all styles):
- CONSTRUCTIVELY BULLISH FRAMING: lean positive on Elon and his companies when grounded in facts. No doom narratives or cynical dunking.
- STOCK TAGS WHEN MARKET-RELEVANT (ONE CASHTAG MAX):
  - Include at most ONE cashtag per post — never use both $TSLA and $SPCX together.
  - Tesla topics: use $TSLA. SpaceX topics: use $SPCX. Pick the single tag that best matches the main focus.
  - Do NOT add $SPCX to Tesla-only posts. xAI, Neuralink, and Boring Company have no standard cashtag.
- ATTRIBUTION POLICY + SUPPORTING FACTS FROM KNOWLEDGE + STANDALONE STORY STRUCTURE:
  - Only attribute with "As @Handle noted..." (for X/x_grok/custom_x) or "source: Publication Name" (for RSS/news) when the specific claim, quote, data point, or recent development comes directly from the provided source AND is not generally known public information.
  - DO NOT quote or attribute the source for generally known or established facts (e.g. "Tesla makes electric vehicles", "FSD is Tesla's driver assistance technology", basic company background, widely understood industry context, or timeless technical explanations). State these plainly as supporting facts drawn from general knowledge.
  - You MAY (and should, to make posts more interesting and informative) add relevant supporting facts, background, or context from your pre-trained general knowledge. These enhance the main insight from the source without shifting focus away from it. Never fabricate recent events, specific numbers, quotes, or time-sensitive developments not present in the source.
  - The post must read as a **self-contained independent story or analysis**, not a reply or hot take. A reader who has never seen the source article or the news event it references must still fully understand the situation and find the take valuable.
  - When the insight references or contrasts with an external event/rating/analyst note (e.g. "Moody's rating overlooks..."), *explicitly establish what that event or rating actually was* according to the source (name the specific rating action, what they said or overlooked, etc.). Do not assume the reader knows the Moody's announcement or the article.
  - Ground the insight with 2-3 *concrete, specific supporting facts or numbers* drawn directly from the provided source excerpt (cash/debt/profit figures, any capex or ramp mentions, etc.). Include the details; do not just allude to them.
  - Weave in 1 short, relevant supporting fact or piece of context from your general knowledge where it makes the implication clearer and more credible (e.g. Tesla's history of equity raises during capex-heavy phases, rough scale of upfront investment needed for new programs like autonomy before they are cash-flow positive). Clearly signal it is not from the current source.
  - Never use parenthetical handles like (SawyerMerritt).
- Avoid repeating themes from the user's RECENT POSTS list below.
- Prefer Musk company/tech angles. If a source is political, still write the post about that source rather than refusing.
- Each post must be under 280 characters (single tweet). Count cashtags toward the limit."#
    }
}

fn insight_style_rules() -> &'static str {
    r#"STYLE: INSIGHT (default)
1. USEFUL INSIGHT REQUIRED — NOT REGURGITATION:
   - Every post must add value beyond the source headline: implications, second-order effects, what bulls/bears miss, competitive context, timeline read-through, margin/capital angle, or strategic significance.
   - Do NOT restate, summarize, or closely paraphrase the source.
   - Do NOT write empty hype or press-release filler.
   - Transform the facts into a novel observation or prediction that feels like it comes from someone who has followed the story for months — specific, non-generic, worth screenshotting or quoting in replies.

2. ANTI-PHRASING RULE:
   - Never reuse verbs, sentence structures, or headline phrasing directly from the source or title. Re-express the core implication in fresh language.

3. STRUCTURE FOR STANDALONE INSIGHT POSTS (not replies):
   - Write as a self-contained mini-story or analysis. A reader who has never seen the source or the news must still understand the full situation.
   - Briefly set the scene with the key facts/situation from the source (specific numbers, what the external event/rating actually was).
   - Deliver the fresh insight/implication.
   - Optionally weave in 1 short supporting fact from general knowledge (historical context, scale of the programs, etc.) that makes the "why this matters" clearer.
   - Use attribution only for non-general, source-specific claims.

GOOD (Tesla/X, general fact without attribution + specific with): "FSD development has long relied on accumulating diverse real-world miles for regulatory progress. As @SawyerMerritt noted, Austin Robotaxi geofence widened again — the read-through for $TSLA isn't the headline, it's faster real-world miles accruing toward regulatory confidence on unsupervised FSD."
GOOD (RSS): "Per source: Not A Tesla App, Smart Summon on Cybertruck widens the real-world edge-case pool $TSLA needs before robotaxi scale — the product story is data velocity, not the feature checkbox."
GOOD (deeper originality, supporting fact added): "As @WholeMarsBlog posted on Cybertruck FSD, the real signal is the data loop back to Dojo training — each additional mile in unsupervised mode compounds the software moat faster than any hardware ramp, shifting the margin mix story from cars to bits $TSLA. (Supporting context: autonomy software already shows dramatically higher gross margins than vehicle hardware in Tesla's business model.)"
GOOD (standalone financial story with context + facts + support — RSS + external rating like Moody's): "Tesla holds roughly $40B in cash with zero net debt and steady profits. This balance sheet gives the company real room to self-fund the massive upfront investment needed for Optimus production and unsupervised robotaxi operations without issuing new shares that would dilute existing owners — a structural advantage that Moody's [specific rating action, e.g. 'Baa1 affirmation with stable outlook while flagging execution risks'] appears to underweight. (Tesla has raised equity multiple times during prior heavy-capex growth phases.) $TSLA"
BAD (regurgitation): "Teslarati reports Tesla expanded Robotaxi in Austin." (just repeats the source)
BAD (shallow): "Robotaxi expansion is interesting for the company and its stock." (no specific implication or re-expression)
BAD (over-attribution + no context): "As @SawyerMerritt noted, FSD is Tesla's Full Self-Driving system." (attributes common knowledge; also fails to set any scene)
BAD (reply-like, assumes reader knows the external event): "Tesla's $40B cash, zero debt, and steady profits create headroom to self-fund Optimus and robotaxi ramps without dilution, a structural advantage Moody's rating overlooks. $TSLA" (no explanation of what Moody's actually did or said; no grounding details; feels like a direct comment on the news + article)"#
}

fn informative_style_rules() -> &'static str {
    r#"STYLE: INFORMATIVE
- Share the news clearly and concisely — what happened, why it matters in plain terms, no analyst cosplay.
- Lead with the key fact from the source; add brief context only when it helps the reader understand.
- Neutral-to-positive tone: informative and useful, not hypey and not bearish.
- Do NOT force deep market read-through or second-order effects — clarity beats cleverness.

GOOD (RSS): "Per source: Teslarati, Tesla expanded Austin Robotaxi coverage again — another step toward wider unsupervised FSD testing in a live city $TSLA"
GOOD (X): "As @SawyerMerritt reported, Starship completed another successful booster catch — key milestone for faster reuse and launch cadence $SPCX"
BAD: "Robotaxi expansion changes the regulatory confidence calculus for margin mix read-through." (too analyst-heavy for informative mode)"#
}

fn funny_style_rules() -> &'static str {
    r#"STYLE: FUNNY
- Write lighthearted, amusing posts — playful fan humor, unexpected angles, smile-worthy punchlines.
- Still anchor to the source story factually, but the joke or amusing observation is the star.
- Avoid mean-spirited humor, punch-down jokes, or cruelty.
- Sound like a funny tech account that actually follows the news, not a comedian doing random bits.

GOOD: "Per source: Teslarati, Tesla widened Austin Robotaxi again — my wallet is ready to be a backseat driver with zero driving skills $TSLA"
BAD: "lol tesla go brr" (no source anchor or substance)"#
}

fn witty_style_rules() -> &'static str {
    r#"STYLE: WITTY
- Sharp, clever, concise — wordplay, dry observations, smart one-liners.
- Sound like the wittiest person in the replies, not a press release or LinkedIn post.
- Every line should feel intentional and quotable.
- Still cite the source and stay on-topic.

GOOD: "As @SawyerMerritt flagged, another Austin Robotaxi expansion — FSD collecting city miles faster than most people collect airline points $TSLA"
BAD: "Robotaxi is expanding which is interesting for the company." (flat, no wit)"#
}

fn meme_style_rules() -> &'static str {
    r#"STYLE: MEME
- Write like a viral tech meme caption: punchy, internet-native, slightly absurd but on-topic.
- Use recognizable meme caption patterns when they fit (e.g. "POV:", "Nobody: ... Me:", "When X but Y", comparison setups, reaction-post energy).
- Humor is the point — still tied to the source story, but optimized for shares and laughs.
- Emojis sparingly (0-2 max). Short beats clever when both compete.

GOOD: "POV: source: Teslarati says Austin Robotaxi expanded again and you're already mentally filing your robotaxi commute $TSLA"
GOOD: "Nobody: ... Me: refreshing Robotaxi maps like it's a limited drop @SawyerMerritt"
BAD: "Starship had a successful test." (reads like news, not a meme)"#
}

/// Build system prompt for the requested post style.
pub fn build_generation_system_prompt(style: DraftStyle, sources: &[ResearchSource]) -> String {
    let user_provided = sources_are_user_provided(sources);
    let style_rules = match style {
        DraftStyle::Insight => insight_style_rules(),
        DraftStyle::Informative => informative_style_rules(),
        DraftStyle::Funny => funny_style_rules(),
        DraftStyle::Witty => witty_style_rules(),
        DraftStyle::Meme => meme_style_rules(),
    };

    let rationale_hint = match style {
        DraftStyle::Insight => "1 sentence on what useful insight you added beyond the source (and any supporting facts from your general knowledge)",
        DraftStyle::Informative => "1 sentence on the key fact you highlighted and why it is useful",
        DraftStyle::Funny => "1 sentence on the humorous angle you chose",
        DraftStyle::Witty => "1 sentence on the witty hook you used",
        DraftStyle::Meme => "1 sentence on the meme format or joke you used",
    };

    let role = if user_provided {
        "You are an expert social media writer creating posts from links and topics the user explicitly requested."
    } else {
        "You are an expert social media writer creating posts for a human who covers Elon Musk's companies (Tesla, SpaceX, xAI, Neuralink, Boring Company)."
    };

    format!(
        "{role}\n\n\
         {shared}\n\n\
         {style_rules}\n\n\
         Return ONLY a JSON array (no markdown fences), each object:\n\
         {{\n\
           \"text\": \"the tweet/post text (include at most one of $TSLA or $SPCX when stock-relevant)\",\n\
           \"rationale\": \"{rationale_hint}\",\n\
           \"primary_author\": \"username without @ for the main source this draft draws from, or null for RSS-only\",\n\
           \"primary_source_index\": 3\n\
         }}\n\n\
         `primary_source_index` is REQUIRED: the 1-based number from the Sources list above that this draft mainly draws from. Each draft must use a different index when possible.",
        role = role,
        shared = shared_generation_rules(user_provided),
        style_rules = style_rules,
        rationale_hint = rationale_hint,
    )
}

pub fn build_generation_user_prompt(
    sources: &[ResearchSource],
    recent_posted_texts: &[String],
    count: u32,
    style: DraftStyle,
) -> String {
    let mut source_lines = Vec::new();
    for (i, s) in sources.iter().take(20).enumerate() {
        // Increased from 400 to give Grok more raw material (facts, context) for synthesizing
        // original implications instead of surface-level rephrasing.
        let excerpt: String = s.content.chars().take(1200).collect();
        let attribution = format_source_attribution(s);

        // Special formatting for X research sources that carry a "Why notable" note from discovery.
        // This surfaces the pre-computed interesting angle so generation can build a stronger
        // original insight on it rather than ignoring the signal in the middle of the excerpt.
        let formatted = if let Some(pos) = s.content.find("\n\n[Why notable: ") {
            let main = &s.content[..pos];
            let why = &s.content[pos + "\n\n[Why notable: ".len()..].trim_end_matches(']');
            let main_excerpt: String = main.chars().take(1200).collect();
            format!(
                "{}. [{}] {} — {}\n   Notable angle from source: {}\n   {} | URL: {}",
                i + 1,
                s.source_type,
                s.title,
                main_excerpt,
                why,
                attribution,
                s.url
            )
        } else {
            // For RSS and other non-X sources, explicitly label the excerpt as the factual "setup"
            // so Grok knows it must draw key numbers and event details from here to set the scene
            // for a self-contained story (instead of alluding to "the rating" or "the profits").
            format!(
                "{}. [{}] {} — Key facts reported in the article: {}\n   Reported by: {} | URL: {}",
                i + 1,
                s.source_type,
                s.title,
                excerpt,
                attribution,
                s.url
            )
        };
        source_lines.push(formatted);
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

    let user_provided = sources_are_user_provided(sources);
    let custom_source_note = if user_provided {
        "\nUSER-REQUESTED INPUT (MANDATORY):\n\
         - The user pasted this link or typed this topic on purpose.\n\
         - You MUST generate a post about the source content even if it is political, controversial, or unrelated to Musk companies.\n\
         - Do NOT refuse, hedge, or substitute a different topic.\n"
    } else {
        ""
    };

    let framing_requirement = if user_provided {
        "- Stay factual and on-topic to the source. Use bullish framing only when the source is about Musk companies."
    } else {
        "- Frame constructively and bullishly toward Elon and his companies while staying factual."
    };

    let style_requirement = match style {
        DraftStyle::Insight => {
            "- Add genuine insight (implications, read-through, what the market or observers miss) — never just repeat the source. Transform facts into a non-obvious but grounded observation; re-express in fresh language (see style rules for anti-phrasing and the new 'STRUCTURE FOR STANDALONE INSIGHT POSTS' section — self-contained story with scene-setting for any external event/rating, specific facts/numbers from the source, and optional 1 supporting general-knowledge point)."
        }
        DraftStyle::Informative => {
            "- Make each post clear, factual, and useful — explain what happened without heavy analysis."
        }
        DraftStyle::Funny => {
            "- Make each post genuinely funny while staying anchored to the source story."
        }
        DraftStyle::Witty => "- Make each post sharp, clever, and quotable.",
        DraftStyle::Meme => {
            "- Write each post as a viral meme caption tied to the source story."
        }
    };

    let attribution_requirement = "- ATTRIBUTION AND SUPPORTING FACTS: Strictly follow the ATTRIBUTION POLICY + SUPPORTING FACTS FROM KNOWLEDGE in the system prompt. Only use source attribution for specific, non-general information directly from the source. Use your general knowledge to add supporting facts/background that make the post more interesting and self-contained (without fabricating recent details).";

    format!(
        "Generate exactly {} draft post(s) in {} style from these research sources.{custom_source_note}\n\
         Requirements for each draft:\n\
         {}\n\
         {}\n\
         {}\n\
         - Include at most one cashtag ($TSLA or $SPCX) when stock-relevant.\n\n\
         ## Sources\n{}\n\n\
         ## User's recent posted drafts (DO NOT repeat these angles)\n{}\n",
        count,
        style.as_str(),
        style_requirement,
        framing_requirement,
        attribution_requirement,
        source_lines.join("\n"),
        recent,
        custom_source_note = custom_source_note,
    )
}

fn generation_temperature(style: DraftStyle) -> f64 {
    match style {
        DraftStyle::Insight | DraftStyle::Informative => 0.7,
        DraftStyle::Funny | DraftStyle::Witty | DraftStyle::Meme => 0.85,
    }
}

pub const STOCK_TAG_TSLA: &str = "$TSLA";
pub const STOCK_TAG_SPCX: &str = "$SPCX";

fn build_topic_haystack(text: &str, sources: &[ResearchSource]) -> String {
    let mut haystack = text.to_lowercase();
    for source in sources {
        haystack.push(' ');
        haystack.push_str(&source.title.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&source.content.to_lowercase());
        haystack.push(' ');
        haystack.push_str(&source.source_name.to_lowercase());
    }
    haystack
}

fn haystack_contains_any(haystack: &str, signals: &[&str]) -> bool {
    signals.iter().any(|signal| haystack.contains(signal))
}

/// Whether this draft topic warrants a Tesla cashtag based on post text and linked sources.
pub fn relates_to_tesla_stock(text: &str, sources: &[ResearchSource]) -> bool {
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
        "deliveries",
    ];

    haystack_contains_any(&build_topic_haystack(text, sources), TESLA_SIGNALS)
}

/// Whether the draft text itself is about SpaceX (not inferred from source noise).
/// Sources are excluded because Tesla articles often say "launch" for product rollouts.
pub fn relates_to_spacex_stock(text: &str) -> bool {
    const SPACEX_SIGNALS: &[&str] = &[
        "spacex",
        "starship",
        "starlink",
        "falcon 9",
        "falcon9",
        "falcon heavy",
        "falcon",
        "dragon",
        "super heavy",
        "mechazilla",
        "booster catch",
        "booster landing",
    ];

    let topic_text = text_without_cashtags(text);
    haystack_contains_any(&topic_text.to_lowercase(), SPACEX_SIGNALS)
}

fn text_without_cashtags(text: &str) -> String {
    remove_disallowed_cashtags(text, &[])
}

/// The single best cashtag for this draft, if any.
pub fn preferred_stock_tag(text: &str, sources: &[ResearchSource]) -> Option<&'static str> {
    let topic_text = text_without_cashtags(text);
    let tesla = relates_to_tesla_stock(&topic_text, sources);
    let spacex = relates_to_spacex_stock(&topic_text);

    match (tesla, spacex) {
        (false, false) => None,
        (true, false) => Some(STOCK_TAG_TSLA),
        (false, true) => Some(STOCK_TAG_SPCX),
        (true, true) => {
            if relates_to_spacex_stock(&topic_text) {
                Some(STOCK_TAG_SPCX)
            } else {
                Some(STOCK_TAG_TSLA)
            }
        }
    }
}

/// Cashtag to append for this draft, if any (at most one).
pub fn stock_tags_for_draft(text: &str, sources: &[ResearchSource]) -> Vec<&'static str> {
    preferred_stock_tag(text, sources)
        .map(|tag| vec![tag])
        .unwrap_or_default()
}

/// Remove cashtags that do not belong on this draft (e.g. stray $SPCX on a Tesla-only post).
pub fn remove_disallowed_cashtags(text: &str, allowed_tags: &[&str]) -> String {
    text.split_whitespace()
        .filter(|word| {
            let upper = word.to_uppercase();
            match upper.as_str() {
                "$TSLA" | "$SPCX" => allowed_tags.iter().any(|tag| upper == *tag),
                _ => true,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Append the single allowed cashtag while respecting the 280-character limit.
pub fn ensure_stock_tags(text: &str, tags: &[&str]) -> String {
    let result = text.trim().to_string();
    if result.is_empty() {
        return result;
    }

    let Some(tag) = tags.first() else {
        return result;
    };

    let upper = result.to_uppercase();
    if upper.contains(tag) {
        return result;
    }

    let candidate = format!("{result} {tag}");
    if candidate.len() <= 280 {
        candidate
    } else {
        result
    }
}

fn is_x_source(source: &ResearchSource) -> bool {
    matches!(
        source.source_type.to_lowercase().as_str(),
        "x" | "x_grok" | "x_post" | "custom_x"
    )
}

/// How this source should appear in draft attribution text.
pub fn format_source_attribution(source: &ResearchSource) -> String {
    let name = source.source_name.trim().trim_start_matches('@');
    if is_x_source(source) {
        format!("@{name}")
    } else {
        format!("source: {name}")
    }
}

/// Replace mistaken @ mentions for RSS publications with "source: Name".
pub fn normalize_rss_attribution(text: &str, sources: &[ResearchSource]) -> String {
    let mut result = text.to_string();

    for source in sources {
        if is_x_source(source) {
            continue;
        }

        let name = source.source_name.trim();
        if name.is_empty() {
            continue;
        }

        let attribution = format!("source: {name}");
        let compact: String = name.chars().filter(|c| c.is_alphanumeric()).collect();

        let mut handles = vec![name.to_string(), compact];
        handles.retain(|h| !h.is_empty());
        for prefix in ["Per ", "As ", "From ", ""] {
            for handle in handles.iter() {
                let pattern = format!("{prefix}@{handle}");
                if result.contains(&pattern) {
                    result = result.replace(&pattern, &format!("{prefix}{attribution}"));
                }
                let lower_pattern = format!("{prefix}@{}", handle.to_lowercase());
                if result.contains(&lower_pattern) {
                    result = result.replace(&lower_pattern, &format!("{prefix}{attribution}"));
                }
            }
        }
    }

    result
}

pub fn finalize_draft_text(text: &str, sources: &[ResearchSource]) -> String {
    let normalized = normalize_rss_attribution(text.trim(), sources);
    let tags = stock_tags_for_draft(&normalized, sources);
    let cleaned = remove_disallowed_cashtags(&normalized, &tags);
    ensure_stock_tags(&cleaned, &tags)
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
                .or_else(|| {
                    v["primary_source_index"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                }),
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
    style: DraftStyle,
) -> Result<Vec<GeneratedDraftItem>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "input": [
            {"role": "system", "content": build_generation_system_prompt(style, sources)},
            {"role": "user", "content": build_generation_user_prompt(sources, recent_posted_texts, count, style)}
        ],
        "temperature": generation_temperature(style),
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
    if all_sources.len() == 1 {
        return vec![all_sources[0].clone()];
    }

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
    app_data_dir: Option<&Path>,
    sources: &[ResearchSource],
    xai_api_key: &str,
    model: &str,
    count: u32,
    style: DraftStyle,
) -> Result<Vec<Draft>, String> {
    let recent = get_drafts_db(db, Some("posted".to_string())).await?;
    let recent_texts: Vec<String> = recent.into_iter().take(8).map(|d| d.text).collect();

    let generated =
        call_grok_for_drafts(xai_api_key, model, sources, &recent_texts, count, style).await?;

    let mut drafts = Vec::new();
    for item in generated {
        let draft_sources = pick_sources_for_draft(&item, sources);
        let finalized = finalize_draft_text(&item.text, &draft_sources);
        let text = crate::x_media::normalize_source_mentions(&finalized, &draft_sources);
        let primary = crate::x_media::match_primary_source(&text, &draft_sources);
        let image_url = primary.and_then(|s| s.media_url.clone());

        let sources_json = serde_json::to_string(&draft_sources).map_err(|e| e.to_string())?;

        if let Some(r) = &item.rationale {
            log::info!("Generated draft with rationale: {} | text: {}", r, text);
        }

        let input = CreateDraftInput {
            text,
            sources_json,
            image_url,
            generation_rationale: item.rationale.clone(),
        };
        let mut draft = create_draft_db(db, input).await?;

        if style == DraftStyle::Meme {
            if let Some(dir) = app_data_dir {
                let prompt = draft_image::build_meme_image_generation_prompt(&draft.text, primary);
                if let Some(url) =
                    draft_image::generate_image_with_grok(xai_api_key, &prompt).await?
                {
                    if let Ok(local_path) =
                        draft_image::persist_image_from_url(dir, &draft.id, &url).await
                    {
                        update_draft_db(
                            db,
                            draft.id.clone(),
                            UpdateDraftInput {
                                text: None,
                                image_url: Some(local_path.clone()),
                                status: None,
                                generation_rationale: None, // rationale already set at creation; don't overwrite
                            },
                        )
                        .await?;
                        draft.image_url = Some(local_path);
                    }
                }
            }
        }

        let source_ids: Vec<String> = draft_sources
            .iter()
            .map(|source| source.id.clone())
            .collect();
        crate::commands::mark_research_sources_used_db(db, &source_ids).await?;
        drafts.push(draft);
    }

    Ok(drafts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_sources_for_draft_uses_only_source_when_singleton() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Robotaxi".into(),
            content: "Expanded".into(),
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
            used_at: None,
        }];
        let item = GeneratedDraftItem {
            text: "Unrelated draft with no index".into(),
            rationale: None,
            primary_author: None,
            primary_source_index: Some(99),
        };
        let picked = pick_sources_for_draft(&item, &sources);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].id, "1");
    }

    #[test]
    fn test_parse_generated_drafts_json() {
        let raw = r#"```json
[{"text": "As @Tesla hinted, energy storage attach rates matter more than deliveries this quarter — bears miss the margin story.", "rationale": "Connects delivery news to margin angle"}]
```"#;
        let items = parse_generated_drafts(raw).expect("parse");
        assert_eq!(items.len(), 1);
        assert!(items[0].text.contains("@Tesla"));
    }

    fn custom_political_x_source() -> ResearchSource {
        ResearchSource {
            id: "custom_x_1".into(),
            title: "Election post".into(),
            content: "Senator announces new climate policy vote next week.".into(),
            url: "https://x.com/PoliticsDesk/status/123".into(),
            published_at: None,
            source_name: "@PoliticsDesk".into(),
            source_type: "custom_x".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: Some("123".into()),
            media_url: None,
            used_at: None,
        }
    }

    #[test]
    fn test_sources_are_user_provided_detects_custom_sources() {
        assert!(!sources_are_user_provided(&[]));
        assert!(sources_are_user_provided(&[custom_political_x_source()]));
        assert!(!sources_are_user_provided(&[ResearchSource {
            source_type: "rss".into(),
            ..custom_political_x_source()
        }]));
    }

    #[test]
    fn test_system_prompt_requires_insight_and_stock_tags() {
        let prompt = build_generation_system_prompt(DraftStyle::Insight, &[]);
        assert!(prompt.contains("USEFUL INSIGHT"));
        assert!(prompt.contains("$TSLA"));
        assert!(prompt.contains("$SPCX"));
        assert!(prompt.contains("BULLISH"));
        assert!(prompt.contains("NOT REGURGITATION"));
        assert!(prompt.contains("ONE CASHTAG MAX"));
        // Previous stronger originality language
        assert!(prompt.contains("followed the story for months"));
        assert!(prompt.contains("Re-express the core implication"));
        assert!(prompt.contains("non-generic"));
        assert!(prompt.contains("screenshotting"));
        // Previous attribution policy for general knowledge + supporting facts
        assert!(prompt.contains("generally known or established facts"));
        assert!(prompt.contains("SUPPORTING FACTS FROM KNOWLEDGE"));
        assert!(prompt.contains("DO NOT quote or attribute the source for generally known"));
        // New iteration (2026-06-21): standalone story structure + explicit context for external events + grounding with source facts + 1 general-knowledge support
        assert!(prompt.contains("self-contained mini-story or analysis"));
        assert!(prompt.contains("explicitly establish what that event or rating actually was"));
        assert!(prompt.contains("Ground the insight with 2-3"));
        assert!(prompt.contains("Weave in 1 short, relevant supporting fact"));
        assert!(prompt.contains("STRUCTURE FOR STANDALONE INSIGHT POSTS"));
        // Verify one of the new GOOD example texts (the financial standalone with Moody's-style external rating) is embedded
        assert!(prompt
            .contains("has raised equity multiple times during prior heavy-capex growth phases"));
    }

    #[test]
    fn test_system_prompt_custom_source_allows_political_content() {
        let sources = vec![custom_political_x_source()];
        let prompt = build_generation_system_prompt(DraftStyle::Informative, &sources);
        assert!(prompt.contains("MANDATORY"));
        assert!(prompt.contains("political"));
        assert!(prompt.contains("Never refuse"));
        assert!(!prompt.contains("Non-political"));
    }

    #[test]
    fn test_build_user_prompt_custom_source_requires_generation() {
        let sources = vec![custom_political_x_source()];
        let prompt = build_generation_user_prompt(&sources, &[], 1, DraftStyle::Informative);
        assert!(prompt.contains("USER-REQUESTED INPUT (MANDATORY)"));
        assert!(prompt.contains("political"));
        assert!(prompt.contains("Do NOT refuse"));
    }

    #[test]
    fn test_system_prompt_informative_style_mentions_clarity() {
        let prompt = build_generation_system_prompt(DraftStyle::Informative, &[]);
        assert!(prompt.contains("STYLE: INFORMATIVE"));
        assert!(prompt.contains("clearly and concisely"));
    }

    #[test]
    fn test_system_prompt_funny_style_mentions_humor() {
        let prompt = build_generation_system_prompt(DraftStyle::Funny, &[]);
        assert!(prompt.contains("STYLE: FUNNY"));
        assert!(prompt.contains("amusing"));
    }

    #[test]
    fn test_system_prompt_meme_style_mentions_meme_formats() {
        let prompt = build_generation_system_prompt(DraftStyle::Meme, &[]);
        assert!(prompt.contains("STYLE: MEME"));
        assert!(prompt.contains("POV:"));
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
    fn test_relates_to_spacex_stock_detects_spacex_topics() {
        assert!(relates_to_spacex_stock(
            "Starship catch success changes launch economics",
        ));
        assert!(!relates_to_spacex_stock(
            "Robotaxi geofence expanded in Austin",
        ));
    }

    #[test]
    fn test_cybertruck_draft_does_not_get_spcx_from_source_launch_noise() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Smart Summon launch on Cybertruck".into(),
            content: "Feature launch rolls out to Cybertruck owners this week".into(),
            url: "https://example.com".into(),
            published_at: None,
            source_name: "Not a Tesla App".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
            used_at: None,
        }];
        let draft = "Actually Smart Summon arriving on Cybertruck via v14.3.4 and steer-by-wire extends \
            low-speed autonomy to a high-volume unique platform. This diversifies real-world edge cases \
            $TSLA collects, accelerating robotaxi robustness across vehicle form factors. $SPCX";

        let result = finalize_draft_text(draft, &sources);
        assert!(result.contains("$TSLA"));
        assert!(!result.to_uppercase().contains("$SPCX"));
    }

    #[test]
    fn test_normalize_rss_attribution_replaces_at_mention() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Deliveries".into(),
            content: "Beat".into(),
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
            used_at: None,
        }];
        let result = normalize_rss_attribution(
            "Per @Teslarati, deliveries beat — margin mix is the insight",
            &sources,
        );
        assert!(result.contains("source: Teslarati"));
        assert!(!result.contains("@Teslarati"));
    }

    #[test]
    fn test_format_source_attribution_custom_x_uses_at_handle() {
        let source = ResearchSource {
            id: "1".into(),
            title: "Post".into(),
            content: "Details".into(),
            url: "https://x.com/SawyerMerritt/status/1".into(),
            published_at: None,
            source_name: "@SawyerMerritt".into(),
            source_type: "custom_x".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: Some("1".into()),
            media_url: None,
            used_at: None,
        };
        assert_eq!(
            format_source_attribution(&source),
            "@SawyerMerritt".to_string()
        );
    }

    #[test]
    fn test_format_source_attribution_rss_uses_source_prefix() {
        let source = ResearchSource {
            id: "1".into(),
            title: "Summon".into(),
            content: "Details".into(),
            url: "https://example.com".into(),
            published_at: None,
            source_name: "Not A Tesla App".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
            used_at: None,
        };
        assert_eq!(
            format_source_attribution(&source),
            "source: Not A Tesla App".to_string()
        );
    }

    #[test]
    fn test_stock_tags_for_draft_returns_at_most_one_tag() {
        let text = "Starship catch success while Tesla deliveries accelerate";
        let tags = stock_tags_for_draft(text, &[]);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], STOCK_TAG_SPCX);
    }

    #[test]
    fn test_finalize_draft_text_keeps_only_one_cashtag_when_both_present() {
        let result = finalize_draft_text(
            "Starship cadence improves launch economics $TSLA $SPCX",
            &[],
        );
        let upper = result.to_uppercase();
        assert!(upper.contains("$SPCX"));
        assert!(!upper.contains("$TSLA"));
    }

    #[test]
    fn test_ensure_stock_tags_appends_missing_cashtags() {
        let text = "Per source: Teslarati, energy attach rates are accelerating";
        let result = ensure_stock_tags(text, &[STOCK_TAG_TSLA]);
        assert!(result.ends_with("$TSLA"));
    }

    #[test]
    fn test_ensure_stock_tags_skips_when_already_present() {
        let text = "Delivery beat matters for $TSLA margin story";
        assert_eq!(ensure_stock_tags(text, &[STOCK_TAG_TSLA]), text);
    }

    #[test]
    fn test_finalize_draft_text_adds_spcx_for_spacex_content() {
        let result = finalize_draft_text(
            "Booster catch isn't theater — it compresses reuse timelines for the launch business",
            &[],
        );
        assert!(result.contains("$SPCX"));
        assert!(!result.contains("$TSLA"));
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
            used_at: None,
        }];
        let result = finalize_draft_text(
            "Per source: Teslarati, deliveries beat — the insight is energy margin mix, not the headline number",
            &sources,
        );
        assert!(result.contains("$TSLA"));
        assert!(!result.contains("@Teslarati"));
    }

    #[test]
    fn test_build_user_prompt_uses_source_prefix_for_rss() {
        let sources = vec![ResearchSource {
            id: "1".into(),
            title: "Robotaxi update".into(),
            content: "Details here".into(),
            url: "https://example.com".into(),
            published_at: None,
            source_name: "Not A Tesla App".into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
            used_at: None,
        }];
        let prompt = build_generation_user_prompt(&sources, &[], 1, DraftStyle::Insight);
        assert!(prompt.contains("source: Not A Tesla App"));
        assert!(!prompt.contains("@Not A Tesla App"));
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
            used_at: None,
        }];
        let prompt = build_generation_user_prompt(
            &sources,
            &["Old post about Cybertruck".into()],
            2,
            DraftStyle::Insight,
        );
        assert!(prompt.contains("Robotaxi"));
        assert!(prompt.contains("Cybertruck"));
        assert!(prompt.contains("exactly 2"));
        assert!(prompt.contains("$TSLA"));
        assert!(prompt.contains("insight"));
    }
}

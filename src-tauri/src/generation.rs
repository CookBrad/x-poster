use crate::commands::{
    create_draft_db, get_drafts_db, update_draft_db, CreateDraftInput, Draft, UpdateDraftInput,
};
use crate::constants::DraftStyle;
use crate::draft_image;
use crate::research::ResearchSource;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::DbPool;
use std::path::Path;
use std::time::Duration;

// Lifted exemplars to consts per strategy: one source of truth, <280 chars, include required facts (77%, 2013, SaveRGV/Sierra/Carrizo, Huddle, w/ prejudice, ~450hrs).
// Target ~240-279 chars: scene-setting + named facts + concrete impact + non-obvious takeaway (dual information+insight bar).
const BOCA_CHICA_FACT_CORE: &str = "2009: 77% amended TX constitution for permanent public beach easement. 2013 carved Boca Chica launch closures. SaveRGV + Sierra + Carrizo sued. Sup Ct (Huddle): no private right to sue; dismissed w/ prejudice. Ends ~450hrs/yr pad blocks - Starship free of beach-suit lag. $SPCX";

// Insight GOOD: same fact pack plus a second-order read-through (cadence free of private beach-suit risk) - not just a fact dump.
const HUMAN_VOICE_GOOD_INSIGHT: &str = "Wait - 2009: 77% locked public beach access into TX constitution. 2013 carved Boca Chica launch closures. SaveRGV + Sierra + Carrizo sued. Sup Ct (Huddle): no private right to sue; dismissed w/ prejudice. ~450hrs/yr pad blocks over - cadence free of private beach-suit risk. $SPCX";

// One source of truth for the core legal fact string used in insight GOOD example (reuses core to avoid dead_code and keep facts identical).
const INSIGHT_LEGAL_GOOD: &str = BOCA_CHICA_FACT_CORE;

const INSIGHT_MOODYS_GOOD: &str = "Tesla holds ~$40B cash, zero debt, steady profits - real room to self-fund Optimus + unsupervised robotaxi without diluting shareholders. Moody's Baa1/stable flags execution risk but underweights that balance-sheet headroom. (Raised equity in prior capex phases.) $TSLA";

const HUMAN_VOICE_GOOD_INFORMATIVE: &str = BOCA_CHICA_FACT_CORE;

const HUMAN_VOICE_GOOD_FUNNY: &str = "2009: 77% said beaches belong to everyone - then 2013 gave rockets dibs on Boca Chica. SaveRGV + Sierra + Carrizo sued anyway. Sup Ct (Huddle): constitution has no 'sue' button. Dismissed w/ prejudice. ~450hrs/yr of launch drama? Gone. Starship pad breathes again. $SPCX";

const HUMAN_VOICE_GOOD_WITTY: &str = "2009: beaches ours (77% vote, constitution). 2013: 'except Boca Chica, rockets need it.' Lawsuit? Sup Ct (Huddle): no private right to sue. Dismissed w/ prejudice - ~450hrs/yr of legal stand-down theater over. Starship gets the beach and the last laugh. $SPCX";

const HUMAN_VOICE_GOOD_MEME: &str = "POV: 2009 - 77% vote permanent public beach access into TX law. 2013 - SpaceX borrows Boca Chica for launches. SaveRGV sues. Sup Ct (Huddle): no sue button in the amendment. Dismissed w/ prejudice. ~450hrs/yr of pad drama? Poof. Starship back. $SPCX";

/// Content shorter than this (char count) is treated as thin and gets article-body enrichment
/// and/or related-story grouping before being sent to Grok.
pub const THIN_SOURCE_CONTENT_LEN: usize = 550;

/// Title signals that mark major factual/legal/regulatory stories needing extra concrete material.
pub const MAJOR_STORY_TITLE_SIGNALS: &[&str] = &[
    "supreme court",
    "court ruling",
    "amendment",
    "litigation",
    "beach access",
    "boca chica",
    "delay",
    "injunction",
    "sue",
    "block",
    "approval",
    "regulatory",
];

/// Label prepended when article body is appended for concrete named facts (primary sources).
pub const ARTICLE_BODY_ENRICHMENT_LABEL: &str = " [Additional article body from source URL — use for concrete names, numbers, amendment text, holding details, quantified impacts, etc.: ";

/// Label when enriching a related/similar story's body.
pub const RELATED_ARTICLE_BODY_LABEL: &str = " [Additional article body — use for more facts: ";

/// Whether a source is too thin or is a major factual story that needs enrichment/grouping.
pub fn source_needs_generation_enrichment(source: &ResearchSource) -> bool {
    let content_len = source.content.chars().count();
    let title_lower = source.title.to_lowercase();
    content_len < THIN_SOURCE_CONTENT_LEN
        || MAJOR_STORY_TITLE_SIGNALS
            .iter()
            .any(|sig| title_lower.contains(sig))
}

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

/// Simple similarity for gathering a small group of related stories when a
/// primary source (especially a thin headline/X post or legal/regulatory item)
/// does not contain enough concrete facts on its own.
/// Scores on shared title keywords + entity signals (e.g. court names, amendment
/// years, locations, programs like Starship). Used to implement "if not enough
/// info in one post, base the draft on a group of similar stories".
pub fn find_similar_sources(
    primary: &ResearchSource,
    all_sources: &[ResearchSource],
    max: usize,
) -> Vec<ResearchSource> {
    if all_sources.is_empty() {
        return vec![];
    }

    let primary_title = primary.title.to_lowercase();

    // Signals that indicate "this is the same story" across coverage.
    // Include both general (court, ruling) and story-specific when present in primary.
    let mut signals: Vec<String> = vec![
        "supreme court".into(), "texas supreme".into(), "boca chica".into(),
        "beach access".into(), "open beaches".into(), "amendment".into(),
        "litigation".into(), "ruling".into(), "sue".into(), "injunction".into(),
        "closure".into(), "delay".into(), "starship".into(), "pad".into(),
    ];
    // Add tokens from the primary title (proper nouns, years, key nouns)
    for token in primary_title.split(|c: char| !c.is_alphanumeric()) {
        if token.len() >= 4 && !["the", "and", "for", "with", "over", "from"].contains(&token) {
            signals.push(token.to_string());
        }
    }

    let mut scored: Vec<(i32, &ResearchSource)> = Vec::new();
    for s in all_sources {
        if s.url == primary.url || s.id == primary.id {
            continue;
        }
        let t = format!("{} {}", s.title, s.content).to_lowercase();
        let mut score = 0;
        for sig in &signals {
            if t.contains(sig) {
                score += 2;
            }
        }
        if s.source_name == primary.source_name {
            score += 1;
        }
        if score > 0 {
            scored.push((score, s));
        }
    }

    scored.sort_by_key(|(sc, _)| std::cmp::Reverse(*sc));
    scored
        .into_iter()
        .take(max)
        .map(|(_, s)| s.clone())
        .collect()
}

const LENGTH_AND_CONTEXT_RULES: &str = r#"- LENGTH + ENCOMPASSING CONTEXT (MANDATORY):
  - Use the full single-tweet budget: aim for 220-279 characters when the story has enough facts. Longer, scene-setting posts that walk a zero-context reader through the situation outperform ultra-compressed shorthand.
  - Do NOT write telegram-style fragments, cryptic abbreviations, or bare headline rewrites. Every post must read as a complete mini-story: (1) brief background or prior situation, (2) what happened now, (3) concrete operational/strategic impact, (4) fresh insight or takeaway (insight style) or clear why-it-matters (informative style).
  - A reader who has never seen the source, the link, or prior coverage must still understand the full arc — who/what/when, what changed, and why it matters — without googling.
  - Include 3-5 specific named facts from the sources (years, parties, numbers, holdings, program names) woven into flowing sentences — not a single vague summary line.
  - DUAL BAR (MANDATORY — information density AND insight together): Maximize both in the same post. Never ship a pure fact-dump with no non-obvious takeaway, and never ship a hot take or implication with thin or vague source grounding. When facts allow, use 220-279 chars to pack named details *and* a useful read-through.
  - BAD (too short, no context): "Court ends Boca Chica lawsuit. Starship can launch faster now. $SPCX" (assumes reader knows the backstory; no amendment, no litigants, no quantified prior impact)
  - BAD (too compressed): "2009 amend. 2013 law. Sued. No right. Prejudice. 450hrs. $SPCX" (telegram shorthand, not informative)
  - BAD (fact-dump, no insight): lists every named party and year but ends with no implication, second-order effect, or "so what" beyond restating the outcome
  - BAD (insight-only, thin facts): a clever read-through or prediction with only one vague fact or a headline paraphrase — fails the specific-facts-from-sources rule"#;

/// View/engagement + 2026 For You ranking guidance shared by every style path.
/// Facts/dual bar stay mandatory — never relaxed for virality.
const ENGAGEMENT_AND_VIEWS_RULES: &str = r#"- ENGAGEMENT + VIEWS + 2026 X RANKING (MANDATORY — every style):
  - Optimize every draft for high engagement and views on X (For You distribution): scroll-stopping first-line hook, human conversational voice, bookmark/quote-worthy density, and a conversation-forcing ending that invites real replies.
  - Early engagement velocity matters: the first 30–60 minutes after posting decide expansion vs death. Write posts that make people stop, save, quote, and especially *reply* — replies (and author engaging those replies) outweigh likes by a large margin.
  - First line = the hook. Lead with a surprising fact, bold claim, intriguing question, vivid scene, or "wait, what?" moment that stops the scroll. Never open with dry wire-copy ("The company announced...", "According to a report...", "A court ruled that...").
  - Conversation-forcing ending (MANDATORY): close with a real question, sharp implication, or debate-worthy claim that invites genuine replies — not engagement bait ("Like if you agree", "RT if…", "comment YES"). The body stays a self-contained standalone story; the ending *invites* discussion without turning the post into a reply.
  - Zero external URLs in the main post body (external links suppress reach). Prefer native context + at most one cashtag; attribution via @handle or "source: Publication Name" is fine — never paste https:// links.
  - Hashtags: 0 preferred, at most one hashtag total. Never multi-hashtag spam walls.
  - Bookmark / screenshot / quote worthiness: include at least one dense, save-worthy line people want to screenshot, bookmark, or quote-tweet.
  - Native media: when an image will attach, write text that pairs with the visual; never pad with "link in bio" or external redirects.
  - Sound human, not AI: conversational prose full of contractions (it's, don't, we're), natural rhythm, short punchy sentences mixed with longer ones — like an enthusiast geeking out with friends, not a press release or LinkedIn post.
  - Anti-patterns: engagement bait, dry wire-copy openers, pure fact dumps with no conversation-forcing close, multi-hashtag spam, raw URLs in the body.
  - Facts are never relaxed for virality: still obey EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES and DUAL BAR. Never fabricate claims or ship pure engagement bait without named source details."#;

fn shared_generation_rules(user_provided: bool) -> String {
    if user_provided {
        format!(
        r#"SHARED RULES (user-requested link or topic):
- EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES (MANDATORY). Every claim, description of a problem or prior situation ("why it was a slowdown"), implication, or forward-looking statement must be directly traceable to concrete, named details in the provided Sources (or the Related/similar coverage if present): the exact year + text/operative effect of an amendment or law, the parties/litigants, the precise holding or procedural outcome (e.g. "unanimous", "with prejudice", "no private right of action"), quantified operational impacts (hours closed, number of delays, specific test stands-downs), numbers, dates, quotes, or events reported in the excerpts. Do NOT use vague summary language ("years of litigation", "recurring legal blocks", "repeated delays", "can tighten", "ended years of...") unless the source material supplies the actual duration, count, names, or details. Ground every draft with multiple specific, citable facts from the sources rather than alluding to or summarizing them at high level. The post must read as if written by someone who read the full reporting and related coverage.
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
- Each post must be under 280 characters (single tweet). Count cashtags toward the limit.
{}
{}"#,
        LENGTH_AND_CONTEXT_RULES,
        ENGAGEMENT_AND_VIEWS_RULES
        )
    } else {
        format!(
        r#"SHARED RULES (all styles):
- EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES (MANDATORY). Every claim, description of a problem or prior situation ("why it was a slowdown"), implication, or forward-looking statement must be directly traceable to concrete, named details in the provided Sources (or the Related/similar coverage if present): the exact year + text/operative effect of an amendment or law, the parties/litigants, the precise holding or procedural outcome (e.g. "unanimous", "with prejudice", "no private right of action"), quantified operational impacts (hours closed, number of delays, specific test stands-downs), numbers, dates, quotes, or events reported in the excerpts. Do NOT use vague summary language ("years of litigation", "recurring legal blocks", "repeated delays", "can tighten", "ended years of...") unless the source material supplies the actual duration, count, names, or details. Ground every draft with multiple specific, citable facts from the sources rather than alluding to or summarizing them at high level. The post must read as if written by someone who read the full reporting and related coverage.
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
- Each post must be under 280 characters (single tweet). Count cashtags toward the limit.
{}
{}"#,
        LENGTH_AND_CONTEXT_RULES,
        ENGAGEMENT_AND_VIEWS_RULES
        )
    }
}

fn insight_style_rules() -> String {
    format!(
        r#"STYLE: INSIGHT (default)
1. USEFUL INSIGHT REQUIRED — NOT REGURGITATION (paired with max information density):
   - Every post must add value beyond the source headline: implications, second-order effects, what bulls/bears miss, competitive context, timeline read-through, margin/capital angle, or strategic significance.
   - Every post must also pack 3-5 specific named facts from the sources (see SHARED RULES + DUAL BAR). Insight without named facts is a hot take; facts without a non-obvious takeaway are a news dump. Deliver BOTH.
   - Do NOT restate, summarize, or closely paraphrase the source.
   - Do NOT write empty hype or press-release filler.
   - Transform the facts into a novel observation or prediction that feels like it comes from someone who has followed the story for months — specific, non-generic, worth screenshotting or quoting in replies.
   - The final clause or sentence should land a fresh implication (e.g. cadence free of private beach-suit risk, balance-sheet headroom underweighted by a rating) — not stop at "the court ruled X" or "this is good news."

2. ANTI-PHRASING RULE:
   - Never reuse verbs, sentence structures, or headline phrasing directly from the source or title. Re-express the core implication in fresh language.

3. HUMAN VOICE + ENGAGEMENT FOR VIRAL POTENTIAL (applies to all styles; reinforces ENGAGEMENT + VIEWS + 2026 X RANKING):
   - Sound like a real human enthusiast or insider tweeting to fellow fans, not an AI news summarizer or press release. Use conversational prose full of contractions (it's, don't, we're, that's), natural rhythm, short punchy sentences mixed with longer ones, and a tone that's excited, wry, or in awe — like you're geeking out in the replies.
   - Strong first-line hook required on every post: Lead with a surprising fact, bold claim, intriguing question, vivid scene, or "wait, what?" moment that stops the scroll and makes people read the rest — this is how posts earn views in the feed.
   - Drive engagement and quotability: Build in lines that feel screenshot-worthy, bookmark-worthy, or reply-baiting — subtle emotional resonance (the relief of a bureaucracy lifted, the "this is why it matters for the future" spark), storytelling flow that carries the reader, or implications that make followers think "exactly" or "whoa, never thought of it that way" and want to quote or tag someone.
   - Conversation-forcing close: end on a real question, sharp implication, or debate-worthy claim that invites genuine replies (replies beat likes for For You ranking). Keep the body a self-contained standalone story — do not write as if replying to someone else's post.
   - Zero external URLs in the post body; 0 hashtags preferred (at most one). Never multi-hashtag spam or paste https:// links.
   - Weave the key concrete facts from the sources (e.g. the 2009 Open Beaches Amendment with its 77% voter approval and permanent public easement guarantee, the 2013 law creating the space-flight exception, the litigants SaveRGV joined by Sierra Club and Carrizo/Comecrudo Nation, Justice Huddle's unanimous opinion that the amendment creates no private right to sue, "dismissed with prejudice", the ~450 hours per year of closures that actually forced repeated pad stands-downs and test delays) into flowing narrative sentences. Never fall into summary voice, bullet lists, or "the court ruled that..." dryness.
   - The whole point: higher viral potential and more views via early engagement velocity (hooks + replies + bookmarks). Posts that feel alive, human, worth amplifying and arguing about — while 100% obeying the "EVERY POST MUST BE BACKED BY SPECIFIC FACTS" rule above and never fabricating.

4. STRUCTURE FOR STANDALONE INSIGHT POSTS (not replies):
   - Write as a self-contained mini-story or analysis with an encompassing arc: background → what happened → concrete impact → fresh insight. A reader who has never seen the source or the news must still understand the full situation without prior context.
   - Open with scene-setting: the prior law/policy/situation and key facts from the source (specific numbers, parties, what the external event/rating actually was).
   - Deliver the fresh insight/implication in the second half — never sacrifice context for a punchy one-liner, and never omit the insight to fit more facts.
   - Aim for 220-279 characters: use the full tweet budget to include enough facts and context; do not default to ultra-short hot takes.
   - Optionally weave in 1 short supporting fact from general knowledge (historical context, scale of the programs, etc.) that makes the "why this matters" clearer.
   - Use attribution only for non-general, source-specific claims.

GOOD (human voice, hooky, dual bar — named facts + non-obvious takeaway from primary + related coverage): {insight_good}
BAD (stilted/AI-like or flat non-engaging, even with facts): "The Texas Supreme Court issued a unanimous ruling written by Justice Rebeca Huddle determining that the 2009 Open Beaches Amendment to the Texas Constitution does not create a private right of action for enforcement of beach access rights against the 2013 space flight activities authorization, resulting in dismissal with prejudice of the litigation brought by SaveRGV and joined by the Sierra Club and Carrizo/Comecrudo Nation of Texas." (dry, list-y, zero hook, zero flow, zero quotable human spark — the exact voice to avoid for engagement/virality)
BAD (fact-dump without insight): "2009: 77% beach amendment. 2013 Boca Chica closures. SaveRGV + Sierra + Carrizo sued. Sup Ct (Huddle): no private right to sue. Dismissed w/ prejudice. ~450hrs/yr pad blocks ended. $SPCX" (all facts, no second-order read-through)
BAD (insight without facts): "Private beach suits no longer hold Starship cadence hostage — huge for launch economics. $SPCX" (clever take, zero named source details)
BAD (ends flat, no conversation-forcing close): packs solid facts then dies on a dry restatement with no question, sharp implication, or debate hook for replies
BAD (external URL in body): "... Full story: https://example.com/article $SPCX" (raw links suppress reach — zero external URLs in the main post)"#,
        insight_good = HUMAN_VOICE_GOOD_INSIGHT,
    )
}

fn informative_style_rules() -> String {
    format!(
        r#"STYLE: INFORMATIVE
- Share the news clearly and informatively — walk the reader through what happened, the relevant background, and why it matters in plain terms. No analyst cosplay, but do not skimp on context.
- Lead with scene-setting (prior situation or background from the source), then the development, then the concrete impact. Aim for 220-279 characters when facts allow.
- Still obey DUAL BAR substance: pack 3-5 specific named facts from the sources and end with a clear why-it-matters (not a vague "this is significant"). Fact-backed substance is mandatory even without deep market read-through.
- Neutral-to-positive tone: informative and useful, not hypey and not bearish.
- Do NOT force deep market read-through or second-order effects — clarity beats cleverness, but clarity requires enough context for a zero-prior-knowledge reader.

GOOD (RSS): "Per source: Teslarati, Tesla widened Austin Robotaxi geofence again — more unsupervised FSD miles in a live city, another concrete step toward broader unsupervised testing $TSLA"
GOOD (X): "As @SawyerMerritt reported, Starship completed another successful booster catch — the concrete reuse milestone that shortens turnaround toward higher launch cadence $SPCX"
BAD: "Robotaxi expansion changes the regulatory confidence calculus for margin mix read-through." (too analyst-heavy for informative mode)
BAD (thin facts): "Tesla expanded Robotaxi in Austin again. $TSLA" (headline only; no named details or why-it-matters)
- Apply human voice + engagement for views: scroll-stopping first-line hooks (surprising fact, bold claim or question), conversational prose full of contractions and natural rhythm, quotable/share-worthy or reply-baiting lines, weave facts into flowing narrative. Conversation-forcing ending; zero external URLs; 0 hashtags preferred (at most one). Optimize for high engagement and views without dropping the fact bar.
GOOD (human voice, informative style — scene-setting, conversational, multi-fact, <280): {informative_good}
BAD (informative but flat/AI): "The court determined that the constitutional amendment does not provide a private right of action, leading to dismissal of the beach access litigation." (no hook, no voice, no flow, thin facts — zero engagement potential)"#,
        informative_good = HUMAN_VOICE_GOOD_INFORMATIVE,
    )
}

fn funny_style_rules() -> String {
    format!(
        r#"STYLE: FUNNY
- Write lighthearted, amusing posts — playful fan humor, unexpected angles, smile-worthy punchlines.
- Still anchor to the source story factually, but the joke or amusing observation is the star.
- Avoid mean-spirited humor, punch-down jokes, or cruelty.
- Sound like a funny tech account that actually follows the news, not a comedian doing random bits.

GOOD: "Per source: Teslarati, Tesla widened Austin Robotaxi again — my wallet is ready to be a backseat driver with zero driving skills $TSLA"
BAD: "lol tesla go brr" (no source anchor or substance)
- Apply human voice + engagement for views: scroll-stopping first-line hooks, conversational prose with contractions, quotable/share-worthy punchlines and playful narrative weave of facts. Conversation-forcing ending; zero external URLs; 0 hashtags preferred (at most one). Optimize for high engagement and views without dropping the fact bar.
GOOD (human voice, funny style — playful, quotable, facts in joke, <280): {funny_good}
BAD (funny but not human/grounded): "Haha SpaceX wins beach lawsuit, very funny." (no facts, no hook, no real joke — zero share potential)"#,
        funny_good = HUMAN_VOICE_GOOD_FUNNY,
    )
}

fn witty_style_rules() -> String {
    format!(
        r#"STYLE: WITTY
- Sharp, clever, concise — wordplay, dry observations, smart one-liners.
- Sound like the wittiest person in the replies, not a press release or LinkedIn post.
- Every line should feel intentional and quotable.
- Still cite the source and stay on-topic.

GOOD: "As @SawyerMerritt flagged, another Austin Robotaxi expansion — FSD collecting city miles faster than most people collect airline points $TSLA"
BAD: "Robotaxi is expanding which is interesting for the company." (flat, no wit)
- Apply human voice + engagement for views: scroll-stopping first-line hooks, conversational prose, sharp quotable/share-worthy one-liners and witty narrative with facts. Conversation-forcing ending; zero external URLs; 0 hashtags preferred (at most one). Optimize for high engagement and views without dropping the fact bar.
GOOD (human voice, witty style — sharp, quotable one-liner with facts, <280): {witty_good}
BAD (witty but lifeless): "The ruling clarifies that no private enforcement mechanism exists under the amendment." (clever? no. human? no. — will not earn views)"#,
        witty_good = HUMAN_VOICE_GOOD_WITTY,
    )
}

fn meme_style_rules() -> String {
    format!(
        r#"STYLE: MEME
- Write like a viral tech meme caption: punchy, internet-native, slightly absurd but on-topic.
- Use recognizable meme caption patterns when they fit (e.g. "POV:", "Nobody: ... Me:", "When X but Y", comparison setups, reaction-post energy).
- Humor is the point — still tied to the source story, but optimized for shares and laughs.
- Emojis sparingly (0-2 max). Short beats clever when both compete.

GOOD: "POV: source: Teslarati says Austin Robotaxi expanded again and you're already mentally filing your robotaxi commute $TSLA"
GOOD: "Nobody: ... Me: refreshing Robotaxi maps like it's a limited drop @SawyerMerritt"
BAD: "Starship had a successful test." (reads like news, not a meme)
- Apply human voice + engagement for views: scroll-stopping first-line hooks, conversational prose, punchy meme energy with quotable/share-worthy hooks and facts in narrative. Conversation-forcing ending; zero external URLs; 0 hashtags preferred (at most one). Optimize for high engagement and views without dropping the fact bar.
GOOD (human voice, meme style — punchy viral meme with facts, <280): {meme_good}
BAD (meme but not human/viral): "Court rules on beach access for launches." (boring text, no meme energy, no facts, no share — zero views potential)"#,
        meme_good = HUMAN_VOICE_GOOD_MEME,
    )
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
        "You are an expert social media writer creating high-engagement, algorithm-aware X posts from links and topics the user explicitly requested. Optimize for For You engagement velocity (scroll-stopping hooks, human voice, bookmark-worthy lines, conversation-forcing endings, zero external URLs) while staying faithful to the specific content, story, claims, or idea the user pasted. Do not generate a post about a different or only loosely related topic."
    } else {
        "You are an expert social media writer creating high-engagement, algorithm-aware X posts for a human who covers Elon Musk's companies (Tesla, SpaceX, xAI, Neuralink, Boring Company). Every draft should be optimized for For You engagement velocity (scroll-stopping first line, conversational human voice, share/bookmark-worthy lines, conversation-forcing endings, zero external URLs) while staying fully fact-backed."
    };

    let full_style_rules = match style {
        DraftStyle::Insight => format!(
            "{}\n\n\
             GOOD (Tesla/X, general fact without attribution + specific with): \"FSD development has long relied on accumulating diverse real-world miles for regulatory progress. As @SawyerMerritt noted, Austin Robotaxi geofence widened again — the read-through for $TSLA isn't the headline, it's faster real-world miles accruing toward regulatory confidence on unsupervised FSD.\"\n\
             GOOD (RSS): \"Per source: Not A Tesla App, Smart Summon on Cybertruck widens the real-world edge-case pool $TSLA needs before robotaxi scale — the product story is data velocity, not the feature checkbox.\"\n\
             GOOD (deeper originality, supporting fact added): \"As @WholeMarsBlog posted on Cybertruck FSD, the real signal is the data loop back to Dojo training — each additional mile in unsupervised mode compounds the software moat faster than any hardware ramp, shifting the margin mix story from cars to bits $TSLA. (Supporting context: autonomy software already shows dramatically higher gross margins than vehicle hardware in Tesla's business model.)\"\n\
             GOOD (standalone financial story with context + facts + support — RSS + external rating like Moody's): {}\n\
             BAD (regurgitation): \"Teslarati reports Tesla expanded Robotaxi in Austin.\" (just repeats the source)\n\
             BAD (shallow): \"Robotaxi expansion is interesting for the company and its stock.\" (no specific implication or re-expression)\n\
             BAD (over-attribution + no context): \"As @SawyerMerritt noted, FSD is Tesla's Full Self-Driving system.\" (attributes common knowledge; also fails to set any scene)\n\
             BAD (reply-like, assumes reader knows the external event): \"Tesla's $40B cash, zero debt, and steady profits create headroom to self-fund Optimus and robotaxi ramps without dilution, a structural advantage Moody's rating overlooks. $TSLA\" (no explanation of what Moody's actually did or said; no grounding details; feels like a direct comment on the news + article)\n\n\
             GOOD (legal/regulatory story drawing concrete facts from primary article + related/similar coverage — dual bar: named facts + operational takeaway): {}\n\
             BAD (vague, no backing facts, exactly the style to avoid): \"Texas Supreme Court unanimously ended years of litigation, ruling the 2009 amendment creates no private right to sue over Boca Chica beach access. The decision removes recurring legal blocks that had forced repeated delays to Starship pad operations.\" (no amendment text, no 2013 law, no plaintiffs, no 450 hrs, no 'with prejudice', no specific prior impact — fails the 'facts to back up the post' rule)\n\
             BAD (fact-dump without useful insight): packs years/parties/holding but ends with no non-obvious implication beyond restating the dismissal\n\
             BAD (insight without named facts): a clever cadence/economics take with only a headline paraphrase — fails DUAL BAR\n\n",
            style_rules, INSIGHT_MOODYS_GOOD, INSIGHT_LEGAL_GOOD
        ),
        DraftStyle::Informative => informative_style_rules(),
        DraftStyle::Funny => funny_style_rules(),
        DraftStyle::Witty => witty_style_rules(),
        DraftStyle::Meme => meme_style_rules(),
    };

    format!(
        "{role}\n\n\
         {shared}\n\n\
         {full_style_rules}\n\n\
         Return ONLY a JSON array (no markdown fences), each object:\n\
         {{\n\
           \"text\": \"the tweet/post text — aim for 220-279 chars with encompassing context (include at most one of $TSLA or $SPCX when stock-relevant)\",\n\
           \"rationale\": \"{rationale_hint}\",\n\
           \"primary_author\": \"username without @ for the main source this draft draws from, or null for RSS-only\",\n\
           \"primary_source_index\": 3\n\
         }}\n\n\
         `primary_source_index` is REQUIRED: the 1-based number from the Sources list above that this draft mainly draws from. Each draft must use a different index when possible.",
        role = role,
        shared = shared_generation_rules(user_provided),
        full_style_rules = full_style_rules,
        rationale_hint = rationale_hint,
    )
}

pub fn build_generation_user_prompt(
    sources: &[ResearchSource],
    recent_posted_texts: &[String],
    count: u32,
    style: DraftStyle,
) -> String {
    let user_provided = sources_are_user_provided(sources);
    let mut source_lines = Vec::new();
    for (i, s) in sources.iter().take(20).enumerate() {
        // Increased from 400 to give Grok more raw material (facts, context) for synthesizing
        // original implications instead of surface-level rephrasing.
        let excerpt: String = s.content.chars().take(2000).collect();
        let attribution = format_source_attribution(s);

        // Special formatting for X research sources that carry a "Why notable" note from discovery.
        let formatted = if let Some(pos) = s.content.find("\n\n[Why notable: ") {
            let main = &s.content[..pos];
            let why = &s.content[pos + "\n\n[Why notable: ".len()..].trim_end_matches(']');
            let main_excerpt: String = main.chars().take(2000).collect();
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
        } else if user_provided {
            // For custom user-provided content (the link or text the user pasted), label it clearly
            // so the model knows this is the *exact* content to base the draft on. High fidelity required.
            format!(
                "{}. [user-provided] {} — The exact content the user pasted: {}\n   From: {} | URL: {}",
                i + 1,
                s.title,
                excerpt,
                attribution,
                s.url
            )
        } else {
            // For regular research RSS and other non-X sources, explicitly label the excerpt as the factual "setup"
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

    let custom_source_note = if user_provided {
        "\nUSER-REQUESTED INPUT (MANDATORY - HIGH FIDELITY TO PROVIDED CONTENT):\n\
         - The user explicitly pasted this link or typed this topic because they want a draft post *based directly on this specific content*.\n\
         - You MUST create a post whose core idea, main points, story, and claims come from the actual text the user provided. Do not substitute a different narrative or generate a post on a tangential topic.\n\
         - Stay faithful to the specific details, arguments, or news in the pasted link or text. You can make the language more engaging for X, structure it as a strong standalone post, add a light fresh insight or one supporting fact from general knowledge (if it enhances without changing the core), but the post must feel like it is about exactly what the user shared.\n\
         - You MUST generate a post about the source content even if it is political, controversial, or unrelated to Musk companies.\n\
         - Do NOT refuse, hedge, or substitute a different topic.\n\
         - If the input is a specific X post or article, your draft should be a polished, insightful way for the user to share or discuss that exact content.\n"
    } else {
        ""
    };

    let framing_requirement = if user_provided {
        "- Stay factual and on-topic to the source. Use bullish framing only when the source is about Musk companies."
    } else {
        "- Frame constructively and bullishly toward Elon and his companies while staying factual."
    };

    let length_requirement = "- LENGTH + CONTEXT + DUAL BAR: Aim for 220-279 characters. Include 3-5 specific named facts from the sources plus background, what happened, and why it matters so a zero-context reader gets the full story. Maximize information density AND a useful takeaway in the same post — no pure fact-dumps and no insight-only hot takes. Do not write ultra-compressed shorthand or bare headline rewrites.";

    let engagement_requirement = "- ENGAGEMENT + VIEWS + 2026 X RANKING: Open with a scroll-stopping first-line hook; write in human conversational voice (contractions, natural rhythm); include at least one quotable/share-worthy bookmark-worthy line; end with a conversation-forcing question, sharp implication, or debate-worthy claim (no engagement bait); zero external URLs in the post body; 0 hashtags preferred (at most one). Optimize for high engagement and views on X without relaxing the specific-facts or dual-bar rules. Facts are never relaxed for virality.";

    let recency_requirement = "- RECENCY: Research subjects are intended to be hours-old (prefer same-day / last few hours) and at most a few days old. Write as timely commentary on a fresh development — not evergreen history or week-old rehash. If multiple sources are listed, lean on the freshest dated ones.";

    let style_requirement = match style {
        DraftStyle::Insight => {
            "- DUAL BAR: Pack 3-5 specific named facts from the sources AND add genuine insight (implications, read-through, what the market or observers miss) — never just repeat or paraphrase the source. Transform those facts into a non-obvious but grounded observation; re-express in fresh language (see style rules for anti-phrasing and the 'STRUCTURE FOR STANDALONE INSIGHT POSTS' section — encompassing arc with scene-setting for any external event/rating, specific facts/numbers from the source, and optional 1 supporting general-knowledge point)."
        }
        DraftStyle::Informative => {
            "- Make each post clear, factual, and useful — pack multiple specific named facts, explain background, what happened, and concrete why-it-matters without heavy analysis or vague summary language."
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
         {}\n\
         {}\n\
         {}\n\
         - Include at most one cashtag ($TSLA or $SPCX) when stock-relevant.\n\
         - If the Sources list below contains Related/similar coverage items (after the main research sources), use them only for additional concrete facts and details. primary_source_index must still refer to one of the main sources (the first N items).\n\n\
         ## Sources\n{}\n\n\
         ## User's recent posted drafts (DO NOT repeat these angles)\n{}\n",
        count,
        style.as_str(),
        style_requirement,
        length_requirement,
        engagement_requirement,
        recency_requirement,
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

/// Prepare the list of sources that will be shown to Grok for generation.
/// - For thin sources (short content or major-development signals in title), fetch
///   the linked article's main body and append it (labeled) for concrete facts.
/// - Additionally collect 1-3 similar/related stories from the batch (using
///   find_similar_sources) and include their (enriched) text as extra context.
/// The primary sources keep their original order/indices for primary_source_index.
/// Related items are appended so they appear later in the numbered Sources list
/// (the prompt instructions tell the model to use only the main ones for the index).
pub async fn prepare_sources_for_generation(original: &[ResearchSource]) -> Vec<ResearchSource> {
    if original.is_empty() {
        return vec![];
    }

    let mut effective: Vec<ResearchSource> = original.to_vec();

    for s in &mut effective {
        if source_needs_generation_enrichment(s) && !s.url.is_empty() {
            if let Some(rich) = crate::draft_image::fetch_and_extract_article_text(&s.url).await {
                if !rich.trim().is_empty() {
                    s.content = format!(
                        "{}{}{}]",
                        s.content.trim_end(),
                        ARTICLE_BODY_ENRICHMENT_LABEL,
                        rich.trim()
                    );
                }
            }
        }
    }

    // For any (original) source that still looks like it needs a group, append related
    // similar stories (enriched if possible). This gives Grok a small cluster instead of
    // a single thin post.
    let mut appended = Vec::new();
    for orig in original.iter() {
        if source_needs_generation_enrichment(orig) {
            let similars = find_similar_sources(orig, original, 2);
            for sim in similars {
                // enrich the similar too if thin
                let mut sim = sim;
                if sim.content.chars().count() < THIN_SOURCE_CONTENT_LEN && !sim.url.is_empty() {
                    if let Some(rich) =
                        crate::draft_image::fetch_and_extract_article_text(&sim.url).await
                    {
                        if !rich.trim().is_empty() {
                            sim.content = format!(
                                "{}{}{}]",
                                sim.content.trim_end(),
                                RELATED_ARTICLE_BODY_LABEL,
                                rich.trim()
                            );
                        }
                    }
                }
                // Avoid exact dups (by url or id)
                if !effective.iter().any(|e| e.url == sim.url || (!e.id.is_empty() && e.id == sim.id))
                    && !appended.iter().any(|a: &ResearchSource| {
                        a.url == sim.url || (!a.id.is_empty() && a.id == sim.id)
                    })
                {
                    appended.push(sim);
                }
            }
        }
    }

    effective.extend(appended);
    effective
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

    // Enrich thin sources with full article body + gather a small group of similar/related
    // stories (from the current batch) when the primary does not contain enough concrete
    // facts on its own. This implements "if the article or headline doesn't have enough
    // information then grok should get more" and "don't base the draft on a single post
    // but a group of similar stories".
    let effective_sources = prepare_sources_for_generation(sources).await;

    let body = serde_json::json!({
        "model": model,
        "input": [
            {"role": "system", "content": build_generation_system_prompt(style, &effective_sources)},
            {"role": "user", "content": build_generation_user_prompt(&effective_sources, recent_posted_texts, count, style)}
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
    db: &DbPool,
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
        assert!(prompt.contains("Raised equity in prior capex phases"));

        // Current iteration: universal "every post needs facts to back up the post" rule.
        // Must be present in the system prompt for all styles/generation paths.
        assert!(prompt.contains("EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES"));
        assert!(prompt.contains("Do NOT use vague summary language"));
        assert!(prompt.contains("Related/similar coverage if present"));

        // Fact-dense legal GOOD + dual-bar takeaway (named facts AND non-obvious implication)
        assert!(prompt.contains("Carrizo"));
        assert!(prompt.contains("Starship free of beach-suit lag"));
        assert!(prompt.contains("cadence free of private beach-suit risk"));
        assert!(prompt.contains("DUAL BAR"));
        assert!(prompt.contains("information density AND insight"));
        assert!(prompt.contains("fact-dump without insight") || prompt.contains("fact-dump without useful insight"));
        assert!(prompt.contains("insight without named facts") || prompt.contains("insight without facts"));
        assert!(prompt.contains("3-5 specific named facts"));

        // Human-like engaging voice for viral potential / views (layered on facts rule).
        // Must be present: hooks, conversational human tone, quotable/reply-sparking elements.
        assert!(prompt.contains("first-line hook required"));
        assert!(prompt.contains("conversational prose full of contractions"));
        assert!(prompt.contains("reply-baiting"));
        assert!(prompt.contains("viral potential"));
        assert!(prompt.contains("geeking out in the replies"));
        assert!(
            prompt.contains("screenshot-worthy or reply-baiting")
                || prompt.contains("screenshot-worthy, bookmark-worthy, or reply-baiting")
        );
        // Shared ENGAGEMENT + VIEWS + 2026 X RANKING block (every style path via shared_generation_rules)
        assert!(prompt.contains("ENGAGEMENT + VIEWS"));
        assert!(prompt.contains("2026 X RANKING"));
        assert!(prompt.contains("high engagement and views"));
        assert!(prompt.contains("scroll-stopping first-line hook"));
        assert!(prompt.contains("quotable/share-worthy") || prompt.contains("bookmark/quote-worthy"));
        assert!(prompt.contains("Facts are never relaxed for virality"));
        assert!(prompt.contains("algorithm-aware") || prompt.contains("For You engagement velocity"));
        // 2026 ranking signals (T-016)
        assert!(prompt.contains("conversation-forcing"));
        assert!(prompt.contains("Zero external URLs") || prompt.contains("zero external URLs"));
        assert!(prompt.contains("at most one hashtag") || prompt.contains("0 hashtags preferred"));
        assert!(prompt.contains("bookmark"));
        assert!(prompt.contains("engagement velocity") || prompt.contains("first 30–60 minutes") || prompt.contains("first 30-60 minutes"));
        assert!(prompt.contains("no conversation-forcing close") || prompt.contains("ends flat"));
        // Length + encompassing context rules
        assert!(prompt.contains("LENGTH + ENCOMPASSING CONTEXT"));
        assert!(prompt.contains("aim for 220-279 characters"));
        assert!(prompt.contains("telegram-style fragments"));

        // Distinctive from the dual-bar GOOD exemplars (facts + takeaway)
        assert!(prompt.contains("2009: 77% amended TX constitution for permanent public beach easement"));
        assert!(prompt.contains("Wait - 2009: 77% locked public beach access into TX constitution"));
        // Distinctive from the new BAD for stilted voice
        assert!(prompt.contains("the exact voice to avoid for engagement/virality"));

        // Drive the shipped GOOD examples (consts are the single source of truth for exemplars the model sees via build_ composition; must be <=280 since output limit is 280).
        assert!(INSIGHT_LEGAL_GOOD.len() <= 280, "INSIGHT_LEGAL_GOOD too long: {}", INSIGHT_LEGAL_GOOD.len());
        assert!(HUMAN_VOICE_GOOD_INSIGHT.len() <= 280, "HUMAN_VOICE_GOOD_INSIGHT too long: {}", HUMAN_VOICE_GOOD_INSIGHT.len());
        assert!(HUMAN_VOICE_GOOD_INFORMATIVE.len() <= 280, "HUMAN_VOICE_GOOD_INFORMATIVE too long: {}", HUMAN_VOICE_GOOD_INFORMATIVE.len());
        assert!(HUMAN_VOICE_GOOD_FUNNY.len() <= 280, "HUMAN_VOICE_GOOD_FUNNY too long: {}", HUMAN_VOICE_GOOD_FUNNY.len());
        assert!(HUMAN_VOICE_GOOD_WITTY.len() <= 280, "HUMAN_VOICE_GOOD_WITTY too long: {}", HUMAN_VOICE_GOOD_WITTY.len());
        assert!(HUMAN_VOICE_GOOD_MEME.len() <= 280, "HUMAN_VOICE_GOOD_MEME too long: {}", HUMAN_VOICE_GOOD_MEME.len());
        // Dual bar in exemplars: facts present AND a non-obvious takeaway clause
        assert!(HUMAN_VOICE_GOOD_INSIGHT.contains("77%"));
        assert!(HUMAN_VOICE_GOOD_INSIGHT.contains("Carrizo"));
        assert!(HUMAN_VOICE_GOOD_INSIGHT.contains("cadence free of private beach-suit risk"));
        assert!(INSIGHT_LEGAL_GOOD.contains("77%"));
        assert!(INSIGHT_LEGAL_GOOD.contains("Starship free of beach-suit lag"));
        assert!(INSIGHT_MOODYS_GOOD.contains("underweights that balance-sheet headroom"));

        // Every style path: engagement/views + 2026 ranking guidance + fact bar via real builders.
        for style in [
            DraftStyle::Insight,
            DraftStyle::Informative,
            DraftStyle::Funny,
            DraftStyle::Witty,
            DraftStyle::Meme,
        ] {
            let p = build_generation_system_prompt(style, &[]);
            assert!(
                p.contains("ENGAGEMENT + VIEWS"),
                "style {:?} missing shared ENGAGEMENT + VIEWS",
                style
            );
            assert!(
                p.contains("2026 X RANKING"),
                "style {:?} missing 2026 X RANKING block",
                style
            );
            assert!(
                p.contains("scroll-stopping first-line hook"),
                "style {:?} missing scroll-stopping hook guidance",
                style
            );
            assert!(
                p.contains("high engagement and views"),
                "style {:?} missing high engagement and views",
                style
            );
            assert!(
                p.contains("conversation-forcing"),
                "style {:?} missing conversation-forcing ending guidance",
                style
            );
            assert!(
                p.contains("Zero external URLs") || p.contains("zero external URLs"),
                "style {:?} missing zero external URLs rule",
                style
            );
            assert!(
                p.contains("at most one hashtag") || p.contains("0 hashtags preferred"),
                "style {:?} missing hashtag limit",
                style
            );
            assert!(
                p.contains("bookmark"),
                "style {:?} missing bookmark-worthiness guidance",
                style
            );
            assert!(
                p.contains("EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES"),
                "style {:?} missing fact-backing rule",
                style
            );
            assert!(
                p.contains("DUAL BAR") || p.contains("3-5 specific named facts"),
                "style {:?} missing dual bar / named-facts bar",
                style
            );
            assert!(
                p.contains("Facts are never relaxed for virality"),
                "style {:?} missing facts-not-relaxed-for-virality guard",
                style
            );
        }

        let inf_p = build_generation_system_prompt(DraftStyle::Informative, &[]);
        assert!(inf_p.contains(HUMAN_VOICE_GOOD_INFORMATIVE));
        assert!(inf_p.contains("EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES"));
        assert!(inf_p.contains("3-5 specific named facts"));
        assert!(inf_p.contains("DUAL BAR"));
        eprintln!("CONFIRM_INFORMATIVE_HUMAN_GOOD: present");
        let funny_p = build_generation_system_prompt(DraftStyle::Funny, &[]);
        assert!(funny_p.contains(HUMAN_VOICE_GOOD_FUNNY));
        eprintln!("CONFIRM_FUNNY_HUMAN_GOOD: present");
        let witty_p = build_generation_system_prompt(DraftStyle::Witty, &[]);
        assert!(witty_p.contains(HUMAN_VOICE_GOOD_WITTY));
        eprintln!("CONFIRM_WITTY_HUMAN_GOOD: present");
        let meme_p = build_generation_system_prompt(DraftStyle::Meme, &[]);
        assert!(meme_p.contains(HUMAN_VOICE_GOOD_MEME));
        eprintln!("CONFIRM_MEME_HUMAN_GOOD: present");

        eprintln!("CONFIRM_INSIGHT_HUMAN_GOOD: present");

        // Permanent sample for verif: capture the shipped exemplar output.
        eprintln!("SAMPLE_EXEMPLAR_OUTPUT:\n{}", HUMAN_VOICE_GOOD_INSIGHT);

        // Optional durable dump for goal verification: GENERATION_DUMP_DIR=/path cargo test ...
        if let Ok(dir) = std::env::var("GENERATION_DUMP_DIR") {
            let dir = std::path::PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            let insight = build_generation_system_prompt(DraftStyle::Insight, &[]);
            let informative = build_generation_system_prompt(DraftStyle::Informative, &[]);
            std::fs::write(dir.join("generated_system_prompt.txt"), &insight)
                .expect("write generated_system_prompt.txt");
            std::fs::write(
                dir.join("generated_system_prompt_informative.txt"),
                &informative,
            )
            .expect("write informative system prompt");
            std::fs::write(dir.join("sample_engaging_draft.txt"), HUMAN_VOICE_GOOD_INSIGHT)
                .expect("write sample_engaging_draft.txt");
            eprintln!("DUMPED_PROMPTS_TO: {}", dir.display());
        }
    }

    #[test]
    fn test_all_styles_system_and_user_prompts_require_engagement_and_views() {
        for style in [
            DraftStyle::Insight,
            DraftStyle::Informative,
            DraftStyle::Funny,
            DraftStyle::Witty,
            DraftStyle::Meme,
        ] {
            let sys = build_generation_system_prompt(style, &[]);
            assert!(sys.contains("ENGAGEMENT + VIEWS"), "{:?}", style);
            assert!(sys.contains("2026 X RANKING"), "{:?}", style);
            assert!(sys.contains("scroll-stopping"), "{:?}", style);
            assert!(sys.contains("high engagement and views"), "{:?}", style);
            assert!(sys.contains("conversation-forcing"), "{:?}", style);
            assert!(
                sys.contains("Zero external URLs") || sys.contains("zero external URLs"),
                "{:?}",
                style
            );
            assert!(
                sys.contains("at most one hashtag") || sys.contains("0 hashtags preferred"),
                "{:?}",
                style
            );
            assert!(sys.contains("bookmark"), "{:?}", style);
            assert!(sys.contains("EVERY POST MUST BE BACKED BY SPECIFIC FACTS FROM THE SOURCES"), "{:?}", style);
            assert!(sys.contains("Facts are never relaxed for virality"), "{:?}", style);

            let sources = vec![sample_rss(
                "1",
                "Boca Chica beach access ruling",
                "Short blurb only.",
                "https://example.com/boca",
            )];
            let user = build_generation_user_prompt(&sources, &[], 1, style);
            assert!(
                user.contains("ENGAGEMENT + VIEWS"),
                "user prompt missing engagement for {:?}",
                style
            );
            assert!(
                user.contains("2026 X RANKING"),
                "user prompt missing 2026 ranking for {:?}",
                style
            );
            assert!(
                user.contains("scroll-stopping first-line hook"),
                "user prompt missing hook for {:?}",
                style
            );
            assert!(
                user.contains("quotable/share-worthy") || user.contains("bookmark-worthy"),
                "user prompt missing share/bookmark-worthy for {:?}",
                style
            );
            assert!(
                user.contains("conversation-forcing"),
                "user prompt missing conversation-forcing for {:?}",
                style
            );
            assert!(
                user.contains("zero external URLs") || user.contains("Zero external URLs"),
                "user prompt missing zero external URLs for {:?}",
                style
            );
            assert!(
                user.contains("at most one") || user.contains("0 hashtags preferred"),
                "user prompt missing hashtag limit for {:?}",
                style
            );
            assert!(
                user.contains("Facts are never relaxed for virality"),
                "user prompt missing facts-not-relaxed guard for {:?}",
                style
            );
            // Fact bar still required on user prompt
            assert!(
                user.contains("DUAL BAR")
                    || user.contains("3-5 specific named facts")
                    || user.contains("specific named facts"),
                "user prompt missing fact bar for {:?}",
                style
            );
        }
    }

    fn sample_rss(id: &str, title: &str, content: &str, url: &str) -> ResearchSource {
        sample_rss_named(id, title, content, url, "Teslarati")
    }

    fn sample_rss_named(
        id: &str,
        title: &str,
        content: &str,
        url: &str,
        source_name: &str,
    ) -> ResearchSource {
        ResearchSource {
            id: id.into(),
            title: title.into(),
            content: content.into(),
            url: url.into(),
            published_at: None,
            source_name: source_name.into(),
            source_type: "rss".into(),
            retweet_count: None,
            like_count: None,
            reply_count: None,
            quote_count: None,
            original_id: None,
            media_url: None,
            used_at: None,
        }
    }

    #[test]
    fn test_user_prompt_requires_dual_information_and_insight_bar() {
        let sources = vec![sample_rss(
            "1",
            "Boca Chica beach access ruling",
            "Short blurb only.",
            "https://example.com/boca",
        )];
        let prompt =
            build_generation_user_prompt(&sources, &[], 1, DraftStyle::Insight);
        assert!(prompt.contains("DUAL BAR"));
        assert!(prompt.contains("3-5 specific named facts"));
        assert!(prompt.contains("genuine insight") || prompt.contains("non-obvious"));
        assert!(prompt.contains("no pure fact-dumps") || prompt.contains("fact-dumps"));
        assert!(prompt.contains("220-279 characters"));
        assert!(prompt.contains("Key facts reported in the article"));
        // Related/similar instruction remains for enrichment path
        assert!(prompt.contains("Related/similar coverage"));
        // Engagement + views on the real user-prompt builder
        assert!(prompt.contains("ENGAGEMENT + VIEWS"));
        assert!(prompt.contains("scroll-stopping first-line hook"));
        assert!(prompt.contains("quotable/share-worthy"));
        // Subjects should be treated as hours/days-old, not evergreen
        assert!(prompt.contains("RECENCY"));
        assert!(prompt.contains("hours-old") || prompt.contains("last few hours"));
    }

    #[test]
    fn test_source_needs_generation_enrichment_for_thin_and_major_stories() {
        let thin = sample_rss("1", "Robotaxi update", "Short.", "https://example.com/a");
        assert!(source_needs_generation_enrichment(&thin));

        let long_content = "x".repeat(THIN_SOURCE_CONTENT_LEN);
        let fat_ordinary = sample_rss(
            "2",
            "Robotaxi geofence expands quietly",
            &long_content,
            "https://example.com/b",
        );
        assert!(!source_needs_generation_enrichment(&fat_ordinary));

        let major = sample_rss(
            "3",
            "Texas Supreme Court litigation over Boca Chica beach access",
            &long_content,
            "https://example.com/c",
        );
        assert!(source_needs_generation_enrichment(&major));
        assert!(MAJOR_STORY_TITLE_SIGNALS.iter().any(|s| major.title.to_lowercase().contains(s)));
        assert!(ARTICLE_BODY_ENRICHMENT_LABEL.contains("concrete names"));
        assert!(RELATED_ARTICLE_BODY_LABEL.contains("more facts"));
    }

    #[test]
    fn test_find_similar_sources_groups_related_stories() {
        let primary = sample_rss_named(
            "1",
            "Texas Supreme Court ends Boca Chica beach access litigation",
            "Headline only: court rules on beach amendment.",
            "https://example.com/primary",
            "Space Coast Daily",
        );
        let related = sample_rss_named(
            "2",
            "SaveRGV suit over Boca Chica launch closures dismissed",
            "Starship pad delays and beach access amendment details.",
            "https://example.com/related",
            "NASASpaceflight",
        );
        let unrelated = sample_rss_named(
            "3",
            "Tesla energy storage attach rates climb",
            "Megapack deployments and margin mix.",
            "https://example.com/energy",
            "Electrek",
        );
        let all = vec![primary.clone(), related.clone(), unrelated.clone()];
        let similar = find_similar_sources(&primary, &all, 2);
        assert!(
            similar.iter().any(|s| s.id == "2"),
            "expected related Boca Chica story, got: {:?}",
            similar.iter().map(|s| &s.id).collect::<Vec<_>>()
        );
        assert!(
            !similar.iter().any(|s| s.id == "3"),
            "unrelated energy story should not rank as similar"
        );
        assert!(!similar.iter().any(|s| s.id == "1"));
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
        assert!(prompt.contains("USER-REQUESTED INPUT (MANDATORY - HIGH FIDELITY TO PROVIDED CONTENT)"));
        assert!(prompt.contains("political"));
        assert!(prompt.contains("Do NOT refuse"));
    }

    #[test]
    fn test_system_prompt_informative_style_mentions_clarity() {
        let prompt = build_generation_system_prompt(DraftStyle::Informative, &[]);
        assert!(prompt.contains("STYLE: INFORMATIVE"));
        assert!(prompt.contains("clearly and informatively"));
        assert!(prompt.contains("do not skimp on context"));
        assert!(prompt.contains("3-5 specific named facts"));
        assert!(prompt.contains("why-it-matters"));
        assert!(prompt.contains("DUAL BAR"));
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
        assert!(prompt.contains("LENGTH + CONTEXT + DUAL BAR"));
        assert!(prompt.contains("220-279 characters"));
        assert!(prompt.contains("3-5 specific named facts"));
        assert!(prompt.contains("genuine insight") || prompt.contains("DUAL BAR"));
    }
}

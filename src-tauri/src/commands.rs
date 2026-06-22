use crate::{
    constants::{
        draft_status, settings, DraftStyle, DEFAULT_DRAFT_COUNT, DEFAULT_GROK_MODEL,
        MAX_DRAFT_COUNT, RESEARCH_SOURCE_LIMIT,
    },
    custom_source, draft_image, generation, research, x_media, x_post, AppState, DbPool,
};
use std::path::Path;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Draft {
    pub id: String,
    pub text: String,
    pub sources_json: String,
    pub image_url: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub posted_at: Option<String>,
    pub x_post_id: Option<String>,
    pub generation_rationale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftInput {
    pub text: String,
    pub sources_json: String,
    pub image_url: Option<String>,
    pub generation_rationale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDraftInput {
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub status: Option<String>,
    pub generation_rationale: Option<String>,
}

/// Create a new draft in the queue (Tauri command entrypoint)
#[tauri::command]
pub async fn create_draft(
    state: State<'_, AppState>,
    input: CreateDraftInput,
) -> Result<Draft, String> {
    create_draft_db(&state.db, input).await
}

/// Internal implementation — takes a raw pool so it is easy to test and reuse.
pub async fn create_draft_db(db: &DbPool, input: CreateDraftInput) -> Result<Draft, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let draft = Draft {
        id: id.clone(),
        text: input.text,
        sources_json: input.sources_json,
        image_url: input.image_url,
        status: draft_status::PENDING.to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        posted_at: None,
        x_post_id: None,
        generation_rationale: input.generation_rationale,
    };

    sqlx::query(
        r#"
        INSERT INTO drafts (id, text, sources_json, image_url, status, created_at, updated_at, generation_rationale)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&draft.id)
    .bind(&draft.text)
    .bind(&draft.sources_json)
    .bind(&draft.image_url)
    .bind(&draft.status)
    .bind(&draft.created_at)
    .bind(&draft.updated_at)
    .bind(&draft.generation_rationale)
    .execute(db)
    .await
    .map_err(|e| format!("Failed to create draft: {}", e))?;

    Ok(draft)
}

/// Get all drafts, optionally filtered by status (Tauri command entrypoint)
#[tauri::command]
pub async fn get_drafts(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Draft>, String> {
    get_drafts_db(&state.db, status).await
}

/// Internal implementation — easy to call from tests with an in-memory database.
pub async fn get_drafts_db(
    db: &DbPool,
    status: Option<String>,
) -> Result<Vec<Draft>, String> {
    let drafts = if let Some(s) = status {
        sqlx::query_as::<_, Draft>(
            "SELECT * FROM drafts WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(s)
        .fetch_all(db)
        .await
    } else {
        sqlx::query_as::<_, Draft>("SELECT * FROM drafts ORDER BY created_at DESC")
            .fetch_all(db)
            .await
    };

    drafts.map_err(|e| format!("Failed to fetch drafts: {}", e))
}

/// Get a single draft by ID (Tauri command entrypoint)
#[tauri::command]
pub async fn get_draft(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Draft>, String> {
    get_draft_db(&state.db, id).await
}

pub async fn get_draft_db(db: &DbPool, id: String) -> Result<Option<Draft>, String> {
    sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Failed to fetch draft: {}", e))
}

/// Update an existing draft (Tauri command entrypoint)
#[tauri::command]
pub async fn update_draft(
    state: State<'_, AppState>,
    id: String,
    input: UpdateDraftInput,
) -> Result<(), String> {
    update_draft_db(&state.db, id, input).await
}

/// Internal implementation. Builds a dynamic UPDATE while keeping the code readable.
pub async fn update_draft_db(
    db: &DbPool,
    id: String,
    input: UpdateDraftInput,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    // Collect the fields we actually want to update
    let mut sets: Vec<(&str, String)> = vec![("updated_at", now.clone())];

    if let Some(text) = input.text {
        sets.push(("text", text));
    }
    if let Some(image_url) = input.image_url {
        sets.push(("image_url", image_url));
    }
    if let Some(status) = input.status {
        sets.push(("status", status));
    }
    if let Some(generation_rationale) = input.generation_rationale {
        sets.push(("generation_rationale", generation_rationale));
    }

    // Build the SET clause safely
    let set_clause = sets
        .iter()
        .map(|(col, _)| format!("{} = ?", col))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "UPDATE drafts SET {} WHERE id = ?",
        set_clause
    );

    let mut q = sqlx::query(&sql);

    // Bind the values in order
    for (_, value) in &sets {
        q = q.bind(value);
    }
    q = q.bind(id);

    q.execute(db)
        .await
        .map_err(|e| format!("Failed to update draft: {}", e))?;

    Ok(())
}

/// Delete a draft (Tauri command entrypoint)
#[tauri::command]
pub async fn delete_draft(state: State<'_, AppState>, id: String) -> Result<(), String> {
    delete_draft_db(&state.db, id).await
}

pub async fn delete_draft_db(db: &DbPool, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM drafts WHERE id = ?")
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to delete draft: {}", e))?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearPendingDraftsResult {
    pub deleted: u64,
}

#[tauri::command]
pub async fn clear_pending_drafts(state: State<'_, AppState>) -> Result<ClearPendingDraftsResult, String> {
    clear_pending_drafts_db(&state.db).await
}

pub async fn clear_pending_drafts_db(db: &DbPool) -> Result<ClearPendingDraftsResult, String> {
    let result = sqlx::query("DELETE FROM drafts WHERE status = ?")
        .bind(draft_status::PENDING)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to clear pending drafts: {}", e))?;

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE status = ?")
        .bind(draft_status::PENDING)
        .fetch_one(db)
        .await
        .map_err(|e| format!("Failed to verify pending draft deletion: {}", e))?;

    if remaining > 0 {
        return Err(format!(
            "Clear incomplete: {} pending draft(s) still in database",
            remaining
        ));
    }

    Ok(ClearPendingDraftsResult {
        deleted: result.rows_affected(),
    })
}

/// Mark a draft as successfully posted (Tauri command entrypoint)
#[tauri::command]
pub async fn mark_draft_posted(
    state: State<'_, AppState>,
    id: String,
    x_post_id: String,
) -> Result<(), String> {
    mark_draft_posted_db(&state.db, id, x_post_id).await
}

pub async fn mark_draft_posted_db(
    db: &DbPool,
    id: String,
    x_post_id: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE drafts 
        SET status = ?,
            x_post_id = ?, 
            posted_at = ?, 
            updated_at = ?
        WHERE id = ?
        "#
    )
    .bind(draft_status::POSTED)
    .bind(x_post_id)
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(db)
    .await
    .map_err(|e| format!("Failed to mark draft as posted: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> DbPool {
        // Use AnyPool + sqlite memory URL for fast isolated tests even when the app
        // is configured for a remote Postgres DB via DATABASE_URL.
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory sqlite pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("failed to run migrations on test database");

        pool
    }

    #[tokio::test]
    async fn test_create_and_get_draft_via_repository() {
        let db = create_test_pool().await;

        let input = CreateDraftInput {
            text: "Tesla delivered record numbers in Q2".to_string(),
            sources_json: r#"[{"type":"x","id":"abc123"}]"#.to_string(),
            image_url: Some("https://example.com/image.jpg".to_string()),
            generation_rationale: Some("The margin story from energy attach rates is under-appreciated.".to_string()),
        };

        // Use the real reusable function
        let created = create_draft_db(&db, input).await.expect("create failed");

        assert_eq!(created.status, "pending");
        assert!(!created.id.is_empty());
        assert_eq!(created.generation_rationale, Some("The margin story from energy attach rates is under-appreciated.".to_string()));

        // Fetch via the repository function too
        let all = get_drafts_db(&db, None).await.expect("get failed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, created.id);

        let single = get_draft_db(&db, created.id.clone()).await.expect("get one failed");
        assert!(single.is_some());
    }

    #[tokio::test]
    async fn test_update_draft_via_repository() {
        let db = create_test_pool().await;

        let created = create_draft_db(&db, CreateDraftInput {
            text: "Original text".to_string(),
            sources_json: "[]".to_string(),
            image_url: None,
            generation_rationale: None,
        }).await.unwrap();

        let update = UpdateDraftInput {
            text: Some("Updated with fresh analysis".to_string()),
            image_url: None,
            status: Some("pending".to_string()),
            generation_rationale: None,
        };

        update_draft_db(&db, created.id.clone(), update).await.expect("update failed");

        let fetched = get_draft_db(&db, created.id).await.unwrap().unwrap();
        assert_eq!(fetched.text, "Updated with fresh analysis");
    }

    #[tokio::test]
    async fn test_clear_pending_drafts_keeps_posted() {
        let db = create_test_pool().await;

        let pending = create_draft_db(
            &db,
            CreateDraftInput {
                text: "Pending draft".to_string(),
                sources_json: "[]".to_string(),
                image_url: None,
                generation_rationale: None,
            },
        )
        .await
        .unwrap();

        let posted = create_draft_db(
            &db,
            CreateDraftInput {
                text: "Posted draft".to_string(),
                sources_json: "[]".to_string(),
                image_url: None,
                generation_rationale: None,
            },
        )
        .await
        .unwrap();
        mark_draft_posted_db(&db, posted.id.clone(), "tweet-1".to_string())
            .await
            .unwrap();

        let result = clear_pending_drafts_db(&db).await.expect("clear pending");
        assert_eq!(result.deleted, 1);

        let all = get_drafts_db(&db, None).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, posted.id);
        assert_eq!(all[0].status, "posted");

        let gone = get_draft_db(&db, pending.id).await.unwrap();
        assert!(gone.is_none());
    }

    // ============================================
    // Settings / API Key tests (happy + unhappy paths)
    // ============================================

    #[tokio::test]
    async fn test_set_and_get_setting_happy_path() {
        let db = create_test_pool().await;

        // Set a value
        set_setting_db(&db, "xai_api_key".to_string(), "sk-test-12345".to_string())
            .await
            .expect("set_setting failed");

        // Get it back
        let value = get_setting_db(&db, "xai_api_key".to_string())
            .await
            .expect("get_setting failed");

        assert_eq!(value, Some("sk-test-12345".to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting_returns_none() {
        let db = create_test_pool().await;

        let value = get_setting_db(&db, "nonexistent_key".to_string())
            .await
            .expect("get_setting failed");

        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_set_setting_overwrites_existing_value() {
        let db = create_test_pool().await;

        // Set initial value
        set_setting_db(&db, "xai_api_key".to_string(), "old-key".to_string())
            .await
            .expect("set failed");

        // Overwrite
        set_setting_db(&db, "xai_api_key".to_string(), "new-key-999".to_string())
            .await
            .expect("set failed");

        let value = get_setting_db(&db, "xai_api_key".to_string())
            .await
            .expect("get failed");

        assert_eq!(value, Some("new-key-999".to_string()));
    }

    #[tokio::test]
    async fn test_set_empty_value_is_allowed() {
        let db = create_test_pool().await;

        set_setting_db(&db, "empty_key".to_string(), "".to_string())
            .await
            .expect("set empty value failed");

        let value = get_setting_db(&db, "empty_key".to_string())
            .await
            .expect("get failed");

        assert_eq!(value, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_reset_research_data_clears_runs_and_sources() {
        let db = create_test_pool().await;

        // Seed a run + source (simulating what run_research does)
        let run_id = "test-run-reset-1";
        sqlx::query("INSERT INTO research_runs (id, run_at, source) VALUES (?, ?, ?)")
            .bind(run_id)
            .bind("2026-01-01T00:00:00Z")
            .bind("both")
            .execute(&db)
            .await
            .expect("insert run failed");

        sqlx::query(
            r#"INSERT INTO research_sources (id, run_id, title, content, url, source_name, source_type)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#
        )
            .bind("src-reset-1")
            .bind(run_id)
            .bind("Some Tesla news")
            .bind("Details here")
            .bind("https://example.com/tesla")
            .bind("Teslarati")
            .bind("rss")
            .execute(&db)
            .await
            .expect("insert source failed");

        // Pre-check counts via direct queries
        let run_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_runs")
            .fetch_one(&db)
            .await
            .expect("count runs");
        let src_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_sources")
            .fetch_one(&db)
            .await
            .expect("count sources");
        assert_eq!(run_count, 1, "should have seeded run");
        assert_eq!(src_count, 1, "should have seeded source");

        // Also verify get_all_historical would see it (join)
        let hist: Vec<HistoricalResearchSource> = sqlx::query_as(
            r#"SELECT rs.*, rr.run_at FROM research_sources rs JOIN research_runs rr ON rs.run_id=rr.id"#
        )
        .fetch_all(&db)
        .await
        .expect("hist fetch");
        assert_eq!(hist.len(), 1);

        // Execute reset
        let result = reset_research_data_db(&db).await.expect("reset_research_data_db failed");
        assert_eq!(result.deleted_sources, 1);
        assert_eq!(result.deleted_runs, 1);

        // Post-check: both tables empty
        let run_count2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_runs")
            .fetch_one(&db)
            .await
            .expect("count runs after");
        let src_count2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_sources")
            .fetch_one(&db)
            .await
            .expect("count sources after");
        assert_eq!(run_count2, 0, "runs should be cleared by reset");
        assert_eq!(src_count2, 0, "sources should be cleared by reset");

        // get_all should also return empty
        let hist2: Vec<HistoricalResearchSource> = sqlx::query_as(
            r#"SELECT rs.*, rr.run_at FROM research_sources rs JOIN research_runs rr ON rs.run_id=rr.id"#
        )
        .fetch_all(&db)
        .await
        .expect("hist fetch after");
        assert_eq!(hist2.len(), 0);
    }
}

async fn load_grok_settings(db: &DbPool) -> Result<(String, String), String> {
    let xai_key = get_setting_db(db, settings::XAI_API_KEY.to_string())
        .await?
        .unwrap_or_default();
    let grok_model = get_setting_db(db, settings::GROK_MODEL.to_string())
        .await?
        .unwrap_or_else(|| DEFAULT_GROK_MODEL.to_string());
    Ok((xai_key, grok_model))
}

async fn require_xai_api_key(db: &DbPool) -> Result<String, String> {
    get_setting_db(db, settings::XAI_API_KEY.to_string())
        .await?
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "xAI API key is required. Set it in Settings.".to_string())
}

async fn require_setting(db: &DbPool, key: &str, label: &str) -> Result<String, String> {
    get_setting_db(db, key.to_string())
        .await?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} is not set in Settings.", label))
}

async fn fetch_sources_for_run(
    db: &DbPool,
    run_id: &str,
) -> Result<Vec<research::ResearchSource>, String> {
    sqlx::query_as("SELECT * FROM research_sources WHERE run_id = ? ORDER BY published_at DESC")
        .bind(run_id)
        .fetch_all(db)
        .await
        .map_err(|e| format!("Failed to fetch sources for run: {}", e))
}

async fn fetch_run_with_sources(
    db: &DbPool,
    run_id: &str,
) -> Result<ResearchRunWithSources, String> {
    let run: ResearchRun = sqlx::query_as("SELECT * FROM research_runs WHERE id = ?")
        .bind(run_id)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Failed to fetch run: {}", e))?
        .ok_or_else(|| format!("Research run '{}' not found.", run_id))?;

    let sources = fetch_sources_for_run(db, &run.id).await?;
    Ok(ResearchRunWithSources { run, sources })
}

async fn fetch_latest_run_with_sources(
    db: &DbPool,
) -> Result<Option<ResearchRunWithSources>, String> {
    let run: Option<ResearchRun> = sqlx::query_as(
        "SELECT * FROM research_runs ORDER BY run_at DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Failed to fetch latest run: {}", e))?;

    match run {
        Some(run) => {
            let sources = fetch_sources_for_run(db, &run.id).await?;
            Ok(Some(ResearchRunWithSources { run, sources }))
        }
        None => Ok(None),
    }
}

struct DraftSourceContext {
    sources: Vec<research::ResearchSource>,
    normalized_text: String,
    primary_index: Option<usize>,
}

fn build_draft_source_context(draft: &Draft) -> DraftSourceContext {
    let sources: Vec<research::ResearchSource> =
        serde_json::from_str(&draft.sources_json).unwrap_or_default();
    let normalized_text = x_media::normalize_source_mentions(&draft.text, &sources);
    let primary_index = x_media::match_primary_source(&normalized_text, &sources)
        .and_then(|primary| sources.iter().position(|source| source.id == primary.id));

    DraftSourceContext {
        sources,
        normalized_text,
        primary_index,
    }
}

fn draft_has_stored_image(draft: &Draft) -> bool {
    draft.image_url.as_ref().is_some_and(|url| !url.is_empty())
}

async fn maybe_resolve_preview_image(
    db: &DbPool,
    app_data_dir: &Path,
    draft_id: &str,
    draft: &Draft,
    creds: Option<&x_post::XCredentials>,
    should_resolve: bool,
) -> Result<(), String> {
    if !should_resolve {
        return Ok(());
    }

    let context = build_draft_source_context(draft);
    let primary = context
        .primary_index
        .and_then(|index| context.sources.get(index));
    let xai_key = load_grok_settings(db).await.ok().map(|(key, _)| key);

    let preview = draft_image::resolve_draft_image_url(draft_image::DraftImageRequest {
        draft_id,
        draft_text: &context.normalized_text,
        draft_image_url: draft.image_url.as_deref(),
        primary_source: primary,
        x_credentials: creds,
        xai_api_key: xai_key.as_deref(),
        app_data_dir: Some(app_data_dir),
    })
    .await?;

    if let Some(url) = preview {
        update_draft_db(
            db,
            draft_id.to_string(),
            UpdateDraftInput {
                text: None,
                image_url: Some(url),
                status: None,
                generation_rationale: None,
            },
        )
        .await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn fetch_research_sources(state: State<'_, AppState>) -> Result<Vec<research::ResearchSource>, String> {
    let (xai_key, grok_model) = load_grok_settings(&state.db).await?;
    let mut sources = research::fetch_rss_sources().await?;

    if !xai_key.is_empty() {
        match research::fetch_grok_discovered_x_sources(&xai_key, &grok_model).await {
            Ok(grok_sources) => sources.extend(grok_sources),
            Err(e) => log::warn!("Grok X discovery failed: {}", e),
        }
    }

    sources.sort_by(|a, b| {
        let a_prio = if a.source_type == "x_grok" { 0 } else { 1 };
        let b_prio = if b.source_type == "x_grok" { 0 } else { 1 };
        if a_prio != b_prio {
            return a_prio.cmp(&b_prio);
        }
        b.published_at.cmp(&a.published_at)
    });

    sources.truncate(RESEARCH_SOURCE_LIMIT);
    Ok(sources)
}



// ============================================
// Research Run Persistence (Current + Historical tabs)
// ============================================

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct ResearchRun {
    pub id: String,
    pub run_at: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResearchRunWithSources {
    pub run: ResearchRun,
    pub sources: Vec<research::ResearchSource>,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct HistoricalResearchSource {
    pub id: String,
    pub run_id: String,
    pub title: String,
    pub content: String,
    pub url: String,
    pub published_at: Option<String>,
    pub source_name: String,
    pub source_type: String,
    pub retweet_count: Option<i64>,
    pub like_count: Option<i64>,
    pub reply_count: Option<i64>,
    pub quote_count: Option<i64>,
    pub original_id: Option<String>,
    pub media_url: Option<String>,
    pub used_at: Option<String>,
    pub run_at: String,
}

#[tauri::command]
pub async fn run_research(state: State<'_, AppState>, mode: Option<String>) -> Result<ResearchRunWithSources, String> {
    let mode = mode.unwrap_or_else(|| "both".to_string()).to_lowercase();
    log::info!("run_research invoked with mode='{}'", mode);

    let mut sources: Vec<research::ResearchSource> = Vec::new();

    if mode == "rss" || mode == "both" {
        log::info!("run_research: fetching RSS sources for mode {}", mode);
        let rss = research::fetch_rss_sources().await?;
        log::info!("run_research: got {} RSS sources", rss.len());
        sources.extend(rss);
    }

    if mode == "x" || mode == "both" {
        let (xai_key, grok_model) = load_grok_settings(&state.db).await?;

        log::info!("run_research: xAI key present for X mode? {}", !xai_key.is_empty());
        log::info!("run_research: using Grok model: {}", grok_model);

        if xai_key.is_empty() {
            return Err("xAI API key is required to run X (Grok) research.".to_string());
        }

        log::info!("run_research: calling fetch_grok_discovered_x_sources");
        match research::fetch_grok_discovered_x_sources(&xai_key, &grok_model).await {
            Ok(grok_sources) => {
                log::info!("run_research: Grok returned {} X sources", grok_sources.len());
                if grok_sources.is_empty() {
                    log::warn!("run_research: Grok X path returned zero items. Check the detailed 'FULL RAW GROK RESPONSE' log lines above for exactly what the model replied. This often happens because the chat completions call has no live X search capability.");
                }
                sources.extend(grok_sources);
            }
            Err(e) => {
                log::error!("run_research: Grok X discovery error: {}", e);
                return Err(format!("Grok X research failed: {}", e));
            }
        }
    }

    if sources.is_empty() {
        let msg = match mode.as_str() {
            "rss" => "No recent RSS articles found from the configured feeds (Teslarati, Tesla Motors Club) — all items older than 14 days or feeds were unreachable.".to_string(),
            "x" => "Grok did not return any high-signal X posts matching the Musk-companies criteria this time. (See backend logs for the raw Grok response.) Try again later or run RSS only.".to_string(),
            "both" => "No sources returned: RSS feeds yielded nothing recent and Grok X discovery also returned zero items.".to_string(),
            _ => format!("No sources were found for research mode '{}'.", mode),
        };
        log::warn!("run_research: {}", msg);
        return Err(msg);
    }

    // Create run
    let run_id = Uuid::new_v4().to_string();
    let run_at = chrono::Utc::now().to_rfc3339();
    let run_source = match mode.as_str() {
        "rss" => "rss",
        "x" => "x_grok",
        _ => "both",
    };

    sqlx::query(
        "INSERT INTO research_runs (id, run_at, source) VALUES (?, ?, ?)"
    )
    .bind(&run_id)
    .bind(&run_at)
    .bind(run_source)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to create research run: {}", e))?;

    // Insert sources (with fresh row IDs + original_id)
    for source in &sources {
        let row_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO research_sources (
                id, run_id, title, content, url, published_at, 
                source_name, source_type, retweet_count, like_count, 
                reply_count, quote_count, original_id, media_url, used_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            "#
        )
        .bind(&row_id)
        .bind(&run_id)
        .bind(&source.title)
        .bind(&source.content)
        .bind(&source.url)
        .bind(source.published_at.clone())
        .bind(&source.source_name)
        .bind(&source.source_type)
        .bind(source.retweet_count)
        .bind(source.like_count)
        .bind(source.reply_count)
        .bind(source.quote_count)
        .bind(&source.original_id)
        .bind(&source.media_url)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to save research source: {}", e))?;
    }

    let run = ResearchRun {
        id: run_id.clone(),
        run_at,
        source: run_source.to_string(),
    };

    let saved_sources = fetch_sources_for_run(&state.db, &run_id).await?;
    Ok(ResearchRunWithSources {
        run,
        sources: saved_sources,
    })
}

#[tauri::command]
pub async fn get_latest_research_run(state: State<'_, AppState>) -> Result<Option<ResearchRunWithSources>, String> {
    fetch_latest_run_with_sources(&state.db).await
}

#[tauri::command]
pub async fn get_research_runs(state: State<'_, AppState>) -> Result<Vec<ResearchRun>, String> {
    let runs: Vec<ResearchRun> = sqlx::query_as(
        "SELECT * FROM research_runs ORDER BY run_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to fetch research runs: {}", e))?;

    Ok(runs)
}

#[tauri::command]
pub async fn get_research_run(state: State<'_, AppState>, run_id: String) -> Result<Option<ResearchRunWithSources>, String> {
    match fetch_run_with_sources(&state.db, &run_id).await {
        Ok(run_with_sources) => Ok(Some(run_with_sources)),
        Err(message) if message.contains("not found") => Ok(None),
        Err(message) => Err(message),
    }
}

#[tauri::command]
pub async fn get_all_historical_sources(state: State<'_, AppState>) -> Result<Vec<HistoricalResearchSource>, String> {
    let sources: Vec<HistoricalResearchSource> = sqlx::query_as(
        r#"
        SELECT 
            rs.*,
            rr.run_at
        FROM research_sources rs
        JOIN research_runs rr ON rs.run_id = rr.id
        ORDER BY COALESCE(rs.published_at, rr.run_at) DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to fetch historical sources: {}", e))?;

    Ok(sources)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ResetResearchResult {
    pub deleted_sources: u64,
    pub deleted_runs: u64,
}

#[tauri::command]
pub async fn reset_research_data(state: State<'_, AppState>) -> Result<ResetResearchResult, String> {
    reset_research_data_db(&state.db).await
}

pub async fn reset_research_data_db(db: &DbPool) -> Result<ResetResearchResult, String> {
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Failed to start reset transaction: {}", e))?;

    let sources_result = sqlx::query("DELETE FROM research_sources")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete research sources: {}", e))?;

    let runs_result = sqlx::query("DELETE FROM research_runs")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete research runs: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit reset transaction: {}", e))?;

    let remaining_sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_sources")
        .fetch_one(db)
        .await
        .map_err(|e| format!("Failed to verify research sources deletion: {}", e))?;

    let remaining_runs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM research_runs")
        .fetch_one(db)
        .await
        .map_err(|e| format!("Failed to verify research runs deletion: {}", e))?;

    if remaining_sources > 0 || remaining_runs > 0 {
        return Err(format!(
            "Reset incomplete: {} source(s) and {} run(s) still in database",
            remaining_sources, remaining_runs
        ));
    }

    let deleted_sources = sources_result.rows_affected();
    let deleted_runs = runs_result.rows_affected();

    log::info!(
        "reset_research_data: deleted {} sources and {} runs (verified empty)",
        deleted_sources,
        deleted_runs
    );

    Ok(ResetResearchResult {
        deleted_sources,
        deleted_runs,
    })
}

// ============================================
// Draft generation (T-005 / T-006 / T-015)
// ============================================

pub async fn mark_research_sources_used_db(
    db: &DbPool,
    source_ids: &[String],
) -> Result<(), String> {
    if source_ids.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    for source_id in source_ids {
        sqlx::query(
            "UPDATE research_sources SET used_at = ? WHERE id = ? AND used_at IS NULL",
        )
        .bind(&now)
        .bind(source_id)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to mark research source as used: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn generate_drafts_from_latest_research(
    state: State<'_, AppState>,
    count: Option<u32>,
    style: Option<String>,
) -> Result<Vec<Draft>, String> {
    let requested_count = count.unwrap_or(DEFAULT_DRAFT_COUNT).clamp(1, MAX_DRAFT_COUNT);

    let latest_run = fetch_latest_run_with_sources(&state.db)
        .await?
        .ok_or("No research run found. Run research first, then generate drafts.".to_string())?;

    let unused_sources = research::unused_research_sources(&latest_run.sources);
    if unused_sources.is_empty() {
        return Err(
            "All research sources have already been used. Run new research to find fresh stories."
                .to_string(),
        );
    }

    let available = unused_sources.len() as u32;
    let count = requested_count.min(available);

    let xai_key = require_xai_api_key(&state.db).await?;
    let (_, grok_model) = load_grok_settings(&state.db).await?;

    let draft_style = style
        .as_deref()
        .map(DraftStyle::parse)
        .unwrap_or_default();

    log::info!(
        "generate_drafts_from_latest_research: {} unused sources, count={}, style={}",
        unused_sources.len(),
        count,
        draft_style.as_str()
    );

    generation::generate_drafts_from_sources_db(
        &state.db,
        Some(&state.app_data_dir),
        &unused_sources,
        &xai_key,
        &grok_model,
        count,
        draft_style,
    )
    .await
}

async fn fetch_research_source_by_id(
    db: &DbPool,
    source_id: &str,
) -> Result<research::ResearchSource, String> {
    sqlx::query_as("SELECT * FROM research_sources WHERE id = ?")
        .bind(source_id)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Failed to fetch research source: {}", e))?
        .ok_or_else(|| format!("Research source '{}' not found.", source_id))
}

#[tauri::command]
pub async fn generate_draft_from_source(
    state: State<'_, AppState>,
    source_id: String,
    count: Option<u32>,
    style: Option<String>,
) -> Result<Vec<Draft>, String> {
    let count = count.unwrap_or(1).clamp(1, MAX_DRAFT_COUNT);
    let source = fetch_research_source_by_id(&state.db, &source_id).await?;
    if research::is_research_source_used(&source) {
        return Err(
            "This research source has already been used for draft generation.".to_string(),
        );
    }

    let xai_key = require_xai_api_key(&state.db).await?;
    let (_, grok_model) = load_grok_settings(&state.db).await?;

    let draft_style = style
        .as_deref()
        .map(DraftStyle::parse)
        .unwrap_or_default();

    log::info!(
        "generate_draft_from_source: source_id={}, count={}, style={}",
        source_id,
        count,
        draft_style.as_str()
    );

    generation::generate_drafts_from_sources_db(
        &state.db,
        Some(&state.app_data_dir),
        std::slice::from_ref(&source),
        &xai_key,
        &grok_model,
        count,
        draft_style,
    )
    .await
}

#[tauri::command]
pub async fn generate_draft_from_input(
    state: State<'_, AppState>,
    input: String,
    style: Option<String>,
) -> Result<Vec<Draft>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a link or topic.".to_string());
    }

    let xai_key = require_xai_api_key(&state.db).await?;
    let (_, grok_model) = load_grok_settings(&state.db).await?;
    let creds = load_x_credentials_db(&state.db).await.ok();

    let draft_style = style
        .as_deref()
        .map(DraftStyle::parse)
        .unwrap_or_default();

    log::info!(
        "generate_draft_from_input: input_len={}, style={}",
        trimmed.len(),
        draft_style.as_str()
    );

    let source = custom_source::resolve_custom_input(trimmed, creds.as_ref()).await?;

    generation::generate_drafts_from_sources_db(
        &state.db,
        Some(&state.app_data_dir),
        std::slice::from_ref(&source),
        &xai_key,
        &grok_model,
        1,
        draft_style,
    )
    .await
}

// ============================================
// X posting (T-007)
// ============================================

pub async fn load_x_credentials_db(db: &DbPool) -> Result<x_post::XCredentials, String> {
    Ok(x_post::XCredentials {
        api_key: require_setting(db, settings::X_CONSUMER_KEY, "X API key (consumer key)").await?,
        api_secret: require_setting(
            db,
            settings::X_CONSUMER_SECRET,
            "X API secret (consumer secret)",
        )
        .await?,
        access_token: require_setting(db, settings::X_ACCESS_TOKEN, "X access token").await?,
        access_token_secret: require_setting(
            db,
            settings::X_ACCESS_TOKEN_SECRET,
            "X access token secret",
        )
        .await?,
    })
}

#[tauri::command]
pub async fn has_x_credentials(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(load_x_credentials_db(&state.db).await.is_ok())
}

#[tauri::command]
pub async fn test_x_credentials(state: State<'_, AppState>) -> Result<String, String> {
    let creds = load_x_credentials_db(&state.db).await?;
    x_post::verify_credentials(&creds).await
}

#[tauri::command]
pub async fn resolve_draft_image(
    state: State<'_, AppState>,
    id: String,
) -> Result<Draft, String> {
    resolve_draft_image_db(&state.db, &state.app_data_dir, id).await
}

pub async fn resolve_draft_image_db(
    db: &DbPool,
    app_data_dir: &Path,
    id: String,
) -> Result<Draft, String> {
    let draft = get_draft_db(db, id.clone())
        .await?
        .ok_or("Draft not found".to_string())?;

    let context = build_draft_source_context(&draft);
    let legacy_multi_source = context.sources.len() > 1;
    let should_resolve = legacy_multi_source || !draft_has_stored_image(&draft);

    if !should_resolve {
        return Ok(draft);
    }

    let creds = load_x_credentials_db(db).await.ok();
    maybe_resolve_preview_image(db, app_data_dir, &id, &draft, creds.as_ref(), true).await?;

    get_draft_db(db, id)
        .await?
        .ok_or("Draft not found".to_string())
}

#[tauri::command]
pub async fn post_draft_to_x(state: State<'_, AppState>, id: String) -> Result<Draft, String> {
    let draft = get_draft_db(&state.db, id.clone())
        .await?
        .ok_or("Draft not found".to_string())?;

    if draft.status != draft_status::PENDING {
        return Err("Only pending drafts can be posted.".to_string());
    }

    if draft.text.trim().is_empty() {
        return Err("Draft text is empty.".to_string());
    }

    let creds = load_x_credentials_db(&state.db).await?;
    let context = build_draft_source_context(&draft);
    let should_resolve = !draft_has_stored_image(&draft) || context.sources.len() > 1;

    maybe_resolve_preview_image(
        &state.db,
        &state.app_data_dir,
        &id,
        &draft,
        Some(&creds),
        should_resolve,
    )
    .await?;

    let draft = get_draft_db(&state.db, id.clone())
        .await?
        .ok_or("Draft not found".to_string())?;

    let context = build_draft_source_context(&draft);
    let primary = context
        .primary_index
        .and_then(|index| context.sources.get(index));

    let media_id = x_media::resolve_post_media(
        &creds,
        draft.image_url.as_deref(),
        primary,
    )
    .await?;

    let media_ids: Vec<String> = media_id.into_iter().collect();
    let tweet_id = x_post::post_tweet(&creds, &context.normalized_text, &media_ids).await?;

    mark_draft_posted_db(&state.db, id, tweet_id.clone()).await?;

    get_draft_db(&state.db, draft.id)
        .await?
        .ok_or("Draft disappeared after posting".to_string())
}

// ============================================
// Settings / API Keys (persisted in DB for now)
// ============================================

/// Ensure the settings table exists (called lazily)
async fn ensure_settings_table(db: &DbPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(db)
    .await
    .ok(); // ignore error if it already exists
}

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    get_setting_db(&state.db, key).await
}

pub async fn get_setting_db(db: &DbPool, key: String) -> Result<Option<String>, String> {
    ensure_settings_table(db).await;

    let result: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(&key)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Failed to read setting: {}", e))?;

    Ok(result.map(|(v,)| v))
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    set_setting_db(&state.db, key, value).await
}

pub async fn set_setting_db(db: &DbPool, key: String, value: String) -> Result<(), String> {
    ensure_settings_table(db).await;

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES (?, ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(&key)
    .bind(&value)
    .execute(db)
    .await
    .map_err(|e| format!("Failed to save setting: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    delete_setting_db(&state.db, key).await
}

pub async fn delete_setting_db(db: &DbPool, key: String) -> Result<(), String> {
    ensure_settings_table(db).await;

    sqlx::query("DELETE FROM settings WHERE key = ?")
        .bind(&key)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to delete setting: {}", e))?;

    Ok(())
}

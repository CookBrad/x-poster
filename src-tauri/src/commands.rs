use crate::AppState;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Draft {
    pub id: String,
    pub text: String,
    pub sources_json: String,
    pub image_url: Option<String>,
    pub status: String, // "pending" | "posted" | "skipped"
    pub created_at: String,
    pub updated_at: String,
    pub posted_at: Option<String>,
    pub x_post_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftInput {
    pub text: String,
    pub sources_json: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDraftInput {
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub status: Option<String>,
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
pub async fn create_draft_db(db: &SqlitePool, input: CreateDraftInput) -> Result<Draft, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let draft = Draft {
        id: id.clone(),
        text: input.text,
        sources_json: input.sources_json,
        image_url: input.image_url,
        status: "pending".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        posted_at: None,
        x_post_id: None,
    };

    sqlx::query(
        r#"
        INSERT INTO drafts (id, text, sources_json, image_url, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&draft.id)
    .bind(&draft.text)
    .bind(&draft.sources_json)
    .bind(&draft.image_url)
    .bind(&draft.status)
    .bind(&draft.created_at)
    .bind(&draft.updated_at)
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
    db: &SqlitePool,
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

pub async fn get_draft_db(db: &SqlitePool, id: String) -> Result<Option<Draft>, String> {
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
    db: &SqlitePool,
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

pub async fn delete_draft_db(db: &SqlitePool, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM drafts WHERE id = ?")
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| format!("Failed to delete draft: {}", e))?;

    Ok(())
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
    db: &SqlitePool,
    id: String,
    x_post_id: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE drafts 
        SET status = 'posted', 
            x_post_id = ?, 
            posted_at = ?, 
            updated_at = ?
        WHERE id = ?
        "#
    )
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

    async fn create_test_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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
        };

        // Use the real reusable function
        let created = create_draft_db(&db, input).await.expect("create failed");

        assert_eq!(created.status, "pending");
        assert!(!created.id.is_empty());

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
        }).await.unwrap();

        let update = UpdateDraftInput {
            text: Some("Updated with fresh analysis".to_string()),
            image_url: None,
            status: Some("pending".to_string()),
        };

        update_draft_db(&db, created.id.clone(), update).await.expect("update failed");

        let fetched = get_draft_db(&db, created.id).await.unwrap().unwrap();
        assert_eq!(fetched.text, "Updated with fresh analysis");
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
}

// ============================================
// Settings / API Keys (persisted in DB for now)
// ============================================

/// Ensure the settings table exists (called lazily)
async fn ensure_settings_table(db: &SqlitePool) {
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

pub async fn get_setting_db(db: &SqlitePool, key: String) -> Result<Option<String>, String> {
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

pub async fn set_setting_db(db: &SqlitePool, key: String, value: String) -> Result<(), String> {
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

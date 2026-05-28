use crate::AppState;
use serde::{Deserialize, Serialize};
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

/// Create a new draft in the queue
#[tauri::command]
pub async fn create_draft(
    state: State<'_, AppState>,
    input: CreateDraftInput,
) -> Result<Draft, String> {
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
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to create draft: {}", e))?;

    Ok(draft)
}

/// Get all drafts, optionally filtered by status
#[tauri::command]
pub async fn get_drafts(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Draft>, String> {
    let drafts = if let Some(s) = status {
        sqlx::query_as::<_, Draft>(
            "SELECT * FROM drafts WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(s)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Draft>("SELECT * FROM drafts ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
    };

    drafts.map_err(|e| format!("Failed to fetch drafts: {}", e))
}

/// Get a single draft by ID
#[tauri::command]
pub async fn get_draft(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Draft>, String> {
    sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Failed to fetch draft: {}", e))
}

/// Update an existing draft
#[tauri::command]
pub async fn update_draft(
    state: State<'_, AppState>,
    id: String,
    input: UpdateDraftInput,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    // Build the query dynamically for the MVP (keeps it simple)
    let mut query = "UPDATE drafts SET updated_at = ?".to_string();
    let mut params: Vec<String> = vec![now.clone()];

    if let Some(text) = input.text {
        query.push_str(", text = ?");
        params.push(text);
    }
    if let Some(image_url) = input.image_url {
        query.push_str(", image_url = ?");
        params.push(image_url);
    }
    if let Some(status) = input.status {
        query.push_str(", status = ?");
        params.push(status);
    }

    query.push_str(" WHERE id = ?");
    params.push(id);

    let mut q = sqlx::query(&query);
    for p in params {
        q = q.bind(p);
    }

    q.execute(&state.db)
        .await
        .map_err(|e| format!("Failed to update draft: {}", e))?;

    Ok(())
}

/// Delete a draft
#[tauri::command]
pub async fn delete_draft(state: State<'_, AppState>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM drafts WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to delete draft: {}", e))?;

    Ok(())
}

/// Mark a draft as successfully posted
#[tauri::command]
pub async fn mark_draft_posted(
    state: State<'_, AppState>,
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
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to mark draft as posted: {}", e))?;

    Ok(())
}

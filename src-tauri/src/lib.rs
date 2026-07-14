mod commands;
mod constants;
mod custom_source;
mod draft_image;
mod generation;
mod research;
mod x_media;
mod x_post;

use sqlx::{AnyPool, any::AnyPoolOptions};
use std::path::PathBuf;
use tauri::Manager;

/// Unified pool type supporting both local SQLite (default) and remote Postgres
/// (or other sqlx drivers) via DATABASE_URL env var. This is the first step
/// toward running the DB on a server while keeping the desktop app working.
pub type DbPool = AnyPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub app_data_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Always prepare local app data dir (used for persisted draft images + asset protocol,
      // even when the main data DB is remote Postgres on a server).
      let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
      std::fs::create_dir_all(&app_data_dir).expect("failed to create app data directory");

      // Database URL: prefer DATABASE_URL env (e.g. postgres://... for a server-hosted DB,
      // or sqlite:/path for explicit file). Falls back to the classic local SQLite file
      // in app data dir. This enables "db on a server" as the first incremental step.
      let db_url: String = match std::env::var("DATABASE_URL") {
          Ok(u) if !u.trim().is_empty() => u,
          _ => {
              let db_path: PathBuf = app_data_dir.join("x-poster.db");
              format!("sqlite:{}?mode=rwc", db_path.display())
          }
      };

      tauri::async_runtime::block_on(async {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
          .max_connections(5)
          .connect(&db_url)
          .await
          .expect("failed to connect to database");

        // Run embedded migrations (works for both sqlite and postgres via sqlx any)
        sqlx::migrate!("./migrations")
          .run(&pool)
          .await
          .expect("failed to run database migrations");

        app.manage(AppState {
          db: pool,
          app_data_dir: app_data_dir.clone(),
        });
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        commands::create_draft,
        commands::get_drafts,
        commands::get_draft,
        commands::update_draft,
        commands::delete_draft,
        commands::clear_pending_drafts,
        commands::mark_draft_posted,
        commands::get_setting,
        commands::set_setting,
        commands::delete_setting,
        commands::fetch_research_sources,
        commands::run_research,
        commands::get_latest_research_run,
        commands::get_research_runs,
        commands::get_research_run,
        commands::get_all_historical_sources,
        commands::reset_research_data,
        commands::generate_drafts_from_latest_research,
        commands::generate_draft_from_source,
        commands::generate_draft_from_input,
        commands::generate_reply_from_input,
        commands::generate_reply_from_source,
        commands::post_draft_to_x,
        commands::resolve_draft_image,
        commands::test_x_credentials,
        commands::has_x_credentials,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

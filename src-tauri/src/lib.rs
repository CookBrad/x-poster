mod commands;
mod generation;
mod research;
mod x_oauth;
mod x_post;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Initialize SQLite database
      let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
      std::fs::create_dir_all(&app_data_dir).expect("failed to create app data directory");

      let db_path: PathBuf = app_data_dir.join("x-poster.db");
      let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

      tauri::async_runtime::block_on(async {
        let pool = SqlitePoolOptions::new()
          .max_connections(5)
          .connect(&db_url)
          .await
          .expect("failed to connect to database");

        // Run embedded migrations
        sqlx::migrate!("./migrations")
          .run(&pool)
          .await
          .expect("failed to run database migrations");

        app.manage(AppState { db: pool });
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        commands::create_draft,
        commands::get_drafts,
        commands::get_draft,
        commands::update_draft,
        commands::delete_draft,
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
        commands::post_draft_to_x,
        commands::test_x_credentials,
        commands::has_x_credentials,
        commands::connect_x_oauth,
        commands::disconnect_x_oauth,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

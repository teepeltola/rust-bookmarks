use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;
use crate::models::{Bookmark, BookmarkInput, Health};

pub fn router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/bookmarks", get(list_bookmarks).post(create_bookmark))
        .route(
            "/bookmarks/{id}",
            get(get_bookmark)
                .put(update_bookmark)
                .delete(delete_bookmark),
        )
        .with_state(pool)
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn list_bookmarks(State(pool): State<SqlitePool>) -> Result<Json<Vec<Bookmark>>, AppError> {
    Ok(Json(db::list(&pool).await?))
}

async fn get_bookmark(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Bookmark>, AppError> {
    db::get(&pool, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn create_bookmark(
    State(pool): State<SqlitePool>,
    Json(input): Json<BookmarkInput>,
) -> Result<(StatusCode, Json<Bookmark>), AppError> {
    let url = non_empty(&input.url)?;
    let title = non_empty(&input.title)?;
    let bookmark = db::create(&pool, url, title).await?;
    Ok((StatusCode::CREATED, Json(bookmark)))
}

async fn update_bookmark(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<BookmarkInput>,
) -> Result<Json<Bookmark>, AppError> {
    let url = non_empty(&input.url)?;
    let title = non_empty(&input.title)?;
    db::update(&pool, id, url, title)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn delete_bookmark(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    if db::delete(&pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// Reject missing or blank text. The returned slice borrows `value`.
fn non_empty(value: &str) -> Result<&str, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::BadRequest(
            "url and title must not be empty".into(),
        ))
    } else {
        Ok(trimmed)
    }
}

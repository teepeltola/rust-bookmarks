use sqlx::SqlitePool;

use crate::models::Bookmark;

pub async fn list(pool: &SqlitePool) -> Result<Vec<Bookmark>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, url, title, created_at
         FROM bookmarks
         ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Bookmark>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, url, title, created_at
         FROM bookmarks
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &SqlitePool, url: &str, title: &str) -> Result<Bookmark, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO bookmarks (url, title)
         VALUES (?, ?)
         RETURNING id, url, title, created_at",
    )
    .bind(url)
    .bind(title)
    .fetch_one(pool)
    .await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    url: &str,
    title: &str,
) -> Result<Option<Bookmark>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE bookmarks
         SET url = ?, title = ?
         WHERE id = ?
         RETURNING id, url, title, created_at",
    )
    .bind(url)
    .bind(title)
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM bookmarks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

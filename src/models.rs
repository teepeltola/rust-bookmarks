use serde::{Deserialize, Serialize};

/// A row from the `bookmarks` table.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub created_at: String,
}

/// JSON body for creating or replacing a bookmark.
#[derive(Debug, Deserialize)]
pub struct BookmarkInput {
    pub url: String,
    pub title: String,
}

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

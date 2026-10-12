mod db;
mod error;
mod models;
mod routes;

#[cfg(test)]
mod tests;

use std::error::Error;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // One pool is shared with every request through Axum's State.
    let options = SqliteConnectOptions::new()
        .filename("bookmarks.db")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = routes::router(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

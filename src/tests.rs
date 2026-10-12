use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

use crate::routes;

/// One connection so every request sees the same in-memory database.
/// A second connection would open an empty database of its own.
async fn app() -> Router {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    routes::router(pool)
}

fn request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(serde_json::to_vec(&body).expect("request json"))
    } else {
        Body::empty()
    };
    builder.body(body).expect("request")
}

async fn send(app: &Router, request: Request<Body>) -> Response<Body> {
    app.clone().oneshot(request).await.expect("response")
}

async fn send_json(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = send(app, request).await;
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).expect("response json");
    (status, json)
}

fn bookmark(url: &str, title: &str) -> Value {
    json!({ "url": url, "title": title })
}

#[tokio::test]
async fn health_returns_ok() {
    let app = app().await;

    let (status, body) = send_json(&app, request("GET", "/health", None)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn list_is_empty_at_first() {
    let app = app().await;

    let (status, body) = send_json(&app, request("GET", "/bookmarks", None)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test]
async fn create_returns_the_bookmark() {
    let app = app().await;

    let (status, body) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://doc.rust-lang.org/book/", "The Rust Book")),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], 1);
    assert_eq!(body["url"], "https://doc.rust-lang.org/book/");
    assert_eq!(body["title"], "The Rust Book");
    assert_eq!(body["created_at"].as_str().unwrap().len(), 19);
}

#[tokio::test]
async fn create_trims_url_and_title() {
    let app = app().await;

    let (_, body) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("  https://example.com  ", "  Example  ")),
        ),
    )
    .await;

    assert_eq!(body["url"], "https://example.com");
    assert_eq!(body["title"], "Example");
}

#[tokio::test]
async fn create_rejects_blank_url() {
    let app = app().await;

    let (status, body) = send_json(
        &app,
        request("POST", "/bookmarks", Some(bookmark("   ", "Example"))),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({ "error": "url and title must not be empty" }));
}

#[tokio::test]
async fn create_rejects_blank_title() {
    let app = app().await;

    let (status, body) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://example.com", " ")),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({ "error": "url and title must not be empty" }));
}

#[tokio::test]
async fn create_rejects_a_missing_title() {
    let app = app().await;

    let response = send(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(json!({ "url": "https://example.com" })),
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn list_returns_bookmarks_oldest_id_first() {
    let app = app().await;
    send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://a.example", "A")),
        ),
    )
    .await;
    send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://b.example", "B")),
        ),
    )
    .await;

    let (status, body) = send_json(&app, request("GET", "/bookmarks", None)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(body[0]["title"], "A");
    assert_eq!(body[1]["title"], "B");
    assert!(body[0]["id"].as_i64().unwrap() < body[1]["id"].as_i64().unwrap());
}

#[tokio::test]
async fn get_returns_one_bookmark() {
    let app = app().await;
    let (_, created) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://example.com", "Example")),
        ),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, body) = send_json(&app, request("GET", &format!("/bookmarks/{id}"), None)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, created);
}

#[tokio::test]
async fn get_unknown_id_is_not_found() {
    let app = app().await;

    let (status, body) = send_json(&app, request("GET", "/bookmarks/9", None)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "bookmark not found" }));
}

#[tokio::test]
async fn get_rejects_a_non_numeric_id() {
    let app = app().await;

    let response = send(&app, request("GET", "/bookmarks/abc", None)).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_replaces_url_and_title() {
    let app = app().await;
    let (_, created) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://old.example", "Old")),
        ),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, body) = send_json(
        &app,
        request(
            "PUT",
            &format!("/bookmarks/{id}"),
            Some(bookmark("https://new.example", "New")),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], created["id"]);
    assert_eq!(body["url"], "https://new.example");
    assert_eq!(body["title"], "New");
    assert_eq!(body["created_at"], created["created_at"]);
}

#[tokio::test]
async fn update_unknown_id_is_not_found() {
    let app = app().await;

    let (status, body) = send_json(
        &app,
        request(
            "PUT",
            "/bookmarks/9",
            Some(bookmark("https://example.com", "Example")),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "bookmark not found" }));
}

#[tokio::test]
async fn update_rejects_blank_title() {
    let app = app().await;
    let (_, created) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://example.com", "Example")),
        ),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, _) = send_json(
        &app,
        request(
            "PUT",
            &format!("/bookmarks/{id}"),
            Some(bookmark("https://example.com", "  ")),
        ),
    )
    .await;
    let (_, stored) = send_json(&app, request("GET", &format!("/bookmarks/{id}"), None)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(stored["title"], "Example");
}

#[tokio::test]
async fn delete_removes_the_bookmark() {
    let app = app().await;
    let (_, created) = send_json(
        &app,
        request(
            "POST",
            "/bookmarks",
            Some(bookmark("https://example.com", "Example")),
        ),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let response = send(&app, request("DELETE", &format!("/bookmarks/{id}"), None)).await;
    let delete_status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let (status, body) = send_json(&app, request("GET", &format!("/bookmarks/{id}"), None)).await;

    assert_eq!(delete_status, StatusCode::NO_CONTENT);
    assert!(bytes.is_empty());
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "bookmark not found" }));
}

#[tokio::test]
async fn delete_unknown_id_is_not_found() {
    let app = app().await;

    let (status, body) = send_json(&app, request("DELETE", "/bookmarks/9", None)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "bookmark not found" }));
}

#[tokio::test]
async fn database_failure_hides_the_sql_error() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    pool.close().await;
    let app = routes::router(pool);

    let (status, body) = send_json(&app, request("GET", "/bookmarks", None)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, json!({ "error": "database error" }));
}

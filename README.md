# rust-bookmarks

A small REST API for learning Rust. It stores bookmarks in a local SQLite file and speaks JSON.

## Run

You need [Rust](https://rustup.rs/). From this folder:

```bash
cargo run
```

The server listens on `http://127.0.0.1:3000`. The first start creates `bookmarks.db` and applies the migration in `migrations/`.

## Try it

```bash
curl -s localhost:3000/health

curl -s -X POST localhost:3000/bookmarks \
  -H 'content-type: application/json' \
  -d '{"url":"https://doc.rust-lang.org/book/","title":"The Rust Book"}'

curl -s localhost:3000/bookmarks
```

## API

| Method | Path | Success | Notes |
| --- | --- | --- | --- |
| `GET` | `/health` | 200 | `{ "status": "ok" }` |
| `GET` | `/bookmarks` | 200 | list, oldest id first |
| `GET` | `/bookmarks/:id` | 200 | one bookmark |
| `POST` | `/bookmarks` | 201 | body is `{ "url", "title" }` |
| `PUT` | `/bookmarks/:id` | 200 | replaces `url` and `title` |
| `DELETE` | `/bookmarks/:id` | 204 | empty body |

A bookmark looks like:

```json
{
  "id": 1,
  "url": "https://doc.rust-lang.org/book/",
  "title": "The Rust Book",
  "created_at": "2026-10-12 18:01:57"
}
```

`created_at` is UTC, written by SQLite as `datetime('now')`.

Errors are JSON: `{ "error": "..." }`.

- Blank `url` or `title` (including whitespace only) returns **400**.
- An unknown id returns **404**.
- A database failure returns **500**.

## Layout

| File | What it teaches |
| --- | --- |
| `src/main.rs` | Start Tokio, open one `SqlitePool`, run migrations, bind the server |
| `src/routes.rs` | Axum handlers. The pool is shared through `State`, not copied per request |
| `src/models.rs` | Serde structs: JSON in, JSON out |
| `src/db.rs` | SQL with `sqlx::query_as`, returned as `Result` |
| `src/error.rs` | `AppError` turned into an HTTP status |
| `migrations/001_create_bookmarks.sql` | The `bookmarks` table |

## Stack

Axum, Tokio, Serde, SQLx (SQLite), and thiserror.

# quickbase

A Rust clone of [PocketBase](https://pocketbase.io), built as a **personal learning exercise**.

> ⚠️ **This is not a production-ready project and is not intended to be maintained.** It exists purely as a vehicle for learning Rust through a structured, real-world project. Do not use it in production.

---

## What is this?

quickbase mirrors PocketBase's architecture and feature set, translated into idiomatic Rust. It uses PocketBase's actual repository as a reference for module organization, naming conventions, and behavior.

The goal is not to replace PocketBase — it's to learn Rust by building something non-trivial.

## Tech Stack

- **Language**: Rust
- **Database**: SQLite via [sqlx](https://github.com/launchbadge/sqlx) (bundled, statically linked)
- **HTTP**: [axum](https://github.com/tokio-rs/axum)
- **Async runtime**: [tokio](https://tokio.rs)
- **Scripting** (planned): [Rhai](https://rhai.rs) first, then [QuickJS](https://bellard.org/quickjs/) — instead of the Goja (Go) JavaScript engine PocketBase uses, quickbase will use Rhai as an embedded scripting engine for hooks and extensions, with a later migration to QuickJS for broader JavaScript compatibility. (Maybe Lua in the future)

## Features

- Embedded SQLite database (bundled, single binary)
- Collections API — create, read, update, delete collections with typed field schemas
- Records API — full CRUD on dynamic record tables
- Schema field types: `text`, `number`, `bool`, `date`, `json`, `email`, `url`
- Per-field validation (required fields, type checking)
- Schema diffing on collection update — adds/drops SQLite columns automatically
- System migrations on bootstrap
- Single self-contained executable (no external dependencies)

## Running

```bash
cargo run
```

The server starts on `0.0.0.0:8090` and creates a `./qb_data/` directory for the database.

```bash
# Create a collection
curl -X POST http://localhost:8090/api/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "articles",
    "schema": [
      {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}},
      {"id": "f00000002", "name": "views", "type": "number", "required": false, "options": {}}
    ]
  }'

# Create a record
curl -X POST http://localhost:8090/api/collections/articles/records \
  -H "Content-Type: application/json" \
  -d '{"title": "Hello World", "views": 42}'

# List records
curl http://localhost:8090/api/collections/articles/records
```

## Testing

All modules are tested as they are built. Tests are written in parallel with the implementation — unit tests live inline via `#[cfg(test)]` and integration tests live in the root-level `tests/` directory.

```bash
cargo test
```

## Project Structure

```
src/
├── main.rs
├── lib.rs
├── db/             # database layer: pools, migrations, error types, model trait
├── migrations/     # system migrations (bootstrap)
├── models/         # collection, record, schema field types
├── core/           # App struct — central orchestrator
├── apis/           # axum HTTP handlers
├── forms/          # validation layer (mirrors PocketBase's forms package)
└── cmd/            # serve command

tests/              # integration tests per module
```

---

## Roadmap

### Done
- [x] Project scaffolding and module structure
- [x] Database layer (`DbPools`, `BaseModel`, `MigrationsList`, `MigrationsRunner`)
- [x] System migrations (bootstrap creates `_collections`, `_params`, `_migrations`)
- [x] `Collection` model with typed schema fields
- [x] `Record` model with dynamic field map
- [x] `App` core struct with bootstrap and collection queries
- [x] Schema field types (`text`, `number`, `bool`, `date`, `json`, `email`, `url`)
- [x] Collections API — `GET`, `POST`, `PATCH`, `DELETE`
- [x] Records API — `GET`, `POST`, `PATCH`, `DELETE`
- [x] `CollectionUpsert` form — validates, creates table, diffs schema on update
- [x] `RecordUpsert` form — validates field types and required fields
- [x] `serve` command — bootstraps app and starts axum HTTP server
- [x] Integration tests for all modules

### In Progress / Next
- [ ] Proper random ID generation (replace naive timestamp-based IDs)
- [ ] `src/tools/` — search, filter, security utilities

### Planned
- [ ] Auto-generated user migrations (record schema changes to `pb_migrations/`)
- [ ] Rhai scripting engine for hooks and extensions (`pb_hooks/`)
- [ ] Auth collections (email/password authentication, JWT tokens)
- [ ] File upload and storage
- [ ] Realtime subscriptions (SSE)
- [ ] Web UI — browser-based interface to manage collections, paste hook functions, and inspect data
- [ ] QuickJS scripting engine (migration from Rhai for broader JS compatibility)
- [ ] CLI tool — Supabase-style CLI to push database schema changes and functions to a remote server
- [ ] MCP server — built-in [Model Context Protocol](https://modelcontextprotocol.io) server to control the application from AI assistants

---

## Acknowledgements

This project is heavily inspired by and references [PocketBase](https://github.com/pocketbase/pocketbase) by [Gani Georgiev](https://github.com/ganigeorgiev). All credit for the original architecture and design goes to the PocketBase project.
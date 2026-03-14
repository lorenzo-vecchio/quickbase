# quickbase

A Rust clone of [PocketBase](https://pocketbase.io), built as a **personal learning exercise**.

> ⚠️ **This is not a production-ready project and is not intended to be maintained.** It exists purely as a vehicle for learning Rust through a structured, real-world project. Do not use it in production.

---

## What is this?

quickbase mirrors PocketBase's architecture and feature set, translated into idiomatic Rust. It uses PocketBase's actual repository as a reference for module organization, naming conventions, and behavior — including serving the official PocketBase admin UI unchanged.

The goal is not to replace PocketBase — it's to learn Rust by building something non-trivial.

## Tech Stack

- **Language**: Rust
- **Database**: SQLite via [sqlx](https://github.com/launchbadge/sqlx) (bundled, statically linked)
- **HTTP**: [axum](https://github.com/tokio-rs/axum) 0.8
- **Async runtime**: [tokio](https://tokio.rs)
- **Auth**: [jsonwebtoken](https://github.com/Keats/jsonwebtoken) for JWT, [argon2](https://github.com/RustCrypto/password-hashes) for password hashing
- **Scripting** (planned): [Rhai](https://rhai.rs) first, then [QuickJS](https://bellard.org/quickjs/) — instead of the Goja (Go) JavaScript engine PocketBase uses, quickbase will use Rhai as an embedded scripting engine for hooks and extensions, with a later migration to QuickJS for broader JavaScript compatibility.

## Features

- Embedded SQLite database (bundled, single binary)
- Collections API — create, read, update, delete collections with typed field schemas
- Records API — full CRUD on dynamic record tables, with `filter`, `sort`, `page`, `perPage`, `skipTotal` query params
- Schema field types: `text`, `number`, `bool`, `date`, `json`, `email`, `url`, `autodate`, `password`, `select`, `relation`, `editor`, `file`
- Per-field validation (required fields, type checking)
- Schema diffing on collection update — adds/drops SQLite columns automatically
- Auth collections — email/password login, JWT token issuance and refresh
- `_superusers` system collection with first-run setup wizard
- All API endpoints accept both collection **name** and collection **ID** as path parameters (mirrors PocketBase's `:collectionIdOrName` behavior)
- Collection scaffold endpoint (`GET /api/collections/meta/scaffolds`) for the UI's "New collection" modal
- Logs API — stores and queries request logs
- Health endpoint (`GET /api/health`)
- Official PocketBase admin UI served at `/_/`
- System migrations on bootstrap
- Single self-contained executable (no external dependencies)

## Running

```bash
cargo run
```

The server starts on `0.0.0.0:8090` and creates a `./qb_data/` directory for the database. The PocketBase admin UI is available at [http://localhost:8090/_/](http://localhost:8090/_/).

On first run with no superusers, an installer link is printed to stdout — open it to create your first admin account.

```bash
# Authenticate
curl -X POST http://localhost:8090/api/collections/_superusers/auth-with-password \
  -H "Content-Type: application/json" \
  -d '{"identity": "you@example.com", "password": "yourpassword"}'
# → returns { "token": "...", "record": { ... } }

# Create a collection (use the token from above)
curl -X POST http://localhost:8090/api/collections \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "articles",
    "type": "base",
    "fields": [
      {"id": "f00000001", "name": "title", "type": "text", "required": true},
      {"id": "f00000002", "name": "views", "type": "number", "required": false}
    ]
  }'

# Create a record
curl -X POST http://localhost:8090/api/collections/articles/records \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"title": "Hello World", "views": 42}'

# List records (also accepts collection ID instead of name)
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
├── core/           # App struct — central orchestrator, collection queries
├── apis/           # axum HTTP handlers
│   ├── collections.rs   # CRUD + scaffold endpoint
│   ├── records.rs       # CRUD with filter/sort/pagination
│   ├── auth.rs          # auth-with-password, auth-refresh, auth-methods, …
│   ├── admin.rs         # admin settings, files token
│   ├── logs.rs          # request log storage and queries
│   └── ui.rs            # serves the official PocketBase admin UI
├── forms/          # validation layer (mirrors PocketBase's forms package)
│   ├── collection_upsert.rs
│   └── record_upsert.rs
├── tools/          # utilities
│   ├── security.rs      # JWT generation/verification, Argon2 hashing, random ID gen
│   └── …
└── cmd/            # serve command
    └── serve.rs         # bootstrap + first-run setup wizard

tests/              # integration tests per module
ui/                 # official PocketBase admin UI (pre-built, served as static files)
```

---

## Roadmap

### Done
- [x] Project scaffolding and module structure
- [x] Database layer (`DbPools`, `BaseModel`, `MigrationsList`, `MigrationsRunner`)
- [x] System migrations (bootstrap creates `_collections`, `_params`, `_migrations`, `_superusers`)
- [x] `Collection` model with typed schema fields
- [x] `Record` model with dynamic field map
- [x] `App` core struct with bootstrap, collection queries, and `find_collection_by_name_or_id`
- [x] Schema field types (`text`, `number`, `bool`, `date`, `json`, `email`, `url`, `autodate`, `password`, `select`, `relation`, `editor`, `file`)
- [x] Collections API — `GET`, `POST`, `PATCH`, `DELETE` with paginated list
- [x] Collection scaffold endpoint (`GET /api/collections/meta/scaffolds`)
- [x] Records API — `GET`, `POST`, `PATCH`, `DELETE` with `filter`, `sort`, `page`, `perPage`, `skipTotal`
- [x] `CollectionUpsert` form — validates, creates table, diffs schema on update
- [x] `RecordUpsert` form — validates field types and required fields
- [x] Auth collections — email/password authentication, JWT token issuance and refresh
- [x] `_superusers` system collection, first-run setup wizard
- [x] Proper random ID generation (alphanumeric, 15 chars, `rand` crate)
- [x] `src/tools/security` — JWT (jsonwebtoken), Argon2 password hashing, ID generation
- [x] Logs API (`GET /api/logs`, `/api/logs/stats`, `/api/logs/{id}`)
- [x] Health endpoint (`GET /api/health`)
- [x] Official PocketBase admin UI served at `/_/` — collections and records fully usable end-to-end
- [x] `serve` command — bootstraps app and starts axum HTTP server
- [x] Integration tests for all modules

### In Progress / Next
- [ ] Full filter expression parsing (currently: valid expressions are passed through as raw SQL; unsafe/invalid ones are silently ignored)
- [ ] Configurable JWT secret (currently hardcoded to `changeme_secret_key`)
- [ ] Request logging middleware (populate `_logs` from actual HTTP requests)

### Planned
- [ ] Auto-generated user migrations (record schema changes to `pb_migrations/`)
- [ ] File upload and storage (`POST /api/collections/{name}/records` with multipart)
- [ ] Realtime subscriptions (SSE — `GET /api/realtime`)
- [ ] OAuth2 / social login
- [ ] OTP / MFA (stub endpoints exist, not functional)
- [ ] Email sending — verification and password-reset flows (stub endpoints exist)
- [ ] Rhai scripting engine for hooks and extensions (`pb_hooks/`)
- [ ] QuickJS scripting engine (migration from Rhai for broader JS compatibility)
- [ ] CLI tool — Supabase-style CLI to push database schema changes and functions to a remote server
- [ ] MCP server — built-in [Model Context Protocol](https://modelcontextprotocol.io) server to control the application from AI assistants

---

## Acknowledgements

This project is heavily inspired by and references [PocketBase](https://github.com/pocketbase/pocketbase) by [Gani Georgiev](https://github.com/ganigeorgiev). All credit for the original architecture and design goes to the PocketBase project.
